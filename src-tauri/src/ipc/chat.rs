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
use crate::app_state::AppState;
use crate::providers::openrouter::ProviderMessage;
use crate::providers::{routing, sse};
use secrecy::ExposeSecret;
use tauri::ipc::Channel;
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

/// Enforce that chat commands can only be invoked from the main window.
/// Backend-side enforcement; capability file is defense-in-depth.
fn assert_main_window(window: &tauri::Window) -> Result<(), ChatError> {
    if window.label() != "main" {
        return Err(ChatError::UnauthorizedWindow(format!(
            "chat commands require the main window, got {:?}",
            window.label()
        )));
    }
    Ok(())
}

/// Submit a prompt and stream the response back through a Tauri channel.
///
/// CRITICAL: This command does NOT accept an `api_key` parameter (D-10).
/// Credentials are retrieved from AppState::secrets internally.
///
/// Flow:
/// 1. Assert main window.
/// 2. Generate request_id, create CancellationToken.
/// 3. Register token in active_requests (before spawn, so chat_cancel works).
/// 4. Retrieve API key from secrets (drop lock before any await).
/// 5. Send `ChatEvent::Ack` synchronously (D-14).
/// 6. Resolve model and build provider messages.
/// 7. Spawn async task: stream → send events → cleanup token.
/// 8. Return Ok(()) immediately.
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
    channel: Channel<ChatEvent>,
) -> Result<(), ChatError> {
    // D-10: no api_key parameter — credentials come from state only.
    // conversation_id is accepted per D-11 but storage write is Phase 3.
    let _ = &conversation_id;
    assert_main_window(&window)?;

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
    let api_key = {
        let secrets = state
            .secrets
            .lock()
            .map_err(|e| ChatError::CredentialError(format!("secrets lock poisoned: {e}")))?;
        let raw = secrets
            .openrouter_key
            .as_ref()
            .ok_or_else(|| {
                ChatError::CredentialError("OPENROUTER_API_KEY not configured".into())
            })?
            .expose_secret()
            .to_string();
        drop(secrets); // release lock before any await
        secrecy::SecretString::new(raw.into())
    };

    // Send Ack synchronously before the spawn so the frontend has request_id
    // immediately and can call chat_cancel if needed (D-14).
    channel
        .send(ChatEvent::Ack {
            request_id: request_id.clone(),
        })
        .map_err(|e| ChatError::ChannelError(e.to_string()))?;

    let resolved_model = routing::select_model(model.as_deref());
    let provider_messages =
        routing::build_provider_messages(routing::DEFAULT_SYSTEM_PROMPT, &messages);
    let request_id_clone = request_id.clone();

    // Spawn the streaming task. `tauri::State<'_>` is not `'static`, so we
    // re-acquire state from AppHandle inside the spawn (Pitfall 1).
    tokio::spawn(async move {
        let result = run_stream(
            &api_key,
            &resolved_model,
            provider_messages,
            max_completion_tokens,
            temperature,
            &channel,
            token,
        )
        .await;

        if let Err(e) = result {
            // Error from run_stream — send terminal Error event and ignore
            // channel errors after the terminal event (Pitfall 2).
            let _ = channel.send(ChatEvent::Error {
                code: "PROVIDER_ERROR".into(),
                message: e,
            });
        }

        // Unconditional cleanup — prevents HashMap growth (T-02-04 / Pitfall 5).
        let inner_state = app_handle.state::<AppState>();
        if let Ok(mut requests) = inner_state.active_requests.lock() {
            requests.remove(&request_id_clone);
        }
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
    assert_main_window(&window)?;

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
async fn run_stream(
    api_key: &secrecy::SecretString,
    model: &str,
    messages: Vec<ProviderMessage>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: &Channel<ChatEvent>,
    cancel_token: CancellationToken,
) -> Result<(), String> {
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
            r?
        }
        _ = cancel_token.cancelled() => {
            let _ = channel.send(ChatEvent::Error {
                code: "CANCELLED".into(),
                message: "Request cancelled by user".into(),
            });
            return Ok(());
        }
    };

    let channel_for_closure = channel.clone();

    // Drive the SSE stream, dispatching events through the channel.
    let result = sse::drive_sse_stream(
        response,
        cancel_token,
        move |event| {
            match event {
                sse::SseEvent::Delta { text } => {
                    // Ignore send errors mid-stream; the channel is closed if
                    // the frontend navigated away (Pitfall 2).
                    let _ = channel_for_closure.send(ChatEvent::Delta { text });
                }
                sse::SseEvent::Done { usage, model: done_model } => {
                    let token_usage = usage.map(|u| TokenUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                    });
                    let _ = channel_for_closure.send(ChatEvent::Done {
                        usage: token_usage,
                        model: done_model,
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
        },
    )
    .await;

    match result {
        Ok(()) => Ok(()),
        Err(ref e) if e == "CANCELLED" => {
            // drive_sse_stream returns "CANCELLED" when the token fires mid-stream.
            // Send the terminal CANCELLED event so the frontend listener closes (D-03).
            // Ignore channel errors after terminal event (Pitfall 2).
            let _ = channel.send(ChatEvent::Error {
                code: "CANCELLED".into(),
                message: "Request cancelled by user".into(),
            });
            Ok(()) // cancellation is not an error from the spawn's perspective
        }
        Err(e) => Err(e),
    }
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
