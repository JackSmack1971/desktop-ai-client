/// Provider message assembly.
///
/// Responsibilities:
/// 1. Prepend the backend-owned system prompt (per D-12).
/// 2. Convert `policy::ValidatedMessage` (role-checked at the policy
///    boundary) into `ProviderMessage` (the provider wire type).
///
/// Model/token/temperature selection no longer lives here — see
/// `providers::policy::resolve_execution_profile`, which resolves those
/// against the reviewed allowlist in `providers::capabilities` before
/// `ipc::chat` ever calls this module.
///
/// The system prompt is defined here and never accepted from IPC. Any future
/// user-editable custom instructions must come through a separate settings
/// command with its own validation surface.
use crate::providers::openrouter::ProviderMessage;
use crate::providers::policy::ValidatedMessage;

/// Backend-owned default system prompt prepended to every chat request.
/// Never accepted from the IPC surface (D-12 invariant).
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant.";

/// Build the provider message list from role-validated messages.
///
/// Always prepends the backend-owned system prompt as the first message.
/// Maps each `ValidatedMessage` to a `ProviderMessage`, preserving order.
pub fn build_provider_messages(
    system_prompt: &str,
    messages: &[ValidatedMessage],
) -> Vec<ProviderMessage> {
    let mut result = Vec::with_capacity(messages.len() + 1);

    // System prompt is always first (D-12).
    result.push(ProviderMessage {
        role: "system".into(),
        content: system_prompt.into(),
    });

    for msg in messages {
        result.push(ProviderMessage {
            role: msg.role.as_str().into(),
            content: msg.content.clone(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::policy::Role;

    fn user_msg(content: &str) -> ValidatedMessage {
        ValidatedMessage {
            role: Role::User,
            content: content.into(),
        }
    }

    fn assistant_msg(content: &str) -> ValidatedMessage {
        ValidatedMessage {
            role: Role::Assistant,
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
}
