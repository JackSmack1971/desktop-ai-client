/// IPC commands for conversation history.
///
/// Command inventory entries:
///   history_list   — windows: ["main"], production: true, sensitivity: HIGH (contains user prompt content)
///   history_get    — windows: ["main"], production: true, sensitivity: HIGH (contains user prompt content)
///   history_search — windows: ["main"], production: true, sensitivity: HIGH (contains user prompt content)
///   history_delete — windows: ["main"], production: true, sensitivity: HIGH (contains user prompt content)
///
/// All commands assert the caller is the main window. Typed domain stores
/// handle all DB access — no raw SQL in this file.
use crate::security::command_policy;
use crate::storage::fts::FtsStore;
use crate::storage::retention::RetentionStore;
use crate::storage::sqlite::{ConversationStore, MessageStore};

/// Error type returned to the frontend from history IPC commands.
///
/// Variants are serialized as structured error objects:
/// `{ "code": "SCREAMING_SNAKE_CASE", "message": "..." }`
/// This matches the established IPC error shape from `ShellError` and `ChatError`.
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HistoryError {
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
}

impl From<command_policy::PolicyError> for HistoryError {
    fn from(value: command_policy::PolicyError) -> Self {
        match value {
            command_policy::PolicyError::UnauthorizedWindow(msg) => {
                HistoryError::UnauthorizedWindow(msg)
            }
            command_policy::PolicyError::UnknownCommand(msg) => {
                HistoryError::UnauthorizedWindow(msg)
            }
        }
    }
}

/// A single message for inclusion in a `ConversationDetail` response.
///
/// Maps from `MessageRow` fields. Content stays backend-mapped — only these
/// fields cross the Tauri IPC boundary. `camelCase` per the backend rule for
/// DTOs read by TypeScript.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSummary {
    pub id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

/// Summary metadata for a conversation, used in list and search responses.
///
/// The `snippet` field is populated only for search results (from FTS5
/// `snippet()` auxiliary function). It is omitted from list responses.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub model: String,
    pub status: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// Full conversation record with message list, returned by `history_get`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationDetail {
    pub id: String,
    pub title: String,
    pub model: String,
    pub status: String,
    pub updated_at: String,
    pub messages: Vec<MessageSummary>,
}

/// Return a list of all conversations ordered by most-recently-updated first.
///
/// Used to populate the history surface on load. The `snippet` field is
/// absent from each summary — snippets are only present in search results.
#[tauri::command]
pub async fn history_list(
    window: tauri::Window,
    store: tauri::State<'_, ConversationStore>,
) -> Result<Vec<ConversationSummary>, HistoryError> {
    command_policy::policy_check("history_list", window.label())?;
    store
        .list_conversations()
        .map_err(|e| HistoryError::StorageError(e.to_string()))
        .map(|rows| {
            rows.into_iter()
                .map(|r| ConversationSummary {
                    id: r.id,
                    title: r.title,
                    model: r.model,
                    status: r.status,
                    updated_at: r.updated_at,
                    snippet: None,
                })
                .collect()
        })
}

/// Return a full conversation record with its complete message list.
///
/// Returns `HistoryError::NotFound` when the conversation `id` does not exist.
/// Returns `HistoryError::StorageError` on any underlying SQLite failure.
#[tauri::command]
pub async fn history_get(
    window: tauri::Window,
    id: String,
    conv_store: tauri::State<'_, ConversationStore>,
    msg_store: tauri::State<'_, MessageStore>,
) -> Result<ConversationDetail, HistoryError> {
    command_policy::policy_check("history_get", window.label())?;

    let conv = conv_store
        .get_conversation(&id)
        .map_err(|e| HistoryError::StorageError(e.to_string()))?
        .ok_or_else(|| HistoryError::NotFound(id.clone()))?;

    let messages = msg_store
        .get_messages(&id)
        .map_err(|e| HistoryError::StorageError(e.to_string()))?;

    let message_summaries = messages
        .into_iter()
        .map(|m| MessageSummary {
            id: m.id,
            role: m.role,
            content: m.content,
            status: m.status,
            created_at: m.created_at,
        })
        .collect();

    Ok(ConversationDetail {
        id: conv.id,
        title: conv.title,
        model: conv.model,
        status: conv.status,
        updated_at: conv.updated_at,
        messages: message_summaries,
    })
}

