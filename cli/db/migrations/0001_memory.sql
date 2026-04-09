-- Session observation cache (last observed state, not ground truth)
CREATE TABLE IF NOT EXISTS session_state (
    session TEXT PRIMARY KEY,
    app TEXT NOT NULL DEFAULT '',
    activity TEXT NOT NULL DEFAULT '',
    page_fingerprint TEXT NOT NULL DEFAULT '',
    fingerprint_source TEXT NOT NULL DEFAULT '',
    mode TEXT NOT NULL DEFAULT '',
    has_webview INTEGER NOT NULL DEFAULT 0,
    node_reliability TEXT NOT NULL DEFAULT '',
    ref_version INTEGER,
    observed_at TEXT NOT NULL DEFAULT ''
);

-- Structured event log (act / verify / recover only)
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    session TEXT NOT NULL,
    app TEXT NOT NULL DEFAULT '',
    activity TEXT NOT NULL DEFAULT '',
    page_fingerprint TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    op TEXT NOT NULL,
    args_json TEXT NOT NULL,
    status TEXT NOT NULL,
    error_code TEXT,
    failure_cause TEXT,
    evidence_json TEXT NOT NULL DEFAULT '{}',
    duration_ms INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_events_session ON events(session, created_at);
CREATE INDEX IF NOT EXISTS idx_events_app ON events(app, created_at);
CREATE INDEX IF NOT EXISTS idx_events_status ON events(status, created_at);

-- Verified action-outcome transitions (auto-tracked)
CREATE TABLE IF NOT EXISTS transitions (
    pre_app TEXT NOT NULL,
    pre_activity TEXT NOT NULL,
    pre_page_fingerprint TEXT NOT NULL DEFAULT '',
    action_category TEXT NOT NULL,
    action_op TEXT NOT NULL,
    action_args_json TEXT NOT NULL,
    post_app TEXT NOT NULL DEFAULT '',
    post_activity TEXT NOT NULL DEFAULT '',
    post_page_fingerprint TEXT NOT NULL DEFAULT '',
    verify_op TEXT NOT NULL DEFAULT '',
    verify_args_json TEXT NOT NULL DEFAULT '',
    success_count INTEGER NOT NULL DEFAULT 0,
    verified_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_success_at TEXT,
    last_session TEXT,
    last_action_event_id INTEGER,
    last_verify_event_id INTEGER,
    PRIMARY KEY (
        pre_app, pre_activity, pre_page_fingerprint,
        action_category, action_op, action_args_json,
        verify_op, verify_args_json
    )
);

CREATE INDEX IF NOT EXISTS idx_transitions_page
    ON transitions(pre_app, pre_activity, pre_page_fingerprint);

-- Recovery strategies linked to failure causes (auto-tracked)
CREATE TABLE IF NOT EXISTS recoveries (
    pre_app TEXT NOT NULL,
    pre_activity TEXT NOT NULL,
    pre_page_fingerprint TEXT NOT NULL DEFAULT '',
    failure_cause TEXT NOT NULL,
    recovery_category TEXT NOT NULL,
    recovery_op TEXT NOT NULL,
    recovery_args_json TEXT NOT NULL,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_success_at TEXT,
    last_session TEXT,
    last_event_id INTEGER,
    PRIMARY KEY (
        pre_app, pre_activity, pre_page_fingerprint,
        failure_cause, recovery_category, recovery_op, recovery_args_json
    )
);

CREATE INDEX IF NOT EXISTS idx_recoveries_page
    ON recoveries(pre_app, pre_activity, pre_page_fingerprint, failure_cause);

-- Agent-driven knowledge notes (append-only)
CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    app TEXT NOT NULL DEFAULT '',
    topic TEXT NOT NULL,
    content TEXT NOT NULL,
    session TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_notes_app ON notes(app);
CREATE INDEX IF NOT EXISTS idx_notes_topic ON notes(app, topic);

-- Persisted large outputs and saved files
CREATE TABLE IF NOT EXISTS artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    session TEXT NOT NULL DEFAULT '',
    trace_id TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    op TEXT NOT NULL,
    kind TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content_hash TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_artifacts_session ON artifacts(session, created_at);
CREATE INDEX IF NOT EXISTS idx_artifacts_trace ON artifacts(trace_id);
