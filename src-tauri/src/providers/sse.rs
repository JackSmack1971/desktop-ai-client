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
    ProviderError { message: String },
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

/// Drive an SSE response stream, calling `on_event` for each meaningful event.
///
/// Maintains `line_buf` across chunks to handle TCP fragmentation (Pitfall 8).
/// Integrates `cancel_token` per-chunk via `tokio::select!` so cancellation is
/// detected promptly without waiting for the next network chunk.
///
/// Returns `Ok(())` on clean stream end or cancellation.
/// Returns `Err(message)` on network or parse failure.
///
/// Note: `drive_sse_stream` does not call `channel.send()` — that is the
/// caller's responsibility so this module stays free of Tauri imports.
pub async fn drive_sse_stream(
    response: reqwest::Response,
    cancel_token: tokio_util::sync::CancellationToken,
    mut on_event: impl FnMut(SseEvent) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let mut stream = response.bytes_stream();
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
            None => break, // Stream exhausted without [DONE] — treat as done.
            Some(Err(e)) => return Err(e.to_string()),
            Some(Ok(b)) => b,
        };

        let text = String::from_utf8_lossy(&bytes);
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
                    })
                    .map_err(|e| e)?;
                    return Ok(());
                }
                Some(SseEvent::Delta { text }) => {
                    on_event(SseEvent::Delta { text }).map_err(|e| e)?;
                }
                Some(SseEvent::ProviderError { message }) => {
                    on_event(SseEvent::ProviderError { message }).map_err(|e| e)?;
                }
                Some(SseEvent::Comment) | Some(SseEvent::Unknown) | None => {
                    // Skip comments, unknown lines, and empty returns.
                }
            }
        }
    }

    Ok(())
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
}
