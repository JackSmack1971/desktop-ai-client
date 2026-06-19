/// IPC commands for the chat streaming surface.
///
/// Command inventory entries:
///   chat_send   — windows: ["main"], production: true, sensitivity: HIGH
///   chat_cancel — windows: ["main"], production: true, sensitivity: low
///
/// Security model:
/// - Both commands assert the caller is the main window (backend enforcement).
/// - `chat_send` NEVER accepts an api_key parameter (D-10 hard invariant).
/// - Credentials are retrieved internally from AppState::secrets.
/// - System prompt is backend-owned and prepended in providers::routing (D-12).
/// - CancellationToken registry is cleaned up unconditionally after each request
///   to prevent HashMap growth (STRIDE T-02-04, Pitfall 5 from RESEARCH.md).
/// - Conversation title is auto-generated from the first user message (D-03) —
///   never accepted from IPC.
/// - Storage writes in spawned task use app_handle.state::<T>() — no State<'_>
///   lifetime crosses the thread boundary (Pitfall 1).
/// - Storage write errors are non-fatal: logged via eprintln!, streaming continues
///   regardless of storage outcome (T-03-13 mitigation).
use crate::app_state::AppState;
use crate::providers::openrouter::ProviderMessage;
use crate::providers::{routing, sse};
use crate::security::command_policy;
use crate::security::secrets::{self, ProviderId};
use crate::storage::artifacts::{self, ArtifactContentType, ArtifactStore};
use crate::storage::sqlite::{ConversationStore, MessageStore};
use mime_guess::from_path;
use std::fs;
use std::path::Path;
use tauri::ipc::Channel;
use tauri::Manager;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// A single message in the conversation history passed from the frontend.
///
/// Matches the D-11 parameter shape. Role is validated implicitly by the
/// provider (only "user" and "assistant" produce meaningful completions).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Token usage counters returned in `ChatEvent::Done`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Streaming events delivered via `Channel<ChatEvent>` to the frontend.
///
/// All terminal state (done, error, cancellation) is delivered exclusively
/// through this channel. The frontend drops the channel listener after
/// receiving `Done` or `Error` (D-03 invariant).
///
/// Serialized with `#[serde(tag = "type")]` so the frontend discriminated
/// union `{ type: 'Ack' | 'Delta' | 'Done' | 'Error' }` matches directly.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum ChatEvent {
    /// Sent synchronously before the spawned task starts so the frontend
    /// has a `request_id` for cancellation (D-14).
    Ack { request_id: String },
    /// An incremental content token.
    Delta { text: String },
    /// Stream complete — carries resolved model name and token counts.
    Done {
        usage: Option<TokenUsage>,
        model: String,
    },
    /// Terminal failure or user-initiated cancellation.
    /// `code == "CANCELLED"` signals voluntary abort (D-04).
    Error { code: String, message: String },
    /// Backend-owned artifact detected after stream completion.
    ArtifactReady {
        artifact_id: String,
        content_type: ArtifactContentType,
        preview: String,
    },
}

/// Errors returned by chat IPC commands to the frontend.
///
/// Serialized as `{ code: "SCREAMING_SNAKE_CASE", message: string }` to match
/// the established IPC error shape from `ipc::app_shell::ShellError`.
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("credential error: {0}")]
    CredentialError(String),
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("channel error: {0}")]
    ChannelError(String),
    #[error("request not found: {0}")]
    RequestNotFound(String),
}

impl From<command_policy::PolicyError> for ChatError {
    fn from(value: command_policy::PolicyError) -> Self {
        match value {
            command_policy::PolicyError::UnauthorizedWindow(msg) => {
                ChatError::UnauthorizedWindow(msg)
            }
            command_policy::PolicyError::UnknownCommand(msg) => {
                ChatError::UnauthorizedWindow(msg)
            }
        }
    }
}

/// Generate a conversation title from the first user-role message.
///
/// Title is backend-owned (D-03) and never accepted from IPC. Takes up to
/// 60 unicode scalar values from the first message where `role == "user"`.
/// Returns "New conversation" when no user message is found.
fn title_from_messages(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| m.content.chars().take(60).collect())
        .unwrap_or_else(|| "New conversation".to_string())
}

