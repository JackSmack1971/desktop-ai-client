use crate::security::artifact_sandbox::{self, ArtifactSandboxError};
use crate::storage::sqlite::{SqlitePool, StorageError};
use rusqlite::params;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum ArtifactContentType {
    Html,
    Svg,
    PlainText,
    Code { language: String },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactPreview {
    pub artifact_id: String,
    pub content_type: ArtifactContentType,
    pub srcdoc: String,
}

#[derive(Debug, Clone)]
pub struct ArtifactRow {
    pub id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub content_type: ArtifactContentType,
    pub raw_source: String,
    pub created_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactStoreError {
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("artifact not found: {0}")]
    NotFound(String),
    #[error("sandbox error: {0}")]
    SandboxError(String),
}

impl From<ArtifactSandboxError> for ArtifactStoreError {
    fn from(value: ArtifactSandboxError) -> Self {
        ArtifactStoreError::SandboxError(value.to_string())
    }
}

pub struct ArtifactStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl ArtifactStore {
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub fn save_artifact(
        &self,
        id: &str,
        conversation_id: &str,
        message_id: Option<&str>,
        content_type: &ArtifactContentType,
        raw_source: &str,
    ) -> Result<(), ArtifactStoreError> {
        let (content_type_value, language_value) = db_content_type(content_type);
        self.pool
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO artifacts (id, conversation_id, message_id, content_type, language, raw_source)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        id,
                        conversation_id,
                        message_id,
                        content_type_value,
                        language_value,
                        raw_source
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| ArtifactStoreError::StorageError(e.to_string()))
    }

    pub fn get_artifact_row(&self, id: &str) -> Result<Option<ArtifactRow>, ArtifactStoreError> {
        self.pool
            .with_conn(|conn| {
                let result = conn.query_row(
                    "SELECT id, conversation_id, message_id, content_type, language, raw_source, created_at
                     FROM artifacts
                     WHERE id = ?1",
                    params![id],
                    |row| {
                        let content_type_value: String = row.get(3)?;
                        let language: String = row.get(4)?;
                        let content_type = db_content_type_to_enum(&content_type_value, &language)
                            .map_err(|message| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    3,
                                    rusqlite::types::Type::Text,
                                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message)),
                                )
                            })?;
                        Ok(ArtifactRow {
                            id: row.get(0)?,
                            conversation_id: row.get(1)?,
                            message_id: row.get(2)?,
                            content_type,
                            raw_source: row.get(5)?,
                            created_at: row.get(6)?,
                        })
                    },
                );

                match result {
                    Ok(row) => Ok(Some(row)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(StorageError(e)),
                }
            })
            .map_err(|e| ArtifactStoreError::StorageError(e.to_string()))
    }

    pub fn get_artifact_preview(&self, id: &str) -> Result<ArtifactPreview, ArtifactStoreError> {
        let row = self
            .get_artifact_row(id)?
            .ok_or_else(|| ArtifactStoreError::NotFound(id.to_string()))?;
        let srcdoc = artifact_sandbox::sanitize_and_wrap(&row.content_type, &row.raw_source)?;
        Ok(ArtifactPreview {
            artifact_id: row.id,
            content_type: row.content_type,
            srcdoc,
        })
    }

    pub fn detect_artifact(&self, text: &str) -> Option<DetectedArtifact> {
        detect_artifact(text)
    }
}

/// Map an `ArtifactContentType` to its `(content_type, language)` column values.
///
/// `pub(crate)` so `storage::turns::TurnStore` can insert an artifact row
/// inside the same atomic transaction that persists assistant output,
/// without duplicating this match in two places.
pub(crate) fn db_content_type(content_type: &ArtifactContentType) -> (&'static str, String) {
    match content_type {
        ArtifactContentType::Html => ("html", String::new()),
        ArtifactContentType::Svg => ("svg", String::new()),
        ArtifactContentType::PlainText => ("plain_text", String::new()),
        ArtifactContentType::Code { language } => ("code", language.clone()),
    }
}

fn db_content_type_to_enum(
    content_type: &str,
    language: &str,
) -> Result<ArtifactContentType, String> {
    match content_type {
        "html" => Ok(ArtifactContentType::Html),
        "svg" => Ok(ArtifactContentType::Svg),
        "plain_text" => Ok(ArtifactContentType::PlainText),
        "code" => Ok(ArtifactContentType::Code {
            language: language.to_string(),
        }),
        other => Err(format!("unknown artifact content_type: {other}")),
    }
}

#[derive(Debug, Clone)]
pub struct DetectedArtifact {
    pub content_type: ArtifactContentType,
    pub raw_source: String,
}

pub fn detect_artifact(text: &str) -> Option<DetectedArtifact> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((language, body)) = parse_fenced_code(trimmed) {
        let content_type = if language.is_empty() || language.eq_ignore_ascii_case("text") {
            ArtifactContentType::PlainText
        } else {
            ArtifactContentType::Code { language }
        };
        return Some(DetectedArtifact {
            content_type,
            raw_source: body,
        });
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("<svg") {
        return Some(DetectedArtifact {
            content_type: ArtifactContentType::Svg,
            raw_source: trimmed.to_string(),
        });
    }

    if lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || lower.contains("<body")
        || lower.contains("</html>")
    {
        return Some(DetectedArtifact {
            content_type: ArtifactContentType::Html,
            raw_source: trimmed.to_string(),
        });
    }

    None
}

fn parse_fenced_code(text: &str) -> Option<(String, String)> {
    let mut lines = text.lines();
    let first = lines.next()?.trim_start();
    if !first.starts_with("```") {
        return None;
    }

    let language = first.trim_start_matches("```").trim().to_string();
    let mut body = String::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            return Some((language, body.trim_end().to_string()));
        }
        body.push_str(line);
        body.push('\n');
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use rusqlite::Connection;
    use uuid::Uuid;

    fn pool() -> std::sync::Arc<SqlitePool> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        run_migrations(&conn, "0.0.0-test").unwrap();
        std::sync::Arc::new(SqlitePool::from_connection(conn))
    }

    #[test]
    fn detects_html_artifact() {
        let detected = detect_artifact("<html><body>Hello</body></html>").unwrap();
        assert!(matches!(detected.content_type, ArtifactContentType::Html));
    }

    #[test]
    fn detects_code_fence_artifact() {
        let detected = detect_artifact("```python\nprint('hi')\n```").unwrap();
        match detected.content_type {
            ArtifactContentType::Code { language } => assert_eq!(language, "python"),
            other => panic!("unexpected content type: {other:?}"),
        }
        assert!(detected.raw_source.contains("print('hi')"));
    }

    #[test]
    fn store_round_trips_preview() {
        let store = ArtifactStore::new(pool());
        let conversation_id = Uuid::new_v4().to_string();
        store
            .pool
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO conversations (id, title) VALUES (?1, 'Artifact Test')",
                    params![conversation_id],
                )?;
                Ok(())
            })
            .unwrap();
        let id = Uuid::new_v4().to_string();
        store
            .save_artifact(
                &id,
                &conversation_id,
                None,
                &ArtifactContentType::Html,
                "<div onclick=\"x()\">ok</div>",
            )
            .unwrap();

        let preview = store.get_artifact_preview(&id).unwrap();
        assert_eq!(preview.artifact_id, id);
        assert!(preview.srcdoc.contains("Content-Security-Policy"));
        assert!(!preview.srcdoc.to_ascii_lowercase().contains("onclick"));
    }
}
