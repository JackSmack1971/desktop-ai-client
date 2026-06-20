/// Reviewed model capability table — the single source of truth for which
/// models `providers::policy` is allowed to route to, and what each one is
/// capable of.
///
/// `supports_strict_privacy` is a reviewed claim about the provider's
/// data-retention behavior for that model (e.g. zero-data-retention
/// eligibility), not something this code can verify at runtime. Update it
/// only after confirming the upstream provider's current policy — treat this
/// table with the same care as `security/command-inventory.toml`.
use crate::providers::openrouter::DEFAULT_MODEL;

/// Backend-credential identifier for a provider. Distinct from
/// `security::secrets::ProviderId` (the keychain account selector) — this one
/// is provider-routing's own type so `providers` never depends on `security`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    OpenRouter,
}

/// Reviewed capability and bounds entry for one allow-listed model.
#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub id: &'static str,
    pub provider: ProviderId,
    /// Exactly one entry in `MODEL_ALLOWLIST` must set this to `true`.
    pub is_default: bool,
    /// Whether this model is reviewed as eligible for `PrivacyMode::Strict`.
    pub supports_strict_privacy: bool,
    pub max_completion_tokens_cap: u32,
    pub default_max_completion_tokens: u32,
    pub min_temperature: f32,
    pub max_temperature: f32,
    pub default_temperature: f32,
}

/// The reviewed allowlist. A model id not present here is rejected by
/// `providers::policy::resolve_execution_profile` rather than forwarded to a
/// provider unchecked.
pub const MODEL_ALLOWLIST: &[ModelSpec] = &[
    ModelSpec {
        id: DEFAULT_MODEL,
        provider: ProviderId::OpenRouter,
        is_default: true,
        supports_strict_privacy: true,
        max_completion_tokens_cap: 8192,
        default_max_completion_tokens: 2048,
        min_temperature: 0.0,
        max_temperature: 2.0,
        default_temperature: 1.0,
    },
    ModelSpec {
        id: "anthropic/claude-haiku-4-5",
        provider: ProviderId::OpenRouter,
        is_default: false,
        supports_strict_privacy: false,
        max_completion_tokens_cap: 4096,
        default_max_completion_tokens: 1024,
        min_temperature: 0.0,
        max_temperature: 2.0,
        default_temperature: 1.0,
    },
];

/// Look up a model by id. `None` means the model is not allow-listed.
pub fn find_model(id: &str) -> Option<&'static ModelSpec> {
    MODEL_ALLOWLIST.iter().find(|m| m.id == id)
}

/// The reviewed default model — used when the caller does not pin a model.
///
/// Panics if `MODEL_ALLOWLIST` does not contain exactly one `is_default`
/// entry; this is a reviewed-config invariant, not a runtime condition, so a
/// panic here means the table itself is broken and must be fixed, not
/// recovered from.
pub fn default_model_spec() -> &'static ModelSpec {
    MODEL_ALLOWLIST
        .iter()
        .find(|m| m.is_default)
        .expect("MODEL_ALLOWLIST must contain exactly one default model")
}

/// The first allow-listed model that satisfies strict privacy, if any. Used
/// by `providers::policy` as the fallback target when the caller asks for
/// `PrivacyMode::Strict` without pinning a specific model.
pub fn strict_privacy_fallback() -> Option<&'static ModelSpec> {
    MODEL_ALLOWLIST.iter().find(|m| m.supports_strict_privacy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_one_default_model() {
        let defaults = MODEL_ALLOWLIST.iter().filter(|m| m.is_default).count();
        assert_eq!(defaults, 1, "MODEL_ALLOWLIST must have exactly one default");
    }

    #[test]
    fn model_ids_are_unique() {
        let mut ids: Vec<&str> = MODEL_ALLOWLIST.iter().map(|m| m.id).collect();
        ids.sort_unstable();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids, deduped, "MODEL_ALLOWLIST contains duplicate model ids");
    }

    #[test]
    fn default_model_supports_strict_privacy() {
        // Required so PrivacyMode::Strict with no explicit model always
        // resolves without needing a fallback.
        assert!(default_model_spec().supports_strict_privacy);
    }

    #[test]
    fn default_model_spec_matches_openrouter_default_model_constant() {
        assert_eq!(default_model_spec().id, DEFAULT_MODEL);
    }

    #[test]
    fn every_model_has_sane_bounds() {
        for spec in MODEL_ALLOWLIST {
            assert!(
                spec.default_max_completion_tokens <= spec.max_completion_tokens_cap,
                "{} default tokens exceed its own cap",
                spec.id
            );
            assert!(
                spec.min_temperature <= spec.default_temperature
                    && spec.default_temperature <= spec.max_temperature,
                "{} default temperature outside its own range",
                spec.id
            );
        }
    }

    #[test]
    fn find_model_returns_none_for_unlisted_id() {
        assert!(find_model("totally/not-a-real-model").is_none());
    }

    #[test]
    fn find_model_returns_some_for_default() {
        assert!(find_model(DEFAULT_MODEL).is_some());
    }

    #[test]
    fn strict_privacy_fallback_is_present_and_compliant() {
        let fallback = strict_privacy_fallback().expect("a strict-capable model must exist");
        assert!(fallback.supports_strict_privacy);
    }
}
