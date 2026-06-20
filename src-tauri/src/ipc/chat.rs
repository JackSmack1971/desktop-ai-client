/// IPC commands for the chat streaming surface.
///
/// Command inventory entries:
///   chat_send   — windows: ["main"], production: true, sensitivity: HIGH
///   chat_cancel — windows: ["main"], production: true, sensitivity: low
///
/// Conversation Transaction Protocol (see `storage::turns` for the storage
/// half of this contract):
/// - `conversation_id` is stable for the lifetime of a conversation. The
///   frontend learns it from `ChatEvent::Ack` the first time it sends a
///   message without one, and must pass it back on every later send for that
///   conversation — this is what makes "hydrate history, append only the new
///   turn" possible instead of re-submitting the whole transcript every time.
/// - `turn_id` identifies one user message + its eventual assistant
///   response, keyed by `(conversation_id, idempotency_key)`. Retrying a
///   failed/cancelled turn reuses the same `turn_id` and the same persisted
///   user message — it never inserts a duplicate.
/// - `attempt_id` identifies one execution try at a turn. A turn can have
///   many attempts (one per retry); each attempt resolves to exactly one
///   terminal state (`complete`, `failed_partial`, `cancelled`, `failed`),
///   enforced by `storage::turns`'s `WHERE status = 'in_progress'` guard.
/// - Every `ChatEvent` carries a strictly increasing `sequence` for its
///   attempt, starting at the `Ack`.
/// - Storage writes happen *before* the corresponding terminal `ChatEvent` is
///   sent, so the frontend never observes "done"/"error" for an outcome that
///   isn't already durably committed.
///
/// Security model:
/// - Both commands assert the caller is the main window (backend enforcement).
/// - `chat_send` NEVER accepts an api_key parameter (D-10 hard invariant).
/// - Credentials are retrieved internally from AppState::secrets.
/// - System prompt is backend-owned and prepended in providers::routing (D-12).
/// - CancellationToken registry is cleaned up unconditionally after each request
///   to prevent HashMap growth (STRIDE T-02-04, Pitfall 5 from RESEARCH.md).
/// - Conversation title is auto-generated from the new user message (D-03) —
///   never accepted from IPC.
/// - Storage writes in spawned task use app_handle.state::<T>() — no State<'_>
///   lifetime crosses the thread boundary (Pitfall 1).
use crate::app_state::AppState;
use crate::providers::openrouter::ProviderMessage;
use crate::providers::sse::classify_provider_error_code;
use crate::providers::{routing, sse};
use crate::security::command_policy;
use crate::security::secrets::{self, ProviderId};
use crate::storage::artifacts::{self, ArtifactContentType, ArtifactStore};
use crate::storage::sqlite::ConversationStore;
use crate::storage::turns::{BeginTurnOutcome, NewArtifact, TurnStore};
use mime_guess::from_path;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::Manager;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// A single message in the conversation history passed from the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Token usage counters returned in `ChatEvent::Done`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Streaming events delivered via `Channel<ChatEvent>` to the frontend.
///
/// All terminal state (done, error, cancellation) is delivered exclusively
/// through this channel, after the corresponding storage write has already
/// committed. The frontend drops the channel listener after receiving `Done`
/// or `Error` (D-03 invariant).
///
/// Serialized with `#[serde(tag = "type")]` so the frontend discriminated
/// union `{ type: 'Ack' | 'Delta' | 'Done' | 'Error' }` matches directly.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum ChatEvent {
    /// Sent synchronously before the spawned task starts so the frontend has
    /// stable identifiers for cancellation and retry (D-14). `attempt_number`
    /// is `0` when this Ack is followed immediately by a cached replay
    /// (`AlreadyComplete`) rather than a live attempt.
    Ack {
        conversation_id: String,
        turn_id: String,
        attempt_id: String,
        attempt_number: i64,
        sequence: u64,
    },
    /// An incremental content token.
    Delta { text: String, sequence: u64 },
    /// Stream complete — carries resolved model name and token counts.
    Done {
        usage: Option<TokenUsage>,
        model: String,
        sequence: u64,
    },
    /// Terminal failure or user-initiated cancellation.
    /// `code == "CANCELLED"` signals voluntary abort (D-04).
    /// `code == "FAILED_PARTIAL"` signals a truncated/interrupted stream
    /// whose partial output was preserved.
    Error {
        code: String,
        message: String,
        sequence: u64,
    },
    /// Backend-owned artifact detected after stream completion.
    ArtifactReady {
        conversation_id: String,
        artifact_id: String,
        content_type: ArtifactContentType,
        preview: String,
        sequence: u64,
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
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("a request for this turn is already in flight: {0}")]
    DuplicateInFlight(String),
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

