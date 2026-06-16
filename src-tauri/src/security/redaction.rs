/// Unconditional redaction gate before any log or telemetry write.
///
/// Secrets, raw file paths, and content-bearing data are replaced in full.
pub const REDACTED: &str = "[REDACTED]";

pub fn redact_secret(_value: &str) -> String {
    REDACTED.to_string()
}

pub fn redact_path(_path: &std::path::Path) -> String {
    REDACTED.to_string()
}

pub fn redact_content(_content: &str) -> String {
    REDACTED.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn secrets_are_total_replacement() {
        let redacted = redact_secret("sk-or-v1-abc123");
        assert_eq!(redacted, REDACTED);
        assert!(!redacted.contains("sk-or-v1-abc123"));
    }

    #[test]
    fn paths_are_total_replacement() {
        assert_eq!(redact_path(Path::new("/home/user/secret.txt")), REDACTED);
    }

    #[test]
    fn content_is_total_replacement() {
        assert_eq!(redact_content("prompt text"), REDACTED);
    }

    #[test]
    fn output_is_unconditional() {
        assert_eq!(redact_secret("a"), redact_secret("b"));
        assert_eq!(redact_path(Path::new("a")), redact_path(Path::new("b")));
        assert_eq!(redact_content("a"), redact_content("b"));
    }
}
