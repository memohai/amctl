use crate::memory::MemoryStore;
use crate::output::{CommandError, CommandResult};
use serde_json::json;

fn require_store(store: Option<&MemoryStore>) -> Result<&MemoryStore, CommandError> {
    store.ok_or_else(|| {
        CommandError::invalid_params("memory commands require local memory; remove --no-memory")
    })
}

pub fn handle_save(
    memory_store: Option<&MemoryStore>,
    session: &str,
    app: &str,
    topic: &str,
    content: &str,
) -> CommandResult {
    let store = require_store(memory_store)?;
    if topic.trim().is_empty() {
        return Err(CommandError::invalid_params("topic must not be empty"));
    }
    if content.trim().is_empty() {
        return Err(CommandError::invalid_params("content must not be empty"));
    }
    let note = store
        .save_note(app, topic, content, session)
        .map_err(|e| CommandError::internal(format!("save note failed: {e}")))?;
    Ok(serde_json::to_value(note).unwrap_or_else(|_| json!({})))
}

pub fn handle_search(
    memory_store: Option<&MemoryStore>,
    app: Option<&str>,
    topic: Option<&str>,
    query: Option<&str>,
    limit: usize,
) -> CommandResult {
    let store = require_store(memory_store)?;
    let notes = store
        .search_notes(app, topic, query, limit)
        .map_err(|e| CommandError::internal(format!("search notes failed: {e}")))?;
    Ok(json!({ "notes": serde_json::to_value(&notes).unwrap_or_else(|_| json!([])) }))
}

pub fn handle_delete(memory_store: Option<&MemoryStore>, id: i64) -> CommandResult {
    let store = require_store(memory_store)?;
    let deleted = store
        .delete_note(id)
        .map_err(|e| CommandError::internal(format!("delete note failed: {e}")))?;
    Ok(json!({ "deleted": deleted, "id": id }))
}

pub fn handle_log(
    memory_store: Option<&MemoryStore>,
    session: Option<&str>,
    app: Option<&str>,
    status: Option<&str>,
    limit: usize,
) -> CommandResult {
    let store = require_store(memory_store)?;
    let events = store
        .query_events(session, app, status, limit)
        .map_err(|e| CommandError::internal(format!("query events failed: {e}")))?;
    Ok(json!({ "events": serde_json::to_value(&events).unwrap_or_else(|_| json!([])) }))
}

pub fn handle_stats(memory_store: Option<&MemoryStore>, session: Option<&str>) -> CommandResult {
    let store = require_store(memory_store)?;
    let stats = store
        .query_event_stats(session)
        .map_err(|e| CommandError::internal(format!("event stats failed: {e}")))?;
    Ok(serde_json::to_value(stats).unwrap_or_else(|_| json!({})))
}

pub fn handle_experience(
    memory_store: Option<&MemoryStore>,
    app: &str,
    activity: &str,
    page_fingerprint: &str,
    failure_cause: Option<&str>,
    limit: usize,
) -> CommandResult {
    let store = require_store(memory_store)?;
    let transitions = store
        .query_transitions(app, activity, page_fingerprint, limit)
        .map_err(|e| CommandError::internal(format!("query transitions failed: {e}")))?;
    let recoveries = store
        .query_recoveries(app, activity, page_fingerprint, failure_cause, limit)
        .map_err(|e| CommandError::internal(format!("query recoveries failed: {e}")))?;

    let transition_values: Vec<_> = transitions
        .iter()
        .map(|(scope, t)| {
            json!({
                "matchScope": scope,
                "preActivity": t.pre_activity,
                "preFingerprint": t.pre_page_fingerprint,
                "action": format!("{} {}", t.action_category, t.action_op),
                "actionArgs": t.action_args_json,
                "postActivity": t.post_activity,
                "postFingerprint": t.post_page_fingerprint,
                "verifyOp": t.verify_op,
                "successCount": t.success_count,
                "verifiedCount": t.verified_count,
                "failureCount": t.failure_count,
                "lastSuccessAt": t.last_success_at,
            })
        })
        .collect();

    let recovery_values: Vec<_> = recoveries
        .iter()
        .map(|(scope, r)| {
            json!({
                "matchScope": scope,
                "preActivity": r.pre_activity,
                "preFingerprint": r.pre_page_fingerprint,
                "failureCause": r.failure_cause,
                "recovery": format!("{} {}", r.recovery_category, r.recovery_op),
                "recoveryArgs": r.recovery_args_json,
                "successCount": r.success_count,
                "failureCount": r.failure_count,
                "lastSuccessAt": r.last_success_at,
            })
        })
        .collect();

    Ok(json!({
        "query": {
            "app": app,
            "activity": activity,
            "pageFingerprint": page_fingerprint,
            "failureCause": failure_cause,
        },
        "transitions": transition_values,
        "recoveries": recovery_values,
    }))
}

pub fn handle_context(memory_store: Option<&MemoryStore>, session: &str) -> CommandResult {
    let store = require_store(memory_store)?;
    let ctx = store
        .get_session_state(session)
        .map_err(|e| CommandError::internal(format!("get session state failed: {e}")))?;
    match ctx {
        Some(c) => Ok(json!({
            "session": session,
            "app": c.app,
            "activity": c.activity,
            "pageFingerprint": c.page_fingerprint,
            "fingerprintSource": c.fingerprint_source,
            "mode": c.mode,
            "hasWebView": c.has_webview,
            "nodeReliability": c.node_reliability,
            "refVersion": c.ref_version,
            "observedAt": c.observed_at,
        })),
        None => Ok(json!({ "session": session, "found": false })),
    }
}
