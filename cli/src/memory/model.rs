use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PageContext {
    pub app: String,
    pub activity: String,
    pub page_fingerprint: String,
    pub fingerprint_source: String,
    pub mode: String,
    pub has_webview: bool,
    pub node_reliability: String,
    pub ref_version: Option<u64>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub id: i64,
    pub app: String,
    pub topic: String,
    pub content: String,
    pub session: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: i64,
    pub created_at: String,
    pub session: String,
    pub app: String,
    pub activity: String,
    pub page_fingerprint: String,
    pub category: String,
    pub op: String,
    pub args_json: String,
    pub status: String,
    pub error_code: Option<String>,
    pub failure_cause: Option<String>,
    pub evidence_json: String,
    pub duration_ms: i64,
}

pub struct EventRecord<'a> {
    pub session: &'a str,
    pub app: &'a str,
    pub activity: &'a str,
    pub page_fingerprint: &'a str,
    pub category: &'a str,
    pub op: &'a str,
    pub args_json: &'a str,
    pub status: &'a str,
    pub error_code: Option<&'a str>,
    pub failure_cause: Option<&'a str>,
    pub evidence_json: &'a str,
    pub duration_ms: i64,
}

pub struct ArtifactRecord<'a> {
    pub session: &'a str,
    pub trace_id: &'a str,
    pub category: &'a str,
    pub op: &'a str,
    pub kind: &'a str,
    pub mime_type: &'a str,
    pub file_path: &'a std::path::Path,
    pub size_bytes: i64,
    pub content_hash: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventStats {
    pub session: Option<String>,
    pub total_events: i64,
    pub ok_count: i64,
    pub failed_count: i64,
    pub act_count: i64,
    pub verify_count: i64,
    pub recover_count: i64,
    pub apps_touched: Vec<String>,
    pub duration_total_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transition {
    pub pre_app: String,
    pub pre_activity: String,
    pub pre_page_fingerprint: String,
    pub action_category: String,
    pub action_op: String,
    pub action_args_json: String,
    pub post_app: String,
    pub post_activity: String,
    pub post_page_fingerprint: String,
    pub verify_op: String,
    pub verify_args_json: String,
    pub success_count: i64,
    pub verified_count: i64,
    pub failure_count: i64,
    pub last_success_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Recovery {
    pub pre_app: String,
    pub pre_activity: String,
    pub pre_page_fingerprint: String,
    pub failure_cause: String,
    pub recovery_category: String,
    pub recovery_op: String,
    pub recovery_args_json: String,
    pub success_count: i64,
    pub failure_count: i64,
    pub last_success_at: Option<String>,
}