/// Hard-delete a conversation and all its messages.
///
/// Delegates to `RetentionStore::delete_conversation`, which runs the WAL
/// checkpoint after deletion. Deletion is idempotent — returns `Ok(())` when
/// the conversation does not exist.
#[tauri::command]
pub async fn history_delete(
    window: tauri::Window,
    id: String,
    store: tauri::State<'_, RetentionStore>,
) -> Result<(), HistoryError> {
    command_policy::policy_check("history_delete", window.label())?;
    store
        .delete_conversation(&id)
        .map_err(|e| HistoryError::StorageError(e.to_string()))
}

/// Search conversation message content using FTS5 MATCH.
///
/// Returns up to 50 conversations whose messages match `query`, ordered by
/// relevance (FTS5 rank). Each summary includes a highlighted `snippet`
/// field with `<b>` / `</b>` markers around matching terms.
///
/// Returns an empty list when no conversations match.
#[tauri::command]
pub async fn history_search(
    window: tauri::Window,
    query: String,
    store: tauri::State<'_, FtsStore>,
) -> Result<Vec<ConversationSummary>, HistoryError> {
    command_policy::policy_check("history_search", window.label())?;
    store
        .search(&query)
        .map_err(|e| HistoryError::StorageError(e.to_string()))
        .map(|results| {
            results
                .into_iter()
                .map(|r| ConversationSummary {
                    id: r.id,
                    title: r.title,
                    model: r.model,
                    status: r.status,
                    updated_at: r.updated_at,
                    snippet: Some(r.snippet),
                })
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_error_serializes_storage_error() {
        let err = HistoryError::StorageError("disk full".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("STORAGE_ERROR"),
            "expected STORAGE_ERROR code in JSON: {json}"
        );
    }

    #[test]
    fn history_error_serializes_not_found() {
        let err = HistoryError::NotFound("conv-123".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("NOT_FOUND"),
            "expected NOT_FOUND code in JSON: {json}"
        );
    }

    #[test]
    fn history_error_serializes_unauthorized_window() {
        let err = HistoryError::UnauthorizedWindow("devtools".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("UNAUTHORIZED_WINDOW"),
            "expected UNAUTHORIZED_WINDOW code in JSON: {json}"
        );
    }

    #[test]
    fn conversation_summary_serializes_camel_case_fields() {
        let summary = ConversationSummary {
            id: "c1".into(),
            title: "t".into(),
            model: "m".into(),
            status: "active".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
            snippet: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(
            json.contains(r#""updatedAt":"2024-01-01T00:00:00Z""#),
            "expected camelCase updatedAt, got: {json}"
        );
        assert!(!json.contains("updated_at"), "must not leak snake_case key: {json}");
    }

    #[test]
    fn message_summary_serializes_camel_case_fields() {
        let summary = MessageSummary {
            id: "m1".into(),
            role: "user".into(),
            content: "hi".into(),
            status: "complete".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(
            json.contains(r#""createdAt":"2024-01-01T00:00:00Z""#),
            "expected camelCase createdAt, got: {json}"
        );
    }

    #[test]
    fn policy_check_rejects_non_main_window_for_history_commands() {
        for command in ["history_list", "history_get", "history_delete", "history_search"] {
            let err: HistoryError = command_policy::policy_check(command, "evil")
                .unwrap_err()
                .into();
            assert!(
                matches!(err, HistoryError::UnauthorizedWindow(_)),
                "command {command} did not reject non-main window"
            );
        }
    }
}
