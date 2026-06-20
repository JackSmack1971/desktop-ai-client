/// SSE (Server-Sent Events) line parser and stream driver for OpenRouter responses.
///
/// This module is intentionally free of Tauri imports. It takes `reqwest::Response`
/// and a callback, returning parsed events through the callback. All channel.send()
/// calls happen in the caller (`ipc::chat`).
///
/// See RESEARCH.md Section 2 for the OpenRouter SSE format and pitfall list.
use futures_util::StreamExt;

/// Token usage counters included in the final SSE chunk before `[DONE]`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SseUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Deserialization target for an OpenRouter SSE chunk.
#[derive(Debug, serde::Deserialize)]
struct SseChunk {
    pub model: Option<String>,
    pub usage: Option<SseUsage>,
    #[serde(default)]
    pub choices: Vec<SseChoice>,
    pub error: Option<SseChunkError>,
}

/// One completion choice within a streaming chunk.
#[derive(Debug, serde::Deserialize)]
struct SseChoice {
    pub delta: Option<SseDelta>,
    pub finish_reason: Option<String>,
}

/// The delta payload within a choice — carries the incremental content.
#[derive(Debug, serde::Deserialize)]
struct SseDelta {
    pub content: Option<String>,
    pub role: Option<String>,
}

/// A mid-stream error object inside the SSE payload (HTTP 200, error in body).
#[derive(Debug, serde::Deserialize)]
struct SseChunkError {
    pub message: String,
    pub code: Option<serde_json::Value>,
}

/// Classify a mid-stream provider error's `code` into the failure-reason
/// taxonomy `ipc::chat` persists on `turn_attempts.failure_reason`.
///
/// Best-effort: OpenRouter's embedded error `code` is usually the upstream
/// HTTP status as a number or numeric string. Anything not recognized falls
/// back to the generic `provider_protocol_error` bucket.
pub fn classify_provider_error_code(code: Option<&serde_json::Value>) -> &'static str {
    let numeric = code.and_then(|c| {
        c.as_u64()
            .or_else(|| c.as_str().and_then(|s| s.parse::<u64>().ok()))
    });
    match numeric {
        Some(401) | Some(403) => "provider_auth_failed",
        Some(429) => "provider_rate_limited",
        Some(408) => "provider_timeout",
        _ => "provider_protocol_error",
    }
}

/// Parsed events produced by the SSE line parser.
#[derive(Debug)]
pub enum SseEvent {
    /// An incremental content token from the model.
    Delta { text: String },
    /// Stream complete. Includes final model name and token usage if available.
    Done {
        usage: Option<SseUsage>,
        model: String,
    },
    /// A provider error embedded inside the SSE stream (non-HTTP error).
    ProviderError {
        message: String,
        code: Option<serde_json::Value>,
    },
    /// An SSE comment line (e.g., `: OPENROUTER PROCESSING`). Ignored.
    Comment,
    /// Any SSE line that is not `data: ...` and not a comment. Ignored.
    Unknown,
}

/// Parse a single SSE line and return a typed event, or `None` for empty lines.
///
/// Rules:
/// - Empty line → `None`
/// - `: ...` → `Some(SseEvent::Comment)`
/// - `data: [DONE]` → `Some(SseEvent::Done { ... })` (sentinel; usage/model filled by caller)
/// - `data: <JSON>` → parsed; error key takes precedence over delta content
/// - Other lines → `Some(SseEvent::Unknown)`
pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    if line.is_empty() {
        return None;
    }

    if line.starts_with(':') {
        return Some(SseEvent::Comment);
    }

    let Some(data) = line.strip_prefix("data: ") else {
        return Some(SseEvent::Unknown);
    };

    if data.trim() == "[DONE]" {
        return Some(SseEvent::Done {
            usage: None,
            model: String::new(),
        });
    }

    match serde_json::from_str::<SseChunk>(data) {
        Ok(chunk) => {
            // Mid-stream provider error takes precedence over delta content.
            if let Some(err) = chunk.error {
                return Some(SseEvent::ProviderError {
                    message: err.message,
                    code: err.code,
                });
            }

            // Collect non-empty delta content across all choices.
            let text: String = chunk
                .choices
                .iter()
                .filter_map(|c| c.delta.as_ref())
                .filter_map(|d| d.content.as_ref())
                .filter(|c| !c.is_empty())
                .cloned()
                .collect();

            if text.is_empty() {
                None
            } else {
                Some(SseEvent::Delta { text })
            }
        }
        Err(e) => {
            log::warn!("SSE parse error on line: {e:?} — skipping chunk");
            None
        }
    }
}

/// Sentinel error string returned when the byte stream ends (EOF) without
/// ever delivering a `data: [DONE]` line. The caller (`ipc::chat`) maps this
/// to a `failed_partial` terminal attempt state rather than treating a quiet
/// connection drop as a successful completion.
pub const TRUNCATED_STREAM: &str = "TRUNCATED_STREAM";

