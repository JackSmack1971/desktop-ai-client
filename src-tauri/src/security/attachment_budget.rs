/// Attachment intake limits.
///
/// `ipc::chat::resolve_attachments` reads attachment content into a string
/// that gets embedded directly in the provider request. Without a budget,
/// that read is unbounded: an attached file of arbitrary size causes an
/// unbounded process-memory read and an unbounded outbound payload to a
/// third-party API, and a non-text file gets lossily decoded into garbage
/// text instead of being rejected.
///
/// This module only inspects metadata (`fs::metadata`, MIME sniff by
/// extension) — never file content — so a budget violation is caught before
/// any content read happens. Errors carry filenames only, never paths.
use std::path::Path;

/// Limits enforced across one request's full attachment set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentBudget {
    pub max_files: usize,
    pub max_bytes_per_file: u64,
    pub max_total_bytes: u64,
    pub max_estimated_tokens: u64,
}

impl AttachmentBudget {
    /// The reviewed default budget. Revisit if a legitimate use case needs
    /// more headroom — this is a policy choice, not a technical ceiling.
    pub const fn standard() -> Self {
        Self {
            max_files: 5,
            max_bytes_per_file: 2 * 1024 * 1024, // 2 MiB
            max_total_bytes: 6 * 1024 * 1024,    // 6 MiB
            max_estimated_tokens: 50_000,
        }
    }
}

impl Default for AttachmentBudget {
    fn default() -> Self {
        Self::standard()
    }
}

/// Metadata about one attachment, gathered without reading its content.
#[derive(Debug, Clone)]
pub struct AttachmentMeta {
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
    pub mime_subtype: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AttachmentBudgetError {
    #[error("{actual} attachments exceeds the {max} file limit")]
    TooManyFiles { actual: usize, max: usize },
    #[error("attachment '{filename}' is {size} bytes, exceeds the {max}-byte per-file limit")]
    FileTooLarge {
        filename: String,
        size: u64,
        max: u64,
    },
    #[error("attachments total {total} bytes, exceeds the {max}-byte total limit")]
    TotalTooLarge { total: u64, max: u64 },
    #[error("attachments are an estimated {estimated} tokens, exceeds the {max}-token limit")]
    TokenBudgetExceeded { estimated: u64, max: u64 },
    #[error("attachment '{filename}' has unsupported type '{mime_type}/{mime_subtype}' — only text, JSON, and XML attachments are supported")]
    UnsupportedType {
        filename: String,
        mime_type: String,
        mime_subtype: String,
    },
    #[error("failed to read attachment metadata: {0}")]
    Io(String),
}

/// Rough token estimate from a byte count. Conservative (over-estimates for
/// most text) so the budget fails closed rather than under-counting.
pub fn estimate_tokens(total_bytes: u64) -> u64 {
    total_bytes / 4
}

pub fn is_text_like(mime_type: &str, mime_subtype: &str) -> bool {
    mime_type == "text" || mime_subtype == "json" || mime_subtype == "xml"
}

/// Gather metadata for one path without reading its content.
pub fn probe(path: &Path) -> Result<AttachmentMeta, AttachmentBudgetError> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment")
        .to_string();
    let metadata = std::fs::metadata(path).map_err(|e| AttachmentBudgetError::Io(e.to_string()))?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Ok(AttachmentMeta {
        filename,
        size: metadata.len(),
        mime_type: mime.type_().as_str().to_string(),
        mime_subtype: mime.subtype().as_str().to_string(),
    })
}

