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

/// A single message for inclusion in a `ConversationDetail` response.
///
/// Maps from `MessageRow` fields. Content stays backend-mapped — only these
/// fields cross the Tauri IPC boundary.
#[derive(Debug, Clone, serde::Serialize)]
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
pub struct ConversationDetail {
    pub id: String,
    pub title: String,
    pub model: String,
    pub status: String,
    pub updated_at: String,
    pub messages: Vec<MessageSummary>,
}

/// Enforce that history commands can only be invoked from the main window.
///
/// Backend-side enforcement; the capability file is defense-in-depth.
/// Returns `HistoryError::UnauthorizedWindow` if the label is not "main".
fn assert_main_window(window: &tauri::Window) -> Result<(), HistoryError> {
    if window.label() != "main" {
        return Err(HistoryError::UnauthorizedWindow(format!(
            "history commands require the main window, got {:?}",
            window.label()
        )));
    }
    Ok(())
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
    assert_main_window(&window)?;
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
    assert_main_window(&window)?;

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
    assert_main_window(&window)?;
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
    assert_main_window(&window)?;
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
}