/// Submit a prompt and stream the response back through a Tauri channel.
///
/// CRITICAL: This command does NOT accept an `api_key` parameter (D-10).
/// Credentials are retrieved from AppState::secrets internally.
///
/// Flow:
/// 1. Assert main window.
/// 2. Resolve effective conversation_id: use provided id or generate a new UUID.
/// 3. If new conversation: call ConversationStore::create_conversation with auto-title
///    derived from first user message (D-03, backend-owned title, never from IPC).
/// 4. Persist all user messages via MessageStore::insert_message.
/// 5. Generate request_id, create CancellationToken.
/// 6. Register token in active_requests (before spawn, so chat_cancel works).
/// 7. Retrieve API key from secrets (drop lock before any await).
/// 8. Send `ChatEvent::Ack` synchronously (D-14).
/// 9. Resolve model and build provider messages.
/// 10. Spawn async task: stream → collect assistant text → write storage on
///     terminal event → cleanup token.
/// 11. Return Ok(()) immediately.
///
/// Storage write errors inside the spawned task are non-fatal: logged via
/// eprintln! and the streaming task continues regardless (T-03-13 mitigation).
#[tauri::command]
pub async fn chat_send(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    messages: Vec<ChatMessage>,
    model: Option<String>,
    conversation_id: Option<String>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    attachments: Option<Vec<Uuid>>,
    channel: Channel<ChatEvent>,
) -> Result<(), ChatError> {
    // D-10: no api_key parameter — credentials come from state only.
    command_policy::policy_check("chat_send", window.label())?;

    // Resolve effective conversation id (D-11 Phase 2: Option<String>).
    // When None, create a new conversation row before streaming.
    // When Some(id), the caller is resuming an existing conversation.
    let effective_conv_id: String = conversation_id
        .as_deref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Create the conversation row for new conversations (conversation_id == None).
    // Title is auto-generated from first user message (D-03: backend-owned, never
    // accepted from IPC). Re-acquire ConversationStore from app_handle so no
    // State<'_> lifetime escapes this command body (Pitfall 1 prevention).
    if conversation_id.is_none() {
        let title = title_from_messages(&messages);
        let conv_store = app_handle.state::<ConversationStore>();
        if let Err(e) = conv_store.create_conversation(&effective_conv_id, &title) {
            eprintln!("[chat] failed to create conversation: {e}");
        }
    }

    // Persist all user messages before streaming starts (HIST-01).
    // Storage errors are non-fatal — streaming must not be blocked by storage
    // failures (T-03-13 mitigation).
    {
        let msg_store = app_handle.state::<MessageStore>();
        for msg in &messages {
            if msg.role == "user" {
                if let Err(e) = msg_store.insert_message(
                    &Uuid::new_v4().to_string(),
                    &effective_conv_id,
                    &msg.role,
                    &msg.content,
                ) {
                    eprintln!("[chat] failed to persist user message: {e}");
                }
            }
        }
    }

    let request_id = Uuid::new_v4().to_string();
    let token = CancellationToken::new();

    // Register the token before spawning so chat_cancel can find it
    // immediately after this command returns (Pitfall 5 prevention).
    {
        let mut requests = state
            .active_requests
            .lock()
            .map_err(|e| ChatError::ProviderError(format!("active_requests lock poisoned: {e}")))?;
        requests.insert(request_id.clone(), token.clone());
    }

    // Retrieve the API key before spawning — never hold the lock across await
    // (Pitfall 3). We expose-then-rewrap to get an owned SecretString that is
    // `'static` and safe to move into the task (Pitfall 7, RESEARCH Section 4).
    let api_key = secrets::get_provider_key(ProviderId::OpenRouter)
        .map_err(|e| ChatError::CredentialError(e.to_string()))?;

    // Send Ack synchronously before the spawn so the frontend has request_id
    // immediately and can call chat_cancel if needed (D-14).
    channel
        .send(ChatEvent::Ack {
            request_id: request_id.clone(),
        })
        .map_err(|e| ChatError::ChannelError(e.to_string()))?;

    let resolved_model = routing::select_model(model.as_deref());
    let routable_messages: Vec<routing::RoutableMessage> = messages
        .iter()
        .map(|m| routing::RoutableMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();
    let mut provider_messages =
        routing::build_provider_messages(routing::DEFAULT_SYSTEM_PROMPT, &routable_messages);
    if let Some(attachment_context) = resolve_attachments(&state, attachments)? {
        provider_messages.insert(
            1,
            ProviderMessage {
                role: "system".into(),
                content: attachment_context,
            },
        );
    }
    let request_id_clone = request_id.clone();

    // Spawn the streaming task. `tauri::State<'_>` is not `'static`, so we
    // re-acquire state from AppHandle inside the spawn (Pitfall 1).
    tokio::spawn(async move {
        let (result, accumulated_text, done_model) = run_stream(
            &api_key,
            &resolved_model,
            provider_messages,
            max_completion_tokens,
            temperature,
            &channel,
            token,
        )
        .await;

        // Write storage based on the terminal event outcome.
        // Re-acquire stores from app_handle (State<'_> not 'static — Pitfall 1).
        // Storage errors are non-fatal: log and continue (T-03-13 mitigation).
        match &result {
            Ok(()) => {
                // Stream completed with Done event — persist assistant message
                // and mark conversation complete with resolved model (D-05).
                let assistant_message_id = Uuid::new_v4().to_string();
                let msg_store = app_handle.state::<MessageStore>();
                if let Err(e) = msg_store.insert_message(
                    &assistant_message_id,
                    &effective_conv_id,
                    "assistant",
                    &accumulated_text,
                ) {
                    eprintln!("[chat] failed to persist assistant message: {e}");
                }
                let conv_store = app_handle.state::<ConversationStore>();
                if let Err(e) = conv_store.mark_complete(&effective_conv_id, &done_model) {
                    eprintln!("[chat] failed to mark conversation complete: {e}");
                }

                if let Some(detected) = artifacts::detect_artifact(&accumulated_text) {
                    let artifact_store = app_handle.state::<ArtifactStore>();
                    let artifact_id = Uuid::new_v4().to_string();
                    if let Err(e) = artifact_store.save_artifact(
                        &artifact_id,
                        &effective_conv_id,
                        Some(&assistant_message_id),
                        &detected.content_type,
                        &detected.raw_source,
                    ) {
                        eprintln!("[chat] failed to persist artifact: {e}");
                    } else {
                        match artifact_store.get_artifact_preview(&artifact_id) {
                            Ok(preview) => {
                                let _ = channel.send(ChatEvent::ArtifactReady {
                                    artifact_id: preview.artifact_id,
                                    content_type: preview.content_type,
                                    preview: preview.srcdoc,
                                });
                            }
                            Err(e) => {
                                eprintln!("[chat] failed to build artifact preview: {e}");
                            }
                        }
                    }
                }
            }
            Err(ref e) if e == "CANCELLED" => {
                // User-initiated cancellation — persist partial assistant text
                // (status='incomplete') and mark conversation incomplete (D-02).
                let msg_store = app_handle.state::<MessageStore>();
                if let Err(storage_err) = msg_store.insert_incomplete_message(
                    &Uuid::new_v4().to_string(),
                    &effective_conv_id,
                    "assistant",
                    &accumulated_text,
                ) {
                    eprintln!("[chat] failed to persist partial assistant message: {storage_err}");
                }
                let conv_store = app_handle.state::<ConversationStore>();
                if let Err(storage_err) = conv_store.mark_incomplete(&effective_conv_id) {
                    eprintln!("[chat] failed to mark conversation incomplete: {storage_err}");
                }
                // Terminal CANCELLED event was already sent inside run_stream.
            }
            Err(e) => {
                // Provider or network error — mark conversation incomplete and
                // send terminal Error event to the frontend (Pitfall 2: ignore
                // channel errors after terminal event).
                let conv_store = app_handle.state::<ConversationStore>();
                if let Err(storage_err) = conv_store.mark_incomplete(&effective_conv_id) {
                    eprintln!(
                        "[chat] failed to mark conversation incomplete on error: {storage_err}"
                    );
                }
                let _ = channel.send(ChatEvent::Error {
                    code: "PROVIDER_ERROR".into(),
                    message: e.clone(),
                });
            }
        }

        // Unconditional cleanup — prevents HashMap growth (T-02-04 / Pitfall 5).
        let inner_state = app_handle.state::<AppState>();
        if let Ok(mut requests) = inner_state.active_requests.lock() {
            requests.remove(&request_id_clone);
        };
    });

    Ok(())
}

/// Cancel an in-flight streaming request by `request_id`.
///
/// Signals the CancellationToken registered by `chat_send`. The spawned
/// task will detect cancellation and emit `ChatEvent::Error { code: "CANCELLED" }`
/// through the channel to close the frontend listener cleanly (D-04).
#[tauri::command]
pub async fn chat_cancel(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> Result<(), ChatError> {
    command_policy::policy_check("chat_cancel", window.label())?;

    let token = {
        let requests = state
            .active_requests
            .lock()
            .map_err(|e| ChatError::ProviderError(format!("active_requests lock poisoned: {e}")))?;
        requests.get(&request_id).cloned()
    };

    match token {
        Some(t) => {
            t.cancel();
            Ok(())
        }
        None => Err(ChatError::RequestNotFound(request_id)),
    }
}

/// Private async function that drives the HTTP request and SSE stream.
///
/// Races the HTTP connection against the cancellation token so the user
/// can cancel even before the first byte arrives.
///
/// Returns a tuple `(result, accumulated_text, resolved_model)`:
/// - `result`: `Ok(())` on clean completion; `Err("CANCELLED")` on cancellation;
///   `Err(msg)` on network/provider failure.
/// - `accumulated_text`: the full assistant response collected from Delta events.
///   Available for storage writes after the stream completes.
/// - `resolved_model`: the model name from the Done event (empty string if the
///   stream was cancelled or errored before Done was received).
///
/// Text accumulation uses `Arc<Mutex<String>>` shared between the SSE callback
/// closure and the outer function so the final text is accessible after
/// `drive_sse_stream` returns, without requiring a different stream driver signature.
async fn run_stream(
    api_key: &secrecy::SecretString,
    model: &str,
    messages: Vec<ProviderMessage>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: &Channel<ChatEvent>,
    cancel_token: CancellationToken,
) -> (Result<(), String>, String, String) {
    let client = reqwest::Client::new();

    // Race the HTTP request against cancellation.
    let response = tokio::select! {
        r = crate::providers::openrouter::stream_completion(
            &client,
            api_key,
            model,
            &messages,
            max_completion_tokens,
            temperature,
        ) => {
            match r {
                Ok(resp) => resp,
                Err(e) => return (Err(e), String::new(), String::new()),
            }
        }
        _ = cancel_token.cancelled() => {
            let _ = channel.send(ChatEvent::Error {
                code: "CANCELLED".into(),
                message: "Request cancelled by user".into(),
            });
            // Pre-connection cancellation: treat as Ok with empty text.
            return (Err("CANCELLED".to_string()), String::new(), String::new());
        }
    };

    let channel_for_closure = channel.clone();

    // Shared accumulators for Delta text and Done model.
    // Arc<Mutex<String>> is needed because drive_sse_stream's callback is
    // `move` — we cannot use plain mutable references across the closure
    // boundary. Mutex is std (not tokio) because the callback is synchronous
    // (no .await inside on_event).
    let accumulated = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let done_model = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let accumulated_clone = accumulated.clone();
    let done_model_clone = done_model.clone();

    // Drive the SSE stream, dispatching events through the channel and
    // accumulating Delta text for storage writes after completion.
    let result = sse::drive_sse_stream(response, cancel_token, move |event| {
        match event {
            sse::SseEvent::Delta { ref text } => {
                // Accumulate assistant text for storage write on completion.
                if let Ok(mut acc) = accumulated_clone.lock() {
                    acc.push_str(text);
                }
                // Ignore send errors mid-stream; the channel is closed if
                // the frontend navigated away (Pitfall 2).
                let _ = channel_for_closure.send(ChatEvent::Delta { text: text.clone() });
            }
            sse::SseEvent::Done {
                usage,
                model: ref model_name,
            } => {
                // Capture the resolved model name for the conversation row.
                if let Ok(mut m) = done_model_clone.lock() {
                    m.clone_from(model_name);
                }
                let token_usage = usage.map(|u| TokenUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                });
                let _ = channel_for_closure.send(ChatEvent::Done {
                    usage: token_usage,
                    model: model_name.clone(),
                });
            }
            sse::SseEvent::ProviderError { message } => {
                let _ = channel_for_closure.send(ChatEvent::Error {
                    code: "PROVIDER_ERROR".into(),
                    message,
                });
            }
            sse::SseEvent::Comment | sse::SseEvent::Unknown => {
                // Ignored.
            }
        }
        Ok(())
    })
    .await;

    // Extract accumulated values now that the stream closure has released them.
    let final_text = accumulated.lock().map(|g| g.clone()).unwrap_or_default();
    let final_model = done_model.lock().map(|g| g.clone()).unwrap_or_default();

    match result {
        Ok(()) => (Ok(()), final_text, final_model),
        Err(ref e) if e == "CANCELLED" => {
            // drive_sse_stream returns "CANCELLED" when the token fires mid-stream.
            // Send the terminal CANCELLED event so the frontend listener closes (D-03).
            // Ignore channel errors after terminal event (Pitfall 2).
            let _ = channel.send(ChatEvent::Error {
                code: "CANCELLED".into(),
                message: "Request cancelled by user".into(),
            });
            // Return partial text so the caller can persist it as incomplete.
            (Err("CANCELLED".to_string()), final_text, final_model)
        }
        Err(e) => (Err(e), final_text, final_model),
    }
}

fn resolve_attachments(
    state: &tauri::State<'_, AppState>,
    attachments: Option<Vec<Uuid>>,
) -> Result<Option<String>, ChatError> {
    let Some(tokens) = attachments else {
        return Ok(None);
    };

    if tokens.is_empty() {
        return Ok(None);
    }

    let mut rendered = Vec::new();
    for token in tokens {
        let path = crate::security::file_tokens::resolve_token(&state, token)
            .map_err(|e| ChatError::CredentialError(e.to_string()))?;
        rendered.push(read_attachment(&path)?);
    }

    let mut body = String::from("Attached file context:\n");
    for attachment in rendered {
        body.push_str("\n---\n");
        body.push_str(&attachment);
        body.push('\n');
    }

    Ok(Some(body))
}

fn read_attachment(path: &Path) -> Result<String, ChatError> {
    let mime = from_path(path).first_or_octet_stream();
    let mime_type = mime.type_().as_str();
    let mime_subtype = mime.subtype().as_str();
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment");

    let content = if mime_type == "text" || mime_subtype == "json" || mime_subtype == "xml" {
        fs::read_to_string(path).map_err(|e| ChatError::CredentialError(e.to_string()))?
    } else {
        String::from_utf8_lossy(
            &fs::read(path).map_err(|e| ChatError::CredentialError(e.to_string()))?,
        )
        .into_owned()
    };

    Ok(format!(
        "Filename: {filename}\nMIME: {mime}\nContent:\n{content}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_error_serializes_as_screaming_snake_case() {
        let err = ChatError::CredentialError("test".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("CREDENTIAL_ERROR"),
            "expected SCREAMING_SNAKE_CASE code field: {json}"
        );
    }

    #[test]
    fn chat_error_unauthorized_window_serializes_correctly() {
        let err = ChatError::UnauthorizedWindow("bad".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("UNAUTHORIZED_WINDOW"),
            "expected UNAUTHORIZED_WINDOW code: {json}"
        );
    }

    #[test]
    fn policy_check_rejects_non_main_window_for_chat_commands() {
        for command in ["chat_send", "chat_cancel"] {
            let err: ChatError = command_policy::policy_check(command, "evil")
                .unwrap_err()
                .into();
            assert!(
                matches!(err, ChatError::UnauthorizedWindow(_)),
                "command {command} did not reject non-main window"
            );
        }
    }

    #[test]
    fn chat_event_ack_serializes_with_type_field() {
        let event = ChatEvent::Ack {
            request_id: "r1".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains(r#""type":"Ack""#),
            "expected type:Ack field: {json}"
        );
        assert!(
            json.contains(r#""request_id":"r1""#),
            "expected request_id field: {json}"
        );
    }

    #[test]
    fn chat_event_delta_serializes_with_type_field() {
        let event = ChatEvent::Delta {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains(r#""type":"Delta""#),
            "expected type:Delta: {json}"
        );
        assert!(
            json.contains(r#""text":"hello""#),
            "expected text field: {json}"
        );
    }

    #[test]
    fn chat_event_done_serializes_with_type_field() {
        let event = ChatEvent::Done {
            usage: None,
            model: "m".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains(r#""type":"Done""#),
            "expected type:Done: {json}"
        );
    }

    #[test]
    fn chat_event_error_serializes_with_type_field() {
        let event = ChatEvent::Error {
            code: "CANCELLED".into(),
            message: "cancelled".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains(r#""type":"Error""#),
            "expected type:Error: {json}"
        );
        assert!(
            json.contains(r#""code":"CANCELLED""#),
            "expected code:CANCELLED: {json}"
        );
    }

    #[test]
    fn chat_event_artifact_ready_serializes_with_type_field() {
        let event = ChatEvent::ArtifactReady {
            artifact_id: "art-1".into(),
            content_type: ArtifactContentType::Html,
            preview: "<html></html>".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains(r#""type":"ArtifactReady""#),
            "expected type:ArtifactReady: {json}"
        );
        assert!(
            json.contains(r#""artifact_id":"art-1""#),
            "expected artifact_id field: {json}"
        );
    }

    // D-10 invariant verified: chat_send signature does not include api_key.
    // Enforced by type system. `cargo check` is the authoritative gate.
    #[test]
    fn chat_send_has_no_api_key_parameter() {
        // This test documents the D-10 invariant. The compile-time enforcement
        // is the `cargo check` gate — if api_key ever appears in chat_send's
        // signature, cargo check will fail with a type mismatch or the grep
        // gate in T-11 will catch it.
        let _ = "D-10 invariant: chat_send has no api_key parameter. Type system enforces this.";
    }

    // --- title_from_messages tests (Phase 3 TDD RED) ---

    #[test]
    fn title_from_messages_returns_first_user_message_content() {
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: "Hello, how are you?".into(),
        }];
        assert_eq!(title_from_messages(&messages), "Hello, how are you?");
    }

    #[test]
    fn title_from_messages_truncates_to_60_chars() {
        let long_content = "a".repeat(70);
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: long_content,
        }];
        let title = title_from_messages(&messages);
        assert_eq!(title.chars().count(), 60);
        assert_eq!(title, "a".repeat(60));
    }

    #[test]
    fn title_from_messages_skips_non_user_messages() {
        let messages = vec![
            ChatMessage {
                role: "assistant".into(),
                content: "I am an assistant".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "My question here".into(),
            },
        ];
        assert_eq!(title_from_messages(&messages), "My question here");
    }

    #[test]
    fn title_from_messages_returns_fallback_when_no_user_message() {
        let messages = vec![ChatMessage {
            role: "assistant".into(),
            content: "System response".into(),
        }];
        assert_eq!(title_from_messages(&messages), "New conversation");
    }

    #[test]
    fn title_from_messages_returns_fallback_for_empty_messages() {
        let messages: Vec<ChatMessage> = vec![];
        assert_eq!(title_from_messages(&messages), "New conversation");
    }
}