/// Check a full attachment set against the budget. Fails closed on the first
/// violation found, checking count and type before aggregate size so the
/// cheapest, most informative rejection wins.
pub fn check(
    budget: &AttachmentBudget,
    attachments: &[AttachmentMeta],
) -> Result<(), AttachmentBudgetError> {
    if attachments.len() > budget.max_files {
        return Err(AttachmentBudgetError::TooManyFiles {
            actual: attachments.len(),
            max: budget.max_files,
        });
    }

    for attachment in attachments {
        if !is_text_like(&attachment.mime_type, &attachment.mime_subtype) {
            return Err(AttachmentBudgetError::UnsupportedType {
                filename: attachment.filename.clone(),
                mime_type: attachment.mime_type.clone(),
                mime_subtype: attachment.mime_subtype.clone(),
            });
        }
        if attachment.size > budget.max_bytes_per_file {
            return Err(AttachmentBudgetError::FileTooLarge {
                filename: attachment.filename.clone(),
                size: attachment.size,
                max: budget.max_bytes_per_file,
            });
        }
    }

    let total: u64 = attachments.iter().map(|a| a.size).sum();
    if total > budget.max_total_bytes {
        return Err(AttachmentBudgetError::TotalTooLarge {
            total,
            max: budget.max_total_bytes,
        });
    }

    let estimated = estimate_tokens(total);
    if estimated > budget.max_estimated_tokens {
        return Err(AttachmentBudgetError::TokenBudgetExceeded {
            estimated,
            max: budget.max_estimated_tokens,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_meta(filename: &str, size: u64) -> AttachmentMeta {
        AttachmentMeta {
            filename: filename.to_string(),
            size,
            mime_type: "text".to_string(),
            mime_subtype: "plain".to_string(),
        }
    }

    #[test]
    fn accepts_within_budget() {
        let budget = AttachmentBudget::standard();
        let metas = vec![text_meta("a.txt", 1024), text_meta("b.txt", 2048)];
        assert!(check(&budget, &metas).is_ok());
    }

    #[test]
    fn rejects_too_many_files() {
        let budget = AttachmentBudget {
            max_files: 2,
            ..AttachmentBudget::standard()
        };
        let metas = vec![
            text_meta("a.txt", 10),
            text_meta("b.txt", 10),
            text_meta("c.txt", 10),
        ];
        let err = check(&budget, &metas).unwrap_err();
        assert!(matches!(
            err,
            AttachmentBudgetError::TooManyFiles { actual: 3, max: 2 }
        ));
    }

    #[test]
    fn rejects_single_file_over_per_file_cap() {
        let budget = AttachmentBudget {
            max_bytes_per_file: 100,
            ..AttachmentBudget::standard()
        };
        let metas = vec![text_meta("big.txt", 101)];
        let err = check(&budget, &metas).unwrap_err();
        assert!(matches!(err, AttachmentBudgetError::FileTooLarge { .. }));
    }

    #[test]
    fn rejects_total_over_aggregate_cap_even_if_each_file_is_within_per_file_cap() {
        let budget = AttachmentBudget {
            max_bytes_per_file: 100,
            max_total_bytes: 150,
            ..AttachmentBudget::standard()
        };
        let metas = vec![text_meta("a.txt", 90), text_meta("b.txt", 90)];
        let err = check(&budget, &metas).unwrap_err();
        assert!(matches!(err, AttachmentBudgetError::TotalTooLarge { .. }));
    }

    #[test]
    fn rejects_unsupported_mime_type_without_reading_content() {
        let budget = AttachmentBudget::standard();
        let metas = vec![AttachmentMeta {
            filename: "image.png".to_string(),
            size: 10,
            mime_type: "image".to_string(),
            mime_subtype: "png".to_string(),
        }];
        let err = check(&budget, &metas).unwrap_err();
        assert!(matches!(err, AttachmentBudgetError::UnsupportedType { .. }));
    }

    #[test]
    fn accepts_json_and_xml_subtypes_as_text_like() {
        assert!(is_text_like("application", "json"));
        assert!(is_text_like("application", "xml"));
        assert!(is_text_like("text", "plain"));
        assert!(!is_text_like("image", "png"));
        assert!(!is_text_like("application", "octet-stream"));
    }

    #[test]
    fn rejects_estimated_token_budget_exceeded() {
        let budget = AttachmentBudget {
            max_bytes_per_file: 1_000_000,
            max_total_bytes: 1_000_000,
            max_estimated_tokens: 10,
            ..AttachmentBudget::standard()
        };
        // 100 bytes / 4 = 25 estimated tokens, over the 10-token limit.
        let metas = vec![text_meta("a.txt", 100)];
        let err = check(&budget, &metas).unwrap_err();
        assert!(matches!(
            err,
            AttachmentBudgetError::TokenBudgetExceeded { .. }
        ));
    }

    #[test]
    fn empty_attachment_set_is_always_accepted() {
        let budget = AttachmentBudget::standard();
        assert!(check(&budget, &[]).is_ok());
    }

    #[test]
    fn probe_reads_metadata_without_reading_content() {
        let path = std::env::temp_dir().join(format!("attachment-budget-probe-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"hello world").expect("write temp file");
        let meta = probe(&path).expect("probe should succeed");
        let _ = std::fs::remove_file(&path);
        assert_eq!(meta.size, 11);
        assert_eq!(meta.mime_type, "text");
    }

    #[test]
    fn probe_reports_io_error_for_missing_file() {
        let path = std::env::temp_dir().join(format!("does-not-exist-{}.txt", uuid::Uuid::new_v4()));
        let err = probe(&path).unwrap_err();
        assert!(matches!(err, AttachmentBudgetError::Io(_)));
    }

    proptest::proptest! {
        #[test]
        fn prop_total_never_exceeds_cap_without_rejection(sizes in proptest::collection::vec(1u64..=1_000_000u64, 0..=5)) {
            let budget = AttachmentBudget::standard();
            let metas: Vec<AttachmentMeta> = sizes
                .iter()
                .enumerate()
                .map(|(i, &size)| text_meta(&format!("f{i}.txt"), size))
                .collect();
            let total: u64 = sizes.iter().sum();
            let result = check(&budget, &metas);
            if total > budget.max_total_bytes || metas.iter().any(|m| m.size > budget.max_bytes_per_file) {
                proptest::prop_assert!(result.is_err());
            } else if estimate_tokens(total) > budget.max_estimated_tokens {
                proptest::prop_assert!(result.is_err());
            } else {
                proptest::prop_assert!(result.is_ok());
            }
        }
    }
}
