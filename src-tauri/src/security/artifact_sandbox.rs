use crate::storage::artifacts::ArtifactContentType;

/// Content-Security-Policy used for all Phase 5 artifact previews.
pub const ARTIFACT_CSP: &str = "default-src 'none'; img-src data: blob:; font-src data: blob:; style-src 'unsafe-inline'; script-src 'none'; connect-src 'none'; form-action 'none'; object-src 'none'; base-uri 'none'; frame-src 'none'";

#[derive(Debug, thiserror::Error)]
pub enum ArtifactSandboxError {
    #[error("unsupported artifact content type")]
    UnsupportedContentType,
}

/// Remove the most dangerous executable HTML constructs from raw artifact input.
///
/// Phase 5 is static-preview only, so this sanitizer is deliberately narrow:
/// it strips script blocks, removes inline event handlers, and neutralizes
/// javascript: URLs before the content crosses IPC.
pub fn sanitize(raw: &str) -> String {
    let without_scripts = strip_script_blocks(raw);
    let without_handlers = strip_inline_event_handlers(&without_scripts);
    neutralize_javascript_uris(&without_handlers)
}

/// Wrap sanitized content in a minimal HTML document with the Phase 5 CSP.
///
/// The wrapper always includes a base URL and title so `srcdoc` rendering is
/// deterministic and fail-closed.
pub fn wrap_srcdoc(
    content_type: &ArtifactContentType,
    sanitized_content: &str,
) -> Result<String, ArtifactSandboxError> {
    let body = match content_type {
        ArtifactContentType::Html | ArtifactContentType::Svg => sanitized_content.to_string(),
        ArtifactContentType::PlainText => escape_html(sanitized_content),
        ArtifactContentType::Code { language } => {
            let escaped = escape_html(sanitized_content);
            let class_name = if language.trim().is_empty() {
                "code-block".to_string()
            } else {
                format!("code-block language-{}", slugify(language))
            };
            format!("<pre class=\"{class_name}\"><code>{escaped}</code></pre>")
        }
    };

    Ok(format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"{csp}\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><base href=\"about:srcdoc\"><title>Artifact preview</title><style>{style}</style></head><body>{body}</body></html>",
        csp = ARTIFACT_CSP,
        style = preview_styles(),
        body = body,
    ))
}

pub fn sanitize_and_wrap(
    content_type: &ArtifactContentType,
    raw: &str,
) -> Result<String, ArtifactSandboxError> {
    let sanitized = match content_type {
        ArtifactContentType::Html | ArtifactContentType::Svg => sanitize(raw),
        ArtifactContentType::PlainText | ArtifactContentType::Code { .. } => raw.to_string(),
    };
    wrap_srcdoc(content_type, &sanitized)
}

fn strip_script_blocks(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    loop {
        let Some(start) = find_case_insensitive(rest, "<script") else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let after_open = &rest[start..];
        let Some(tag_close) = after_open.find('>') else {
            break;
        };
        let after_tag = &after_open[tag_close + 1..];
        let Some(end_rel) = find_case_insensitive(after_tag, "</script>") else {
            break;
        };
        rest = &after_tag[end_rel + "</script>".len()..];
    }

    output
}

fn strip_inline_event_handlers(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find('<') {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            output.push_str(&rest[start..]);
            return output;
        };

        let tag = &rest[start..start + end + 1];
        output.push_str(&remove_event_attributes(tag));
        rest = &rest[start + end + 1..];
    }

    output.push_str(rest);
    output
}

fn remove_event_attributes(tag: &str) -> String {
    let mut result = String::with_capacity(tag.len());
    let mut chars = tag.chars().peekable();
    let mut in_quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        if let Some(quote) = in_quote {
            result.push(ch);
            if ch == quote {
                in_quote = None;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
            result.push(ch);
            continue;
        }

        if ch.is_whitespace() {
            result.push(ch);
            continue;
        }

        if ch.is_ascii_alphabetic() {
            let mut ident = String::new();
            ident.push(ch);
            while let Some(next) = chars.peek() {
                if next.is_ascii_alphanumeric() || *next == '-' || *next == ':' {
                    ident.push(*next);
                    chars.next();
                } else {
                    break;
                }
            }

            if ident.to_ascii_lowercase().starts_with("on") {
                skip_attribute_value(&mut chars);
                while result.ends_with(' ') {
                    result.pop();
                }
                if !result.ends_with('<') && !result.ends_with(' ') {
                    result.push(' ');
                }
                continue;
            }

            result.push_str(&ident);
            continue;
        }

        result.push(ch);
    }

    result
}

fn skip_attribute_value(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(next) = chars.peek() {
        if next.is_whitespace() {
            break;
        }
        if *next == '=' {
            chars.next();
            break;
        }
        if *next == '>' {
            return;
        }
        chars.next();
    }

    while let Some(next) = chars.peek() {
        if next.is_whitespace() {
            chars.next();
            continue;
        }
        if *next == '\'' || *next == '"' {
            let quote = *next;
            chars.next();
            while let Some(inner) = chars.next() {
                if inner == quote {
                    break;
                }
            }
        } else {
            while let Some(inner) = chars.peek() {
                if inner.is_whitespace() || *inner == '>' {
                    break;
                }
                chars.next();
            }
        }
        break;
    }
}

fn neutralize_javascript_uris(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(idx) = find_case_insensitive(rest, "javascript:") {
        output.push_str(&rest[..idx]);
        output.push_str("about:blank");
        rest = &rest[idx + "javascript:".len()..];
    }

    output.push_str(rest);
    output
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let lower_haystack = haystack.to_ascii_lowercase();
    lower_haystack.find(&needle.to_ascii_lowercase())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn preview_styles() -> &'static str {
    r#"
        :root {
            color-scheme: dark;
        }
        html, body {
            margin: 0;
            min-height: 100%;
            background: #0f0f0f;
            color: #e0e0e0;
            font-family: system-ui, -apple-system, sans-serif;
        }
        body {
            box-sizing: border-box;
            padding: 16px;
            line-height: 1.5;
            overflow-wrap: anywhere;
        }
        pre {
            margin: 0;
            white-space: pre-wrap;
            word-break: break-word;
            background: #1a1a1a;
            border: 1px solid #2a2a2a;
            border-radius: 8px;
            padding: 16px;
        }
        code {
            font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
            font-size: 13px;
        }
        .code-block {
            color: #e0e0e0;
        }
        svg {
            max-width: 100%;
            height: auto;
        }
    "#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_scripts_event_handlers_and_javascript_urls() {
        let raw = r#"<div onclick="alert(1)"><script>alert(1)</script><a href="javascript:alert(2)">go</a></div>"#;
        let sanitized = sanitize(raw);
        assert!(!sanitized.to_ascii_lowercase().contains("<script"));
        assert!(!sanitized.to_ascii_lowercase().contains("onclick"));
        assert!(!sanitized.to_ascii_lowercase().contains("javascript:"));
    }

    #[test]
    fn wrap_srcdoc_includes_csp_and_title() {
        let wrapped = wrap_srcdoc(&ArtifactContentType::Html, "<p>hi</p>").unwrap();
        assert!(wrapped.contains("Content-Security-Policy"));
        assert!(wrapped.contains("Artifact preview"));
        assert!(wrapped.contains("about:srcdoc"));
    }

    #[test]
    fn wrap_srcdoc_escapes_plain_text() {
        let wrapped = wrap_srcdoc(&ArtifactContentType::PlainText, "<tag>").unwrap();
        assert!(wrapped.contains("&lt;tag&gt;"));
        assert!(!wrapped.contains("<tag>"));
    }
}
