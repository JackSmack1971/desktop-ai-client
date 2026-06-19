/// OpenRouter HTTP adapter.
///
/// Responsible for constructing and sending the HTTP request to OpenRouter's
/// chat completions endpoint. Returns the raw `reqwest::Response` for the
/// SSE stream; parsing is handled by `providers::sse`.
///
/// Security: the API key is received as `&secrecy::SecretString` and exposed
/// only inside the Authorization header construction. It is never logged,
/// formatted into error messages, or returned across an IPC boundary.
///
/// Dependency direction: this module depends on `secrecy` and `reqwest` but
/// must NOT import from `ipc::chat`. Error type is `String` so the dependency
/// stays unidirectional.
use secrecy::ExposeSecret;

/// Default model sent to OpenRouter when the frontend does not specify one.
/// Returned in `ChatEvent::Done { model }` so the frontend can display
/// which model actually answered.
pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-6";

/// Base URL for the OpenRouter API.
pub const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1";

/// A message in the conversation history passed to the provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

/// The JSON body sent to `/chat/completions`.
#[derive(serde::Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: &'a [ProviderMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// Send a streaming chat completion request to OpenRouter.
///
/// Returns the raw HTTP response on success; the caller is responsible for
/// consuming the SSE bytes via `providers::sse::drive_sse_stream`.
///
/// Error cases:
/// - Network error during `send()` → `Err("network error: ...")`
/// - Non-2xx HTTP response → `Err("HTTP {status_code}")`
///
/// The `api_key` is exposed only in the Authorization header and is not
/// stored, logged, or propagated into error messages.
pub async fn stream_completion(
    client: &reqwest::Client,
    api_key: &secrecy::SecretString,
    model: &str,
    messages: &[ProviderMessage],
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<reqwest::Response, String> {
    let body = ChatCompletionRequest {
        model,
        messages,
        stream: true,
        max_completion_tokens,
        temperature,
    };

    let response = client
        .post(format!("{OPENROUTER_BASE}/chat/completions"))
        .header(
            "Authorization",
            format!("Bearer {}", api_key.expose_secret()),
        )
        .header("Content-Type", "application/json")
        // Recommended by OpenRouter for request attribution; no hostname leakage.
        .header("HTTP-Referer", "https://desktop-ai-client")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        return Err(format!("HTTP {status}"));
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_correct() {
        assert_eq!(
            DEFAULT_MODEL, "anthropic/claude-sonnet-4-6",
            "DEFAULT_MODEL constant must match D-13 decision"
        );
    }

    #[test]
    fn provider_message_serializes_role_and_content() {
        let msg = ProviderMessage {
            role: "user".into(),
            content: "hi".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(
            json.contains(r#""role":"user""#),
            "missing role field: {json}"
        );
        assert!(
            json.contains(r#""content":"hi""#),
            "missing content field: {json}"
        );
    }

    #[test]
    fn chat_completion_request_sets_stream_true() {
        let msg = ProviderMessage {
            role: "user".into(),
            content: "hello".into(),
        };
        let req = ChatCompletionRequest {
            model: "test/model",
            messages: &[msg],
            stream: true,
            max_completion_tokens: None,
            temperature: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            json.contains(r#""stream":true"#),
            "stream must be true: {json}"
        );
    }
}