/// Drive an SSE response stream, calling `on_event` for each meaningful event.
///
/// Maintains `line_buf` across chunks to handle TCP fragmentation (Pitfall 8).
/// Integrates `cancel_token` per-chunk via `tokio::select!` so cancellation is
/// detected promptly without waiting for the next network chunk.
///
/// Returns `Ok(())` only when a `data: [DONE]` line was actually observed.
/// Returns `Err(TRUNCATED_STREAM)` when the connection ends without `[DONE]`.
/// Returns `Err(message)` on network or parse failure, or `Err("CANCELLED")`.
///
/// Note: `drive_sse_stream` does not call `channel.send()` — that is the
/// caller's responsibility so this module stays free of Tauri imports.
pub async fn drive_sse_stream(
    response: reqwest::Response,
    cancel_token: tokio_util::sync::CancellationToken,
    on_event: impl FnMut(SseEvent) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let stream = response
        .bytes_stream()
        .map(|chunk| chunk.map_err(|e| e.to_string()));
    drive_byte_stream(stream, cancel_token, on_event).await
}

/// Pure byte-stream driver behind `drive_sse_stream`, generic over the chunk
/// type so it can be exercised in tests without a live `reqwest::Response`.
async fn drive_byte_stream<S, B>(
    mut stream: S,
    cancel_token: tokio_util::sync::CancellationToken,
    mut on_event: impl FnMut(SseEvent) -> Result<(), String> + Send + 'static,
) -> Result<(), String>
where
    S: futures_util::Stream<Item = Result<B, String>> + Unpin,
    B: AsRef<[u8]>,
{
    let mut line_buf = String::new();
    let mut final_model: String = String::new();
    let mut final_usage: Option<SseUsage> = None;

    loop {
        let chunk_result = tokio::select! {
            chunk = stream.next() => chunk,
            _ = cancel_token.cancelled() => {
                return Err("CANCELLED".to_string());
            }
        };

        let bytes = match chunk_result {
            // Stream exhausted without ever seeing `data: [DONE]` — a real
            // provider connection (timeout, dropped TCP, proxy cutoff) that
            // never delivered its terminal sentinel. Distinct from a clean
            // `[DONE]`, which returns `Ok(())` below instead.
            None => return Err(TRUNCATED_STREAM.to_string()),
            Some(Err(e)) => return Err(e),
            Some(Ok(b)) => b,
        };

        let text = String::from_utf8_lossy(bytes.as_ref()).into_owned();
        line_buf.push_str(&text);

        // Process every complete line in the buffer.
        while let Some(newline_pos) = line_buf.find('\n') {
            let line = line_buf[..newline_pos].trim_end_matches('\r').to_string();
            line_buf.drain(..=newline_pos);

            // Accumulate model and usage from each chunk for the Done event.
            // We need to peek at raw JSON before calling parse_sse_line because
            // parse_sse_line returns Done only for [DONE] sentinel lines.
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() != "[DONE]" {
                    if let Ok(chunk) = serde_json::from_str::<SseChunk>(data) {
                        if let Some(m) = &chunk.model {
                            if !m.is_empty() {
                                final_model.clone_from(m);
                            }
                        }
                        if let Some(u) = chunk.usage {
                            final_usage = Some(u);
                        }
                    }
                }
            }

            match parse_sse_line(&line) {
                Some(SseEvent::Done { .. }) => {
                    // Fill in the tracked model and usage.
                    on_event(SseEvent::Done {
                        usage: final_usage.clone(),
                        model: final_model.clone(),
                    })?;
                    return Ok(());
                }
                Some(SseEvent::Delta { text }) => {
                    on_event(SseEvent::Delta { text })?;
                }
                Some(SseEvent::ProviderError { message, code }) => {
                    // Stop driving the stream on a mid-stream provider error
                    // (backend.md: "after the first delta, must not silently
                    // continue"; on partial failure, stop the upstream
                    // request). The caller still receives the message via
                    // `on_event`'s side effect before this propagates.
                    on_event(SseEvent::ProviderError {
                        message: message.clone(),
                        code,
                    })?;
                    return Err(message);
                }
                Some(SseEvent::Comment) | Some(SseEvent::Unknown) | None => {
                    // Skip comments, unknown lines, and empty returns.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_line_extracts_delta_content() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#;
        let event = parse_sse_line(line);
        match event {
            Some(SseEvent::Delta { text }) => assert_eq!(text, "Hello"),
            other => panic!("expected Delta, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_line_ignores_comment_lines() {
        let line = ": OPENROUTER PROCESSING";
        let event = parse_sse_line(line);
        assert!(
            matches!(event, Some(SseEvent::Comment)),
            "expected Comment, got {:?}",
            event
        );
    }

    #[test]
    fn parse_sse_line_handles_done_sentinel() {
        let line = "data: [DONE]";
        let event = parse_sse_line(line);
        assert!(
            matches!(event, Some(SseEvent::Done { .. })),
            "expected Done, got {:?}",
            event
        );
    }

    #[test]
    fn parse_sse_line_returns_none_for_empty() {
        let event = parse_sse_line("");
        assert!(event.is_none(), "expected None for empty line");
    }

    #[test]
    fn parse_sse_line_detects_mid_stream_error() {
        let line = r#"data: {"error":{"message":"rate limited","code":429}}"#;
        let event = parse_sse_line(line);
        assert!(
            matches!(event, Some(SseEvent::ProviderError { .. })),
            "expected ProviderError, got {:?}",
            event
        );
    }

    #[test]
    fn parse_sse_line_skips_delta_with_empty_content() {
        let line = r#"data: {"choices":[{"delta":{"content":""}}]}"#;
        let event = parse_sse_line(line);
        assert!(
            event.is_none(),
            "expected None for empty delta content, got {:?}",
            event
        );
    }

    #[test]
    fn classify_provider_error_code_maps_known_statuses() {
        assert_eq!(
            classify_provider_error_code(Some(&serde_json::json!(429))),
            "provider_rate_limited"
        );
        assert_eq!(
            classify_provider_error_code(Some(&serde_json::json!(401))),
            "provider_auth_failed"
        );
        assert_eq!(
            classify_provider_error_code(Some(&serde_json::json!("403"))),
            "provider_auth_failed"
        );
        assert_eq!(
            classify_provider_error_code(Some(&serde_json::json!(500))),
            "provider_protocol_error"
        );
        assert_eq!(classify_provider_error_code(None), "provider_protocol_error");
    }

    fn byte_chunks(lines: &[&str]) -> Vec<Result<Vec<u8>, String>> {
        lines
            .iter()
            .map(|l| Ok(format!("{l}\n").into_bytes()))
            .collect()
    }

    #[tokio::test]
    async fn drive_byte_stream_returns_ok_when_done_sentinel_observed() {
        let chunks = byte_chunks(&[
            r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#,
            "data: [DONE]",
        ]);
        let stream = futures_util::stream::iter(chunks);
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let events_clone = events.clone();
        let result = drive_byte_stream(stream, tokio_util::sync::CancellationToken::new(), move |event| {
            events_clone.lock().unwrap().push(format!("{event:?}"));
            Ok(())
        })
        .await;
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert!(events.lock().unwrap().iter().any(|e| e.contains("Done")));
    }

    #[tokio::test]
    async fn drive_byte_stream_returns_truncated_stream_when_done_never_arrives() {
        let chunks = byte_chunks(&[r#"data: {"choices":[{"delta":{"content":"partial"}}]}"#]);
        let stream = futures_util::stream::iter(chunks);
        let result = drive_byte_stream(stream, tokio_util::sync::CancellationToken::new(), |_| Ok(()))
            .await;
        assert_eq!(result, Err(TRUNCATED_STREAM.to_string()));
    }

    #[tokio::test]
    async fn drive_byte_stream_returns_truncated_stream_for_empty_input() {
        let stream = futures_util::stream::iter(Vec::<Result<Vec<u8>, String>>::new());
        let result = drive_byte_stream(stream, tokio_util::sync::CancellationToken::new(), |_| Ok(()))
            .await;
        assert_eq!(result, Err(TRUNCATED_STREAM.to_string()));
    }

    #[tokio::test]
    async fn drive_byte_stream_stops_on_mid_stream_provider_error() {
        let chunks = byte_chunks(&[
            r#"data: {"choices":[{"delta":{"content":"start"}}]}"#,
            r#"data: {"error":{"message":"rate limited","code":429}}"#,
            // This Delta must NOT be observed — the stream must stop at the error.
            r#"data: {"choices":[{"delta":{"content":"should-not-appear"}}]}"#,
            "data: [DONE]",
        ]);
        let stream = futures_util::stream::iter(chunks);
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let events_clone = events.clone();
        let result = drive_byte_stream(stream, tokio_util::sync::CancellationToken::new(), move |event| {
            events_clone.lock().unwrap().push(format!("{event:?}"));
            Ok(())
        })
        .await;
        assert_eq!(result, Err("rate limited".to_string()));
        let seen = events.lock().unwrap();
        assert!(!seen.iter().any(|e| e.contains("should-not-appear")));
    }

    #[tokio::test]
    async fn drive_byte_stream_returns_cancelled_when_token_fires() {
        let token = tokio_util::sync::CancellationToken::new();
        token.cancel();
        // `stream::pending()` never resolves, so `tokio::select!` against an
        // already-cancelled token can only take the cancellation branch —
        // an always-ready `stream::iter` would race nondeterministically.
        let stream = futures_util::stream::pending::<Result<Vec<u8>, String>>();
        let result = drive_byte_stream(stream, token, |_| Ok(())).await;
        assert_eq!(result, Err("CANCELLED".to_string()));
    }
}