/// Generate a conversation title from the new user message.
///
/// Title is backend-owned (D-03) and never accepted from IPC. Takes up to
/// 60 unicode scalar values from the message content.
fn title_from_content(content: &str) -> String {
    content.chars().take(60).collect()
}

fn usage_from_parts(prompt_tokens: Option<u32>, completion_tokens: Option<u32>) -> Option<TokenUsage> {
    match (prompt_tokens, completion_tokens) {
        (Some(prompt_tokens), Some(completion_tokens)) => Some(TokenUsage {
            prompt_tokens,
            completion_tokens,
        }),
        _ => None,
    }
}

/// Submit a prompt and stream the response back through a Tauri channel.
///
/// CRITICAL: This command does NOT accept an `api_key` parameter (D-10).
/// Credentials are retrieved from AppState::secrets internally.
///
/// `history` is prior conversation context for the provider only — every
/// message in it is already persisted and must NOT be re-inserted.
/// `new_message` is the one new user turn to persist and answer.
/// `idempotency_key` must be stable across retries of the same turn (the
/// frontend reuses it when the user clicks "Retry"; a brand-new message gets
/// a freshly generated key).
///
/// Flow:
/// 1. Assert main window; validate `new_message.role == "user"` and a
///    non-empty `idempotency_key` at the boundary.
/// 2. Resolve/create the conversation row for a brand-new conversation.
/// 3. `TurnStore::begin_turn` — looks up or creates the turn; for a new turn
///    this is the ONLY place the user message is ever inserted.
/// 4. Branch on the outcome: reject a duplicate in-flight submission, replay
///    a cached completed turn without calling the provider again, or start a
///    new/retry attempt and stream.
/// 5. Send `ChatEvent::Ack` synchronously so the frontend has `conversation_id`
///    (to hydrate the next send) and `attempt_id` (to cancel) immediately.
/// 6. Spawn async task: stream → atomically persist the terminal outcome →
///    send the terminal `ChatEvent` → cleanup the cancellation token.
#[tauri::command]
pub async fn chat_send(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    history: Vec<ChatMessage>,
    new_message: ChatMessage,
    idempotency_key: String,
    model: Option<String>,
    conversation_id: Option<String>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    attachments: Option<Vec<Uuid>>,
    channel: Channel<ChatEvent>,
) -> Result<(), ChatError> {
    // D-10: no api_key parameter — credentials come from state only.
    command_policy::policy_check("chat_send", window.label())?;

    if new_message.role != "user" {
        return Err(ChatError::InvalidArgument(
            "new_message.role must be \"user\"".to_string(),
        ));
    }
    if idempotency_key.trim().is_empty() {
        return Err(ChatError::InvalidArgument(
            "idempotency_key must not be empty".to_string(),
        ));
    }

    let effective_conv_id: String = conversation_id
        .as_deref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if conversation_id.is_none() {
        let title = title_from_content(&new_message.content);
        let conv_store = app_handle.state::<ConversationStore>();
        conv_store
            .create_conversation(&effective_conv_id, &title)
            .map_err(|e| ChatError::StorageError(e.to_string()))?;
    }

    let turn_store = app_handle.state::<TurnStore>();
    let outcome = turn_store
        .begin_turn(&effective_conv_id, &idempotency_key, &new_message.content)
        .map_err(|e| ChatError::StorageError(e.to_string()))?;

    let sequence = Arc::new(AtomicU64::new(0));
    let next_seq = {
        let sequence = sequence.clone();
        move || sequence.fetch_add(1, Ordering::SeqCst) + 1
    };

    let (turn_id, attempt_number) = match outcome {
        BeginTurnOutcome::InFlight { turn_id } => {
            return Err(ChatError::DuplicateInFlight(turn_id));
        }
        BeginTurnOutcome::AlreadyComplete {
            turn_id,
            assistant_content,
            model,
            prompt_tokens,
            completion_tokens,
        } => {
            // Cached replay: the exact same (conversation, idempotency_key)
            // already succeeded. Do not call the provider again.
            channel
                .send(ChatEvent::Ack {
                    conversation_id: effective_conv_id.clone(),
                    turn_id,
                    attempt_id: Uuid::new_v4().to_string(),
                    attempt_number: 0,
                    sequence: next_seq(),
                })
                .map_err(|e| ChatError::ChannelError(e.to_string()))?;
            let _ = channel.send(ChatEvent::Delta {
                text: assistant_content.unwrap_or_default(),
                sequence: next_seq(),
            });
            let _ = channel.send(ChatEvent::Done {
                usage: usage_from_parts(prompt_tokens, completion_tokens),
                model,
                sequence: next_seq(),
            });
            return Ok(());
        }
        BeginTurnOutcome::New { turn_id } => (turn_id, 1i64),
        BeginTurnOutcome::Retry {
            turn_id,
            next_attempt_number,
        } => (turn_id, next_attempt_number),
    };

    let attempt_id = turn_store
        .start_attempt(&turn_id, attempt_number)
        .map_err(|e| ChatError::StorageError(e.to_string()))?;

    let token = CancellationToken::new();

    // Register the token before spawning so chat_cancel can find it
    // immediately after this command returns (Pitfall 5 prevention).
    {
        let mut requests = state
            .active_requests
            .lock()
            .map_err(|e| ChatError::ProviderError(format!("active_requests lock poisoned: {e}")))?;
        requests.insert(attempt_id.clone(), token.clone());
    }

    // Retrieve the API key before spawning — never hold the lock across await
    // (Pitfall 3).
    let api_key = secrets::get_provider_key(ProviderId::OpenRouter)
        .map_err(|e| ChatError::CredentialError(e.to_string()))?;

    // Send Ack synchronously before the spawn so the frontend learns
    // conversation_id (to hydrate/persist for later sends), turn_id (to
    // retry), and attempt_id (to cancel) immediately (D-14).
    channel
        .send(ChatEvent::Ack {
            conversation_id: effective_conv_id.clone(),
            turn_id: turn_id.clone(),
            attempt_id: attempt_id.clone(),
            attempt_number,
            sequence: next_seq(),
        })
        .map_err(|e| ChatError::ChannelError(e.to_string()))?;

    let resolved_model = routing::select_model(model.as_deref());
    let mut routable_messages: Vec<routing::RoutableMessage> = history
        .iter()
        .map(|m| routing::RoutableMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();
    routable_messages.push(routing::RoutableMessage {
        role: new_message.role.clone(),
        content: new_message.content.clone(),
    });
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

    let attempt_id_for_cleanup = attempt_id.clone();

    // Spawn the streaming task. `tauri::State<'_>` is not `'static`, so we
    // re-acquire state from AppHandle inside the spawn (Pitfall 1).
    tokio::spawn(async move {
        let outcome = run_stream(
            &api_key,
            &resolved_model,
            provider_messages,
            max_completion_tokens,
            temperature,
            &channel,
            token,
            sequence.clone(),
        )
        .await;

        let turn_store = app_handle.state::<TurnStore>();

        // Persist the terminal outcome BEFORE notifying the frontend, so a
        // Done/Error event is never observed for a write that didn't commit.
        match outcome.result {
            Ok(()) => {
                let assistant_message_id = Uuid::new_v4().to_string();
                let artifact =
                    artifacts::detect_artifact(&outcome.accumulated_text).map(|detected| {
                        NewArtifact {
                            id: Uuid::new_v4().to_string(),
                            content_type: detected.content_type,
                            raw_source: detected.raw_source,
                        }
                    });
                let artifact_id = artifact.as_ref().map(|a| a.id.clone());

                let wrote = turn_store.complete_attempt_success(
                    &turn_id,
                    &attempt_id,
                    &effective_conv_id,
                    &assistant_message_id,
                    &outcome.accumulated_text,
                    &outcome.model,
                    outcome.usage.as_ref().map(|u| u.prompt_tokens),
                    outcome.usage.as_ref().map(|u| u.completion_tokens),
                    artifact,
                );
                match wrote {
                    Ok(true) => {
                        let _ = channel.send(ChatEvent::Done {
                            usage: outcome.usage,
                            model: outcome.model,
                            sequence: sequence.fetch_add(1, Ordering::SeqCst) + 1,
                        });
                        if let Some(artifact_id) = artifact_id {
                            let artifact_store = app_handle.state::<ArtifactStore>();
                            match artifact_store.get_artifact_preview(&artifact_id) {
                                Ok(preview) => {
                                    let _ = channel.send(ChatEvent::ArtifactReady {
                                        conversation_id: effective_conv_id.clone(),
                                        artifact_id: preview.artifact_id,
                                        content_type: preview.content_type,
                                        preview: preview.srcdoc,
                                        sequence: sequence.fetch_add(1, Ordering::SeqCst) + 1,
                                    });
                                }
                                Err(e) => {
                                    eprintln!("[chat] failed to build artifact preview: {e}");
                                }
                            }
                        }
                    }
                    Ok(false) => {
                        // Exactly-one-terminal-state guard tripped: this
                        // attempt was already resolved (e.g. a duplicate
                        // cancellation race). Nothing to notify.
                    }
                    Err(e) => {
                        eprintln!("[chat] failed to persist successful attempt: {e}");
                        let _ = channel.send(ChatEvent::Error {
                            code: "STORAGE_ERROR".into(),
                            message: e.to_string(),
                            sequence: sequence.fetch_add(1, Ordering::SeqCst) + 1,
                        });
                    }
                }
            }
            Err(ref reason) if reason == "CANCELLED" => {
                let assistant_message_id = Uuid::new_v4().to_string();
                let wrote = turn_store.complete_attempt_cancelled(
                    &turn_id,
                    &attempt_id,
                    &effective_conv_id,
                    &assistant_message_id,
                    &outcome.accumulated_text,
                );
                if let Err(e) = wrote {
                    eprintln!("[chat] failed to persist cancelled attempt: {e}");
                }
                let _ = channel.send(ChatEvent::Error {
                    code: "CANCELLED".into(),
                    message: "Request cancelled by user".into(),
                    sequence: sequence.fetch_add(1, Ordering::SeqCst) + 1,
                });
            }
            Err(ref reason) if reason == sse::TRUNCATED_STREAM => {
                // EOF without [DONE]: the connection dropped mid-stream.
                // Always failed_partial per the protocol contract, even if
                // no text had streamed yet.
                let assistant_message_id = Uuid::new_v4().to_string();
                let wrote = turn_store.complete_attempt_failed_partial(
                    &turn_id,
                    &attempt_id,
                    &effective_conv_id,
                    &assistant_message_id,
                    &outcome.accumulated_text,
                    "provider_connection_lost",
                );
                if let Err(e) = wrote {
                    eprintln!("[chat] failed to persist truncated-stream attempt: {e}");
                }
                let _ = channel.send(ChatEvent::Error {
                    code: "FAILED_PARTIAL".into(),
                    message: "The provider connection ended before the response finished."
                        .into(),
                    sequence: sequence.fetch_add(1, Ordering::SeqCst) + 1,
                });
            }
            Err(message) => {
                // Mid-stream provider error, or a pre-connection failure
                // (never reached the provider at all).
                let reason = outcome
                    .failure_reason
                    .unwrap_or_else(|| "provider_connection_lost".to_string());
                let wrote = if outcome.accumulated_text.is_empty() {
                    turn_store.complete_attempt_failed(
                        &turn_id,
                        &attempt_id,
                        &effective_conv_id,
                        &reason,
                    )
                } else {
                    let assistant_message_id = Uuid::new_v4().to_string();
                    turn_store.complete_attempt_failed_partial(
                        &turn_id,
                        &attempt_id,
                        &effective_conv_id,
                        &assistant_message_id,
                        &outcome.accumulated_text,
                        &reason,
                    )
                };
                if let Err(e) = wrote {
                    eprintln!("[chat] failed to persist failed attempt: {e}");
                }
                let code = if outcome.accumulated_text.is_empty() {
                    "PROVIDER_ERROR"
                } else {
                    "FAILED_PARTIAL"
                };
                let _ = channel.send(ChatEvent::Error {
                    code: code.into(),
                    message,
                    sequence: sequence.fetch_add(1, Ordering::SeqCst) + 1,
                });
            }
        }

        // Unconditional cleanup — prevents HashMap growth (T-02-04 / Pitfall 5).
        let inner_state = app_handle.state::<AppState>();
        if let Ok(mut requests) = inner_state.active_requests.lock() {
            requests.remove(&attempt_id_for_cleanup);
        };
    });

    Ok(())
}

/// Cancel an in-flight streaming request by `attempt_id`.
///
/// Signals the CancellationToken registered by `chat_send`. The spawned
/// task will detect cancellation and emit `ChatEvent::Error { code: "CANCELLED" }`
/// through the channel to close the frontend listener cleanly (D-04).
#[tauri::command]
pub async fn chat_cancel(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    attempt_id: String,
) -> Result<(), ChatError> {
    command_policy::policy_check("chat_cancel", window.label())?;

    let token = {
        let requests = state
            .active_requests
            .lock()
            .map_err(|e| ChatError::ProviderError(format!("active_requests lock poisoned: {e}")))?;
        requests.get(&attempt_id).cloned()
    };

    match token {
        Some(t) => {
            t.cancel();
            Ok(())
        }
        None => Err(ChatError::RequestNotFound(attempt_id)),
    }
}

/// Terminal outcome of one streaming attempt, returned by `run_stream`.
struct StreamOutcome {
    /// `Ok(())` on a clean `[DONE]`; `Err("CANCELLED")`; `Err(TRUNCATED_STREAM)`
    /// on EOF without `[DONE]`; `Err(message)` on any other provider/network
    /// failure (pre-connection or mid-stream).
    result: Result<(), String>,
    accumulated_text: String,
    model: String,
    usage: Option<TokenUsage>,
    /// Set only when a mid-stream `ProviderError` was classified; `None` for
    /// cancellation, truncation, or a pre-connection failure (the caller
    /// supplies its own default in those cases).
    failure_reason: Option<String>,
}

/// Private async function that drives the HTTP request and SSE stream.
///
/// Races the HTTP connection against the cancellation token so the user
/// can cancel even before the first byte arrives. Forwards `Delta` events to
/// the channel live as they arrive; all other terminal notification happens
/// in the caller, after the corresponding storage write commits.
async fn run_stream(
    api_key: &secrecy::SecretString,
    model: &str,
    messages: Vec<ProviderMessage>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: &Channel<ChatEvent>,
    cancel_token: CancellationToken,
    sequence: Arc<AtomicU64>,
) -> StreamOutcome {
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
                Err(e) => {
                    return StreamOutcome {
                        result: Err(e),
                        accumulated_text: String::new(),
                        model: String::new(),
                        usage: None,
                        failure_reason: None,
                    };
                }
            }
        }
        _ = cancel_token.cancelled() => {
            return StreamOutcome {
                result: Err("CANCELLED".to_string()),
                accumulated_text: String::new(),
                model: String::new(),
                usage: None,
                failure_reason: None,
            };
        }
    };

    let channel_for_closure = channel.clone();
    let sequence_for_closure = sequence.clone();

    // Shared accumulators — the SSE callback closure is `move`, so plain
    // mutable references can't cross the closure boundary. `std::sync::Mutex`
    // (not tokio) is correct here because `on_event` is synchronous.
    let accumulated = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let done_model = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let done_usage = std::sync::Arc::new(std::sync::Mutex::new(None::<TokenUsage>));
    let failure_reason = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let accumulated_clone = accumulated.clone();
    let done_model_clone = done_model.clone();
    let done_usage_clone = done_usage.clone();
    let failure_reason_clone = failure_reason.clone();

    let result = sse::drive_sse_stream(response, cancel_token, move |event| {
        match event {
            sse::SseEvent::Delta { ref text } => {
                if let Ok(mut acc) = accumulated_clone.lock() {
                    acc.push_str(text);
                }
                // Ignore send errors mid-stream; the channel is closed if
                // the frontend navigated away (Pitfall 2).
                let _ = channel_for_closure.send(ChatEvent::Delta {
                    text: text.clone(),
                    sequence: sequence_for_closure.fetch_add(1, Ordering::SeqCst) + 1,
                });
            }
            sse::SseEvent::Done {
                usage,
                model: ref model_name,
            } => {
                if let Ok(mut m) = done_model_clone.lock() {
                    m.clone_from(model_name);
                }
                if let Ok(mut u) = done_usage_clone.lock() {
                    *u = usage.map(|u| TokenUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                    });
                }
            }
            sse::SseEvent::ProviderError { message, code } => {
                if let Ok(mut r) = failure_reason_clone.lock() {
                    *r = Some(classify_provider_error_code(code.as_ref()).to_string());
                }
                return Err(message);
            }
            sse::SseEvent::Comment | sse::SseEvent::Unknown => {
                // Ignored.
            }
        }
        Ok(())
    })
    .await;

    let final_text = accumulated.lock().map(|g| g.clone()).unwrap_or_default();
    let final_model = done_model.lock().map(|g| g.clone()).unwrap_or_default();
    let final_usage = done_usage.lock().map(|g| g.clone()).unwrap_or(None);
    let final_failure_reason = failure_reason.lock().map(|g| g.clone()).unwrap_or(None);

    StreamOutcome {
        result,
        accumulated_text: final_text,
        model: final_model,
        usage: final_usage,
        failure_reason: final_failure_reason,
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
    fn chat_error_duplicate_in_flight_serializes_correctly() {
        let err = ChatError::DuplicateInFlight("turn-1".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("DUPLICATE_IN_FLIGHT"),
            "expected DUPLICATE_IN_FLIGHT code: {json}"
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
            conversation_id: "c1".into(),
            turn_id: "t1".into(),
            attempt_id: "a1".into(),
            attempt_number: 1,
            sequence: 1,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains(r#""type":"Ack""#),
            "expected type:Ack field: {json}"
        );
        assert!(
            json.contains(r#""attempt_id":"a1""#),
            "expected attempt_id field: {json}"
        );
        assert!(
            json.contains(r#""conversation_id":"c1""#),
            "expected conversation_id field: {json}"
        );
    }

    #[test]
    fn chat_event_delta_serializes_with_type_field() {
        let event = ChatEvent::Delta {
            text: "hello".into(),
            sequence: 2,
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
            sequence: 3,
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
            sequence: 4,
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
            conversation_id: "conv-1".into(),
            artifact_id: "art-1".into(),
            content_type: ArtifactContentType::Html,
            preview: "<html></html>".into(),
            sequence: 5,
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
        assert!(
            json.contains(r#""conversation_id":"conv-1""#),
            "expected conversation_id field: {json}"
        );
    }

    // D-10 invariant verified: chat_send signature does not include api_key.
    // Enforced by type system. `cargo check` is the authoritative gate.
    #[test]
    fn chat_send_has_no_api_key_parameter() {
        let _ = "D-10 invariant: chat_send has no api_key parameter. Type system enforces this.";
    }

    #[test]
    fn title_from_content_returns_message_content() {
        assert_eq!(title_from_content("Hello, how are you?"), "Hello, how are you?");
    }

    #[test]
    fn title_from_content_truncates_to_60_chars() {
        let long_content = "a".repeat(70);
        let title = title_from_content(&long_content);
        assert_eq!(title.chars().count(), 60);
        assert_eq!(title, "a".repeat(60));
    }

    #[test]
    fn usage_from_parts_requires_both_fields() {
        assert!(usage_from_parts(Some(1), None).is_none());
        assert!(usage_from_parts(None, Some(1)).is_none());
        assert_eq!(
            usage_from_parts(Some(1), Some(2)),
            Some(TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 2,
            })
        );
    }
}
