/// Thin provider routing layer for Phase 2.
///
/// Responsibilities:
/// 1. Select provider — always OpenRouter for Phase 2 (capability-based selection deferred).
/// 2. Prepend the backend-owned system prompt (per D-12).
/// 3. Convert `RoutableMessage` (provider-owned type) to `ProviderMessage` (provider type).
///
/// The system prompt is defined here and never accepted from IPC. Any future
/// user-editable custom instructions must come through a separate settings
/// command with its own validation surface.
///
/// `providers` must not depend on `ipc` (see `.planning/codebase/ARCHITECTURE.md`
/// and `providers/AGENTS.md`), so this module owns its own message type instead
/// of importing `ipc::chat::ChatMessage`. Callers convert at the IPC boundary.
use crate::providers::openrouter::{ProviderMessage, DEFAULT_MODEL};

/// Backend-owned default system prompt prepended to every chat request.
/// Never accepted from the IPC surface (D-12 invariant).
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant.";

/// A single conversation message handed to the routing layer.
///
/// Provider-owned counterpart to `ipc::chat::ChatMessage`. Callers at the IPC
/// boundary convert their wire type into this one so `providers` never
/// depends on `ipc`.
#[derive(Debug, Clone)]
pub struct RoutableMessage {
    pub role: String,
    pub content: String,
}

/// Build the provider message list from a routable message slice.
///
/// Always prepends the backend-owned system prompt as the first message.
/// Maps each `RoutableMessage` to a `ProviderMessage` preserving role and content.
pub fn build_provider_messages(
    system_prompt: &str,
    messages: &[RoutableMessage],
) -> Vec<ProviderMessage> {
    let mut result = Vec::with_capacity(messages.len() + 1);

    // System prompt is always first (D-12).
    result.push(ProviderMessage {
        role: "system".into(),
        content: system_prompt.into(),
    });

    for msg in messages {
        result.push(ProviderMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    result
}

/// Select the model to use for this request.
///
/// Returns the frontend-requested model if `Some`, otherwise the backend
/// default constant. Dynamic capability-based selection is deferred.
pub fn select_model(requested: Option<&str>) -> String {
    requested.unwrap_or(DEFAULT_MODEL).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(content: &str) -> RoutableMessage {
        RoutableMessage {
            role: "user".into(),
            content: content.into(),
        }
    }

    fn assistant_msg(content: &str) -> RoutableMessage {
        RoutableMessage {
            role: "assistant".into(),
            content: content.into(),
        }
    }

    #[test]
    fn build_provider_messages_prepends_system_prompt() {
        let messages = [user_msg("Hello")];
        let result = build_provider_messages(DEFAULT_SYSTEM_PROMPT, &messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system", "first element must be system");
        assert_eq!(result[1].role, "user", "second element must be user");
    }

    #[test]
    fn build_provider_messages_preserves_order() {
        let messages = [
            user_msg("Hi"),
            assistant_msg("Hello!"),
            user_msg("How are you?"),
        ];
        let result = build_provider_messages(DEFAULT_SYSTEM_PROMPT, &messages);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
        assert_eq!(result[3].role, "user");
    }

    #[test]
    fn select_model_returns_default_when_none() {
        let model = select_model(None);
        assert_eq!(
            model, DEFAULT_MODEL,
            "should return DEFAULT_MODEL when no override"
        );
    }

    #[test]
    fn select_model_returns_requested_when_some() {
        let model = select_model(Some("some/other-model"));
        assert_eq!(model, "some/other-model");
    }
}
