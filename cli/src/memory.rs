use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, named_params, params};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[cfg(test)]
use crate::db::open_in_memory_connection;
use crate::db::open_memory_connection;

// ── Core types ──

#[derive(Debug)]
pub struct MemoryStore {
    connection: Connection,
}

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

// ── Page fingerprint ──

const ANCHOR_CLASSES: &[&str] = &[
    "RecyclerView",
    "ListView",
    "ViewPager",
    "TabLayout",
    "Toolbar",
    "EditText",
    "WebView",
    "FrameLayout",
    "DialogTitle",
];

/// Rows from screen or refs used for fingerprint computation.
pub struct FingerprintRow<'a> {
    pub class_name: Option<&'a str>,
    pub res_id: Option<&'a str>,
}

/// Build a stable, human-readable page fingerprint.
///
/// Format: `act=<activity>|mode=<mode>|wv=<0|1>|rid=<anchors>|cls=<classes>`
pub fn build_page_fingerprint(
    activity: &str,
    mode: &str,
    has_webview: bool,
    rows: &[FingerprintRow<'_>],
) -> String {
    let mut rid_anchors: BTreeSet<String> = BTreeSet::new();
    let mut cls_anchors: BTreeSet<String> = BTreeSet::new();

    for row in rows {
        if let Some(res_id) = row.res_id {
            let short = shorten_res_id(res_id);
            if !short.is_empty() {
                rid_anchors.insert(short);
            }
        }
        if let Some(class_name) = row.class_name {
            let short_class = shorten_class_name(class_name);
            if is_anchor_class(&short_class) {
                cls_anchors.insert(short_class);
            }
        }
    }

    let rid_anchors: Vec<_> = rid_anchors.into_iter().take(8).collect();
    let cls_anchors: Vec<_> = cls_anchors.into_iter().take(6).collect();

    let mut parts = Vec::with_capacity(5);
    parts.push(format!("act={activity}"));
    if !mode.is_empty() {
        parts.push(format!("mode={mode}"));
    }
    parts.push(format!("wv={}", if has_webview { 1 } else { 0 }));
    if !rid_anchors.is_empty() {
        parts.push(format!("rid={}", rid_anchors.join(",")));
    }
    if !cls_anchors.is_empty() {
        parts.push(format!("cls={}", cls_anchors.join(",")));
    }
    parts.join("|")
}

fn shorten_res_id(res_id: &str) -> String {
    let id = res_id
        .rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(res_id);
    if id.is_empty()
        || id == "content"
        || id == "statusBarBackground"
        || id == "navigationBarBackground"
        || id == "action_bar_root"
    {
        return String::new();
    }
    id.to_string()
}

fn shorten_class_name(class_name: &str) -> String {
    class_name
        .rsplit_once('.')
        .map(|(_, name)| name)
        .unwrap_or(class_name)
        .to_string()
}

fn is_anchor_class(short_name: &str) -> bool {
    ANCHOR_CLASSES
        .iter()
        .any(|anchor| short_name.contains(anchor))
}

// ── MemoryStore ──

impl MemoryStore {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let connection = open_memory_connection(path.as_path())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .with_context(|| "failed to set sqlite busy timeout")?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .with_context(|| "failed to enable sqlite WAL mode")?;
        Ok(Self { connection })
    }

    // ── session_state ──

    pub fn update_session_state(&self, session: &str, ctx: &PageContext) -> anyhow::Result<()> {
        self.connection
            .execute(
                "INSERT INTO session_state (
                    session, app, activity, page_fingerprint, fingerprint_source,
                    mode, has_webview, node_reliability, ref_version, observed_at
                 ) VALUES (
                    :session, :app, :activity, :fp, :fp_source,
                    :mode, :wv, :nr, :rv, :observed_at
                 )
                 ON CONFLICT(session) DO UPDATE SET
                    app = excluded.app,
                    activity = excluded.activity,
                    page_fingerprint = excluded.page_fingerprint,
                    fingerprint_source = excluded.fingerprint_source,
                    mode = excluded.mode,
                    has_webview = excluded.has_webview,
                    node_reliability = excluded.node_reliability,
                    ref_version = excluded.ref_version,
                    observed_at = excluded.observed_at",
                named_params! {
                    ":session": session,
                    ":app": ctx.app,
                    ":activity": ctx.activity,
                    ":fp": ctx.page_fingerprint,
                    ":fp_source": ctx.fingerprint_source,
                    ":mode": ctx.mode,
                    ":wv": ctx.has_webview as i64,
                    ":nr": ctx.node_reliability,
                    ":rv": ctx.ref_version.map(|v| v as i64),
                    ":observed_at": ctx.observed_at,
                },
            )
            .with_context(|| "failed to upsert session_state")?;
        Ok(())
    }

    /// Update only app + activity (from observe top). Does not touch fingerprint.
    pub fn update_session_activity(
        &self,
        session: &str,
        app: &str,
        activity: &str,
        observed_at: &str,
    ) -> anyhow::Result<()> {
        let existing = self.get_session_state(session)?;
        if existing.is_some() {
            self.connection
                .execute(
                    "UPDATE session_state SET app = ?1, activity = ?2, observed_at = ?3
                     WHERE session = ?4",
                    params![app, activity, observed_at, session],
                )
                .with_context(|| "failed to update session activity")?;
        } else {
            self.connection
                .execute(
                    "INSERT INTO session_state (session, app, activity, observed_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![session, app, activity, observed_at],
                )
                .with_context(|| "failed to insert session activity")?;
        }
        Ok(())
    }

    pub fn get_session_state(&self, session: &str) -> anyhow::Result<Option<PageContext>> {
        self.connection
            .query_row(
                "SELECT app, activity, page_fingerprint, fingerprint_source,
                        mode, has_webview, node_reliability, ref_version, observed_at
                 FROM session_state WHERE session = ?1",
                params![session],
                |row| {
                    Ok(PageContext {
                        app: row.get(0)?,
                        activity: row.get(1)?,
                        page_fingerprint: row.get(2)?,
                        fingerprint_source: row.get(3)?,
                        mode: row.get(4)?,
                        has_webview: row.get::<_, i64>(5)? != 0,
                        node_reliability: row.get(6)?,
                        ref_version: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                        observed_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .with_context(|| "failed to read session_state")
    }

    // ── events ──

    pub fn record_event(&self, event: &EventRecord<'_>) -> anyhow::Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        self.connection
            .execute(
                "INSERT INTO events (
                    created_at, session, app, activity, page_fingerprint,
                    category, op, args_json, status, error_code,
                    failure_cause, evidence_json, duration_ms
                 ) VALUES (
                    :ts, :session, :app, :activity, :fp,
                    :category, :op, :args, :status, :err,
                    :cause, :evidence, :dur
                 )",
                named_params! {
                    ":ts": now,
                    ":session": event.session,
                    ":app": event.app,
                    ":activity": event.activity,
                    ":fp": event.page_fingerprint,
                    ":category": event.category,
                    ":op": event.op,
                    ":args": event.args_json,
                    ":status": event.status,
                    ":err": event.error_code,
                    ":cause": event.failure_cause,
                    ":evidence": event.evidence_json,
                    ":dur": event.duration_ms,
                },
            )
            .with_context(|| "failed to insert event")?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn get_event_by_id(&self, id: i64) -> anyhow::Result<Option<Event>> {
        self.connection
            .query_row(
                "SELECT id, created_at, session, app, activity, page_fingerprint,
                        category, op, args_json, status, error_code,
                        failure_cause, evidence_json, duration_ms
                 FROM events WHERE id = ?1",
                params![id],
                read_event_row,
            )
            .optional()
            .with_context(|| format!("failed to get event {id}"))
    }

    pub fn query_events(
        &self,
        session: Option<&str>,
        app: Option<&str>,
        status: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<Event>> {
        let mut sql = String::from(
            "SELECT id, created_at, session, app, activity, page_fingerprint,
                    category, op, args_json, status, error_code,
                    failure_cause, evidence_json, duration_ms
             FROM events WHERE 1=1",
        );
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 0usize;
        if let Some(v) = session {
            idx += 1;
            sql.push_str(&format!(" AND session = ?{idx}"));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(v) = app {
            idx += 1;
            sql.push_str(&format!(" AND app = ?{idx}"));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(v) = status {
            idx += 1;
            sql.push_str(&format!(" AND status = ?{idx}"));
            bind_values.push(Box::new(v.to_string()));
        }
        sql.push_str(&format!(" ORDER BY created_at DESC, id DESC LIMIT {limit}"));
        let refs: Vec<&dyn rusqlite::types::ToSql> =
            bind_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(Event {
                id: row.get(0)?,
                created_at: row.get(1)?,
                session: row.get(2)?,
                app: row.get(3)?,
                activity: row.get(4)?,
                page_fingerprint: row.get(5)?,
                category: row.get(6)?,
                op: row.get(7)?,
                args_json: row.get(8)?,
                status: row.get(9)?,
                error_code: row.get(10)?,
                failure_cause: row.get(11)?,
                evidence_json: row.get(12)?,
                duration_ms: row.get(13)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .with_context(|| "failed to collect events")
    }

    pub fn query_event_stats(&self, session: Option<&str>) -> anyhow::Result<EventStats> {
        let (wc, bind): (&str, Option<&str>) = if let Some(s) = session {
            ("WHERE session = ?1", Some(s))
        } else {
            ("", None)
        };
        let scalar = |sql: &str| -> anyhow::Result<i64> {
            if let Some(s) = bind {
                self.connection.query_row(sql, params![s], |r| r.get(0))
            } else {
                self.connection.query_row(sql, [], |r| r.get(0))
            }
            .with_context(|| format!("stats query failed: {sql}"))
        };
        let joiner = if wc.is_empty() { "WHERE" } else { "AND" };
        let total = scalar(&format!("SELECT COUNT(*) FROM events {wc}"))?;
        let ok = scalar(&format!(
            "SELECT COUNT(*) FROM events {wc} {joiner} status = 'ok'"
        ))?;
        let cat = |c: &str| -> anyhow::Result<i64> {
            scalar(&format!(
                "SELECT COUNT(*) FROM events {wc} {joiner} category = '{c}'"
            ))
        };
        let dur = scalar(&format!(
            "SELECT COALESCE(SUM(duration_ms), 0) FROM events {wc}"
        ))?;
        let apps = {
            let sql =
                format!("SELECT DISTINCT app FROM events {wc} {joiner} app != '' ORDER BY app");
            let mut stmt = self.connection.prepare(&sql)?;
            let read_app = |row: &rusqlite::Row<'_>| row.get::<_, String>(0);
            let rows = if let Some(s) = bind {
                stmt.query_map(params![s], read_app)?
            } else {
                stmt.query_map([], read_app)?
            };
            rows.collect::<Result<Vec<_>, _>>()?
        };
        Ok(EventStats {
            session: session.map(str::to_string),
            total_events: total,
            ok_count: ok,
            failed_count: total - ok,
            act_count: cat("act")?,
            verify_count: cat("verify")?,
            recover_count: cat("recover")?,
            apps_touched: apps,
            duration_total_ms: dur,
        })
    }

    /// Find the most recent act/recover event in the session before the given event id.
    /// Find the most recent act event (not recover) in the session.
    /// Transitions are defined as act → verify; recover events produce
    /// recovery records instead.
    /// Find the most recent unconsumed act event before `before_id`.
    ///
    /// An act is "consumed" if a verify or recover event exists between it and
    /// `before_id`, meaning a previous verify/recover already closed it.
    pub fn previous_action_event(
        &self,
        session: &str,
        before_id: i64,
    ) -> anyhow::Result<Option<Event>> {
        self.connection
            .query_row(
                "SELECT id, created_at, session, app, activity, page_fingerprint,
                        category, op, args_json, status, error_code,
                        failure_cause, evidence_json, duration_ms
                 FROM events
                 WHERE session = ?1 AND id < ?2 AND category = 'act'
                   AND NOT EXISTS (
                       SELECT 1 FROM events e2
                       WHERE e2.session = ?1
                         AND e2.id > events.id AND e2.id < ?2
                         AND e2.category IN ('verify', 'recover')
                   )
                 ORDER BY id DESC LIMIT 1",
                params![session, before_id],
                read_event_row,
            )
            .optional()
            .with_context(|| "failed to query previous action event")
    }

    /// Find the most recent unresolved failed act/verify event in the session.
    ///
    /// A failure is "resolved" if a successful act or verify exists between
    /// it and `before_id`, meaning the agent moved past the failure on its own.
    pub fn previous_failed_event(
        &self,
        session: &str,
        before_id: i64,
    ) -> anyhow::Result<Option<Event>> {
        self.connection
            .query_row(
                "SELECT id, created_at, session, app, activity, page_fingerprint,
                        category, op, args_json, status, error_code,
                        failure_cause, evidence_json, duration_ms
                 FROM events
                 WHERE session = ?1 AND id < ?2 AND status = 'failed'
                       AND category IN ('act', 'verify')
                   AND NOT EXISTS (
                       SELECT 1 FROM events e2
                       WHERE e2.session = ?1
                         AND e2.id > events.id AND e2.id < ?2
                         AND e2.status = 'ok'
                         AND e2.category IN ('act', 'verify')
                   )
                 ORDER BY id DESC LIMIT 1",
                params![session, before_id],
                read_event_row,
            )
            .optional()
            .with_context(|| "failed to query previous failed event")
    }

    /// Invalidate page fingerprint after act/recover. Clears fingerprint and
    /// fingerprint_source. Activity is preserved; observe screen/refs now
    /// provide their own fresh topActivity from the server.
    pub fn invalidate_session_page(&self, session: &str) -> anyhow::Result<()> {
        self.connection
            .execute(
                "UPDATE session_state SET
                    page_fingerprint = '',
                    fingerprint_source = ''
                 WHERE session = ?1",
                params![session],
            )
            .with_context(|| "failed to invalidate session page")?;
        Ok(())
    }

    // ── transitions ──

    pub fn upsert_transition(
        &self,
        pre: &PageContext,
        action_event: &Event,
        post: &PageContext,
        verify_event: &Event,
        verified: bool,
    ) -> anyhow::Result<()> {
        let success = if action_event.status == "ok" { 1 } else { 0 };
        let verified_val = if verified { 1 } else { 0 };
        let failure = if verified { 0 } else { 1 };
        self.connection
            .execute(
                "INSERT INTO transitions (
                    pre_app, pre_activity, pre_page_fingerprint,
                    action_category, action_op, action_args_json,
                    post_app, post_activity, post_page_fingerprint,
                    verify_op, verify_args_json,
                    success_count, verified_count, failure_count,
                    last_success_at, last_session, last_action_event_id, last_verify_event_id
                 ) VALUES (
                    :pa, :pact, :pfp,
                    :ac, :ao, :aa,
                    :poa, :poact, :pofp,
                    :vo, :va,
                    :sc, :vc, :fc,
                    :lsa, :ls, :laei, :lvei
                 )
                 ON CONFLICT(
                    pre_app, pre_activity, pre_page_fingerprint,
                    action_category, action_op, action_args_json,
                    verify_op, verify_args_json
                 ) DO UPDATE SET
                    post_app = excluded.post_app,
                    post_activity = excluded.post_activity,
                    post_page_fingerprint = excluded.post_page_fingerprint,
                    success_count = transitions.success_count + excluded.success_count,
                    verified_count = transitions.verified_count + excluded.verified_count,
                    failure_count = transitions.failure_count + excluded.failure_count,
                    last_success_at = COALESCE(excluded.last_success_at, transitions.last_success_at),
                    last_session = COALESCE(excluded.last_session, transitions.last_session),
                    last_action_event_id = COALESCE(excluded.last_action_event_id, transitions.last_action_event_id),
                    last_verify_event_id = COALESCE(excluded.last_verify_event_id, transitions.last_verify_event_id)",
                named_params! {
                    ":pa": pre.app,
                    ":pact": pre.activity,
                    ":pfp": pre.page_fingerprint,
                    ":ac": action_event.category,
                    ":ao": action_event.op,
                    ":aa": action_event.args_json,
                    ":poa": post.app,
                    ":poact": post.activity,
                    ":pofp": post.page_fingerprint,
                    ":vo": verify_event.op,
                    ":va": verify_event.args_json,
                    ":sc": success,
                    ":vc": verified_val,
                    ":fc": failure,
                    ":lsa": if verified { Some(&verify_event.created_at as &str) } else { None },
                    ":ls": if verified { Some(&verify_event.session as &str) } else { None },
                    ":laei": if verified { Some(action_event.id) } else { None },
                    ":lvei": if verified { Some(verify_event.id) } else { None },
                },
            )
            .with_context(|| "failed to upsert transition")?;
        Ok(())
    }

    /// Three-tier transition query: page → activity → app.
    pub fn query_transitions(
        &self,
        app: &str,
        activity: &str,
        page_fingerprint: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, Transition)>> {
        let mut out = Vec::new();
        self.query_transitions_scope(
            &mut out,
            "page",
            "pre_app = ?1 AND pre_activity = ?2 AND pre_page_fingerprint = ?3",
            params![app, activity, page_fingerprint],
            limit,
        )?;
        let remaining = limit.saturating_sub(out.len());
        if remaining > 0 {
            self.query_transitions_scope(
                &mut out,
                "activity",
                "pre_app = ?1 AND pre_activity = ?2 AND pre_page_fingerprint <> ?3",
                params![app, activity, page_fingerprint],
                remaining,
            )?;
        }
        let remaining = limit.saturating_sub(out.len());
        if remaining > 0 {
            self.query_transitions_scope(
                &mut out,
                "app",
                "pre_app = ?1 AND pre_activity <> ?2",
                params![app, activity],
                remaining,
            )?;
        }
        Ok(out)
    }

    fn query_transitions_scope<P: rusqlite::Params>(
        &self,
        out: &mut Vec<(String, Transition)>,
        scope: &str,
        where_sql: &str,
        params: P,
        limit: usize,
    ) -> anyhow::Result<()> {
        let sql = format!(
            "SELECT pre_app, pre_activity, pre_page_fingerprint,
                    action_category, action_op, action_args_json,
                    post_app, post_activity, post_page_fingerprint,
                    verify_op, verify_args_json,
                    success_count, verified_count, failure_count, last_success_at
             FROM transitions WHERE {where_sql}
             ORDER BY verified_count DESC, failure_count ASC, last_success_at DESC
             LIMIT {limit}"
        );
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt.query_map(params, |row| {
            Ok(Transition {
                pre_app: row.get(0)?,
                pre_activity: row.get(1)?,
                pre_page_fingerprint: row.get(2)?,
                action_category: row.get(3)?,
                action_op: row.get(4)?,
                action_args_json: row.get(5)?,
                post_app: row.get(6)?,
                post_activity: row.get(7)?,
                post_page_fingerprint: row.get(8)?,
                verify_op: row.get(9)?,
                verify_args_json: row.get(10)?,
                success_count: row.get(11)?,
                verified_count: row.get(12)?,
                failure_count: row.get(13)?,
                last_success_at: row.get(14)?,
            })
        })?;
        for row in rows {
            out.push((scope.to_string(), row?));
        }
        Ok(())
    }

    // ── recoveries ──

    pub fn upsert_recovery(
        &self,
        pre: &PageContext,
        failure_cause: &str,
        recovery_event: &Event,
    ) -> anyhow::Result<()> {
        let success = if recovery_event.status == "ok" { 1 } else { 0 };
        let failure = if recovery_event.status == "ok" { 0 } else { 1 };
        self.connection
            .execute(
                "INSERT INTO recoveries (
                    pre_app, pre_activity, pre_page_fingerprint,
                    failure_cause, recovery_category, recovery_op, recovery_args_json,
                    success_count, failure_count, last_success_at,
                    last_session, last_event_id
                 ) VALUES (
                    :pa, :pact, :pfp,
                    :fc, :rc, :ro, :ra,
                    :sc, :flc, :lsa,
                    :ls, :lei
                 )
                 ON CONFLICT(
                    pre_app, pre_activity, pre_page_fingerprint,
                    failure_cause, recovery_category, recovery_op, recovery_args_json
                 ) DO UPDATE SET
                    success_count = recoveries.success_count + excluded.success_count,
                    failure_count = recoveries.failure_count + excluded.failure_count,
                    last_success_at = COALESCE(excluded.last_success_at, recoveries.last_success_at),
                    last_session = COALESCE(excluded.last_session, recoveries.last_session),
                    last_event_id = COALESCE(excluded.last_event_id, recoveries.last_event_id)",
                named_params! {
                    ":pa": pre.app,
                    ":pact": pre.activity,
                    ":pfp": pre.page_fingerprint,
                    ":fc": failure_cause,
                    ":rc": recovery_event.category,
                    ":ro": recovery_event.op,
                    ":ra": recovery_event.args_json,
                    ":sc": success,
                    ":flc": failure,
                    ":lsa": if success > 0 { Some(&recovery_event.created_at as &str) } else { None },
                    ":ls": if success > 0 { Some(&recovery_event.session as &str) } else { None },
                    ":lei": if success > 0 { Some(recovery_event.id) } else { None },
                },
            )
            .with_context(|| "failed to upsert recovery")?;
        Ok(())
    }

    /// Three-tier recovery query: page → activity → app.
    pub fn query_recoveries(
        &self,
        app: &str,
        activity: &str,
        page_fingerprint: &str,
        failure_cause: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, Recovery)>> {
        let mut out = Vec::new();
        let fc_clause = if failure_cause.is_some() {
            " AND failure_cause = :fc"
        } else {
            ""
        };

        let query_scope = |out: &mut Vec<(String, Recovery)>,
                           conn: &Connection,
                           scope: &str,
                           base_where: &str,
                           limit: usize|
         -> anyhow::Result<()> {
            let sql = format!(
                "SELECT pre_app, pre_activity, pre_page_fingerprint,
                        failure_cause, recovery_category, recovery_op, recovery_args_json,
                        success_count, failure_count, last_success_at
                 FROM recoveries WHERE {base_where}{fc_clause}
                 ORDER BY success_count DESC, failure_count ASC
                 LIMIT {limit}"
            );
            let mut stmt = conn.prepare(&sql)?;
            let read_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<Recovery> {
                Ok(Recovery {
                    pre_app: row.get(0)?,
                    pre_activity: row.get(1)?,
                    pre_page_fingerprint: row.get(2)?,
                    failure_cause: row.get(3)?,
                    recovery_category: row.get(4)?,
                    recovery_op: row.get(5)?,
                    recovery_args_json: row.get(6)?,
                    success_count: row.get(7)?,
                    failure_count: row.get(8)?,
                    last_success_at: row.get(9)?,
                })
            };
            let rows = if let Some(fc) = failure_cause {
                match scope {
                    "page" => stmt.query_map(
                        named_params! {":a": app, ":act": activity, ":fp": page_fingerprint, ":fc": fc},
                        read_row,
                    )?,
                    "activity" => stmt.query_map(
                        named_params! {":a": app, ":act": activity, ":fp": page_fingerprint, ":fc": fc},
                        read_row,
                    )?,
                    _ => stmt.query_map(named_params! {":a": app, ":act": activity, ":fc": fc}, read_row)?,
                }
            } else {
                match scope {
                    "page" => stmt.query_map(
                        named_params! {":a": app, ":act": activity, ":fp": page_fingerprint},
                        read_row,
                    )?,
                    "activity" => stmt.query_map(
                        named_params! {":a": app, ":act": activity, ":fp": page_fingerprint},
                        read_row,
                    )?,
                    _ => stmt.query_map(named_params! {":a": app, ":act": activity}, read_row)?,
                }
            };
            for row in rows {
                out.push((scope.to_string(), row?));
            }
            Ok(())
        };

        query_scope(
            &mut out,
            &self.connection,
            "page",
            "pre_app = :a AND pre_activity = :act AND pre_page_fingerprint = :fp",
            limit,
        )?;
        let remaining = limit.saturating_sub(out.len());
        if remaining > 0 {
            query_scope(
                &mut out,
                &self.connection,
                "activity",
                "pre_app = :a AND pre_activity = :act AND pre_page_fingerprint <> :fp",
                remaining,
            )?;
        }
        let remaining = limit.saturating_sub(out.len());
        if remaining > 0 {
            query_scope(
                &mut out,
                &self.connection,
                "app",
                "pre_app = :a AND pre_activity <> :act",
                remaining,
            )?;
        }
        Ok(out)
    }

    // ── notes (append-only) ──

    pub fn save_note(
        &self,
        app: &str,
        topic: &str,
        content: &str,
        session: &str,
    ) -> anyhow::Result<Note> {
        let now = chrono::Utc::now().to_rfc3339();
        self.connection
            .execute(
                "INSERT INTO notes (created_at, app, topic, content, session)
                 VALUES (:ts, :app, :topic, :content, :session)",
                named_params! {
                    ":ts": now,
                    ":app": app,
                    ":topic": topic,
                    ":content": content,
                    ":session": session,
                },
            )
            .with_context(|| "failed to insert note")?;
        let id = self.connection.last_insert_rowid();
        Ok(Note {
            id,
            app: app.to_string(),
            topic: topic.to_string(),
            content: content.to_string(),
            session: session.to_string(),
            created_at: now,
        })
    }

    pub fn search_notes(
        &self,
        app: Option<&str>,
        topic_prefix: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<Note>> {
        let mut sql = String::from(
            "SELECT id, app, topic, content, session, created_at FROM notes WHERE 1=1",
        );
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 0usize;
        if let Some(v) = app {
            idx += 1;
            sql.push_str(&format!(" AND app = ?{idx}"));
            bind_values.push(Box::new(v.to_string()));
        }
        if let Some(prefix) = topic_prefix {
            idx += 1;
            sql.push_str(&format!(" AND topic LIKE ?{idx}"));
            bind_values.push(Box::new(format!("{prefix}%")));
        }
        if let Some(q) = query {
            idx += 1;
            sql.push_str(&format!(" AND (content LIKE ?{idx} OR topic LIKE ?{idx})"));
            bind_values.push(Box::new(format!("%{q}%")));
        }
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));
        let refs: Vec<&dyn rusqlite::types::ToSql> =
            bind_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), read_note_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .with_context(|| "failed to collect notes")
    }

    pub fn delete_note(&self, id: i64) -> anyhow::Result<bool> {
        let affected = self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", params![id])
            .with_context(|| "failed to delete note")?;
        Ok(affected > 0)
    }
}

// ── helpers ──

fn read_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        created_at: row.get(1)?,
        session: row.get(2)?,
        app: row.get(3)?,
        activity: row.get(4)?,
        page_fingerprint: row.get(5)?,
        category: row.get(6)?,
        op: row.get(7)?,
        args_json: row.get(8)?,
        status: row.get(9)?,
        error_code: row.get(10)?,
        failure_cause: row.get(11)?,
        evidence_json: row.get(12)?,
        duration_ms: row.get(13)?,
    })
}

fn read_note_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        app: row.get(1)?,
        topic: row.get(2)?,
        content: row.get(3)?,
        session: row.get(4)?,
        created_at: row.get(5)?,
    })
}

pub fn package_name_from_activity(top_activity: &str) -> String {
    let trimmed = top_activity.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some((pkg, _)) = trimmed.split_once('/') {
        return pkg.trim().to_string();
    }
    trimmed.to_string()
}

#[cfg(test)]
impl MemoryStore {
    pub(crate) fn new_in_memory() -> anyhow::Result<Self> {
        let connection = open_in_memory_connection()?;
        Ok(Self { connection })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_fingerprint_basic() {
        let rows = vec![
            FingerprintRow {
                class_name: Some("android.widget.FrameLayout"),
                res_id: Some("com.android.settings:id/settings_main_pane"),
            },
            FingerprintRow {
                class_name: Some("androidx.recyclerview.widget.RecyclerView"),
                res_id: Some("com.android.settings:id/recycler_view"),
            },
            FingerprintRow {
                class_name: Some("android.widget.TextView"),
                res_id: None,
            },
        ];
        let fp =
            build_page_fingerprint("com.android.settings/.Settings", "SYSTEM_API", false, &rows);
        assert!(fp.contains("act=com.android.settings/.Settings"));
        assert!(fp.contains("mode=SYSTEM_API"));
        assert!(fp.contains("wv=0"));
        assert!(fp.contains("rid="));
        assert!(fp.contains("settings_main_pane"));
        assert!(fp.contains("recycler_view"));
        assert!(fp.contains("cls="));
        assert!(fp.contains("RecyclerView"));
    }

    #[test]
    fn page_fingerprint_stable_across_calls() {
        let rows = vec![FingerprintRow {
            class_name: Some("android.widget.TextView"),
            res_id: Some("com.example:id/title"),
        }];
        let a = build_page_fingerprint("com.example/.Main", "SYSTEM_API", false, &rows);
        let b = build_page_fingerprint("com.example/.Main", "SYSTEM_API", false, &rows);
        assert_eq!(a, b);
    }

    #[test]
    fn page_fingerprint_ignores_text_content() {
        let rows_a = vec![FingerprintRow {
            class_name: Some("android.widget.TextView"),
            res_id: Some("com.example:id/title"),
        }];
        let rows_b = vec![FingerprintRow {
            class_name: Some("android.widget.TextView"),
            res_id: Some("com.example:id/title"),
        }];
        let a = build_page_fingerprint("com.example/.Main", "SYSTEM_API", false, &rows_a);
        let b = build_page_fingerprint("com.example/.Main", "SYSTEM_API", false, &rows_b);
        assert_eq!(a, b);
    }

    #[test]
    fn page_fingerprint_without_class_still_uses_res_id() {
        let rows = vec![
            FingerprintRow {
                class_name: None,
                res_id: Some("com.example:id/main_pane"),
            },
            FingerprintRow {
                class_name: None,
                res_id: Some("com.example:id/toolbar"),
            },
        ];
        let fp = build_page_fingerprint("com.example/.Main", "SYSTEM_API", false, &rows);
        assert!(
            fp.contains("rid=main_pane,toolbar"),
            "res_id anchors should be present even without class: {fp}"
        );
        assert!(!fp.contains("cls="), "no cls anchors when class is absent");
    }

    #[test]
    fn session_state_round_trip() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: Some(42),
            observed_at: "2026-04-03T10:00:00Z".into(),
        };
        store.update_session_state("s1", &ctx).expect("upsert");
        let got = store
            .get_session_state("s1")
            .expect("get")
            .expect("should exist");
        assert_eq!(got.page_fingerprint, "act=com.a/.Main|wv=0");
        assert_eq!(got.fingerprint_source, "screen");
        assert_eq!(got.ref_version, Some(42));
    }

    #[test]
    fn update_session_activity_preserves_fingerprint() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0|rid=toolbar".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "2026-04-03T10:00:00Z".into(),
        };
        store.update_session_state("s1", &ctx).expect("full update");

        store
            .update_session_activity("s1", "com.a", "com.a/.Sub", "2026-04-03T10:01:00Z")
            .expect("activity update");
        let got = store.get_session_state("s1").expect("get").expect("exists");
        assert_eq!(got.activity, "com.a/.Sub");
        assert_eq!(
            got.page_fingerprint, "act=com.a/.Main|wv=0|rid=toolbar",
            "fingerprint should be preserved"
        );
    }

    #[test]
    fn notes_are_append_only() {
        let store = MemoryStore::new_in_memory().expect("init");
        store
            .save_note("com.a", "nav/wifi", "path A", "s1")
            .expect("save");
        store
            .save_note("com.a", "nav/wifi", "path B", "s2")
            .expect("save");
        let notes = store
            .search_notes(Some("com.a"), Some("nav/wifi"), None, 10)
            .expect("search");
        assert_eq!(notes.len(), 2, "both notes should exist");
    }

    #[test]
    fn transition_upsert_and_three_tier_query() {
        let store = MemoryStore::new_in_memory().expect("init");
        let pre = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0|rid=list".into(),
            fingerprint_source: "screen".into(),
            mode: "".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "".into(),
        };
        let post = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Detail".into(),
            page_fingerprint: "act=com.a/.Detail|wv=0|rid=detail_pane".into(),
            fingerprint_source: "screen".into(),
            mode: "".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "".into(),
        };
        let action = Event {
            id: 1,
            created_at: "2026-04-03T10:00:00Z".into(),
            session: "s1".into(),
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: pre.page_fingerprint.clone(),
            category: "act".into(),
            op: "tap".into(),
            args_json: r#"{"by":"text","value":"Item"}"#.into(),
            status: "ok".into(),
            error_code: None,
            failure_cause: None,
            evidence_json: "{}".into(),
            duration_ms: 10,
        };
        let verify = Event {
            id: 2,
            created_at: "2026-04-03T10:00:01Z".into(),
            session: "s1".into(),
            app: "com.a".into(),
            activity: "com.a/.Detail".into(),
            page_fingerprint: post.page_fingerprint.clone(),
            category: "verify".into(),
            op: "text-contains".into(),
            args_json: r#"{"text":"Detail"}"#.into(),
            status: "ok".into(),
            error_code: None,
            failure_cause: None,
            evidence_json: r#"{"matched":true}"#.into(),
            duration_ms: 5,
        };
        store
            .upsert_transition(&pre, &action, &post, &verify, true)
            .expect("upsert");

        let results = store
            .query_transitions("com.a", "com.a/.Main", &pre.page_fingerprint, 10)
            .expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "page");
        assert_eq!(results[0].1.verified_count, 1);
    }

    #[test]
    fn recovery_upsert_and_query() {
        let store = MemoryStore::new_in_memory().expect("init");
        let pre = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0".into(),
            fingerprint_source: "screen".into(),
            mode: "".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "".into(),
        };
        let recovery_event = Event {
            id: 3,
            created_at: "2026-04-03T10:00:02Z".into(),
            session: "s1".into(),
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: pre.page_fingerprint.clone(),
            category: "recover".into(),
            op: "back".into(),
            args_json: r#"{"times":1}"#.into(),
            status: "ok".into(),
            error_code: None,
            failure_cause: None,
            evidence_json: "{}".into(),
            duration_ms: 5,
        };
        store
            .upsert_recovery(&pre, "REF_ALIAS_STALE", &recovery_event)
            .expect("upsert");

        let results = store
            .query_recoveries(
                "com.a",
                "com.a/.Main",
                &pre.page_fingerprint,
                Some("REF_ALIAS_STALE"),
                10,
            )
            .expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "page");
        assert_eq!(results[0].1.success_count, 1);
    }

    #[test]
    fn invalidate_session_page_clears_fingerprint_keeps_activity() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0|rid=list".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: Some(5),
            observed_at: "2026-01-01T00:00:00Z".into(),
        };
        store.update_session_state("s1", &ctx).expect("setup");
        store.invalidate_session_page("s1").expect("invalidate");

        let got = store
            .get_session_state("s1")
            .expect("get")
            .expect("should still exist");
        assert_eq!(got.activity, "com.a/.Main", "activity should be preserved");
        assert!(
            got.page_fingerprint.is_empty(),
            "fingerprint should be cleared"
        );
        assert!(
            got.fingerprint_source.is_empty(),
            "source should be cleared"
        );
        assert_eq!(got.app, "com.a", "app should be preserved");
    }

    fn test_event<'a>(cat: &'a str, status: &'a str, cause: Option<&'a str>) -> EventRecord<'a> {
        EventRecord {
            session: "s1",
            app: "com.a",
            activity: "com.a/.Main",
            page_fingerprint: "",
            category: cat,
            op: "test",
            args_json: "{}",
            status,
            error_code: None,
            failure_cause: cause,
            evidence_json: "{}",
            duration_ms: 1,
        }
    }

    #[test]
    fn previous_failed_event_skips_failed_recovers() {
        let store = MemoryStore::new_in_memory().expect("init");

        let _ = store
            .record_event(&test_event("act", "failed", Some("ELEMENT_NOT_FOUND")))
            .expect("e1");
        let _ = store
            .record_event(&test_event("recover", "failed", None))
            .expect("e2");
        let e3_id = store
            .record_event(&test_event("recover", "ok", None))
            .expect("e3");

        let found = store
            .previous_failed_event("s1", e3_id)
            .expect("query")
            .expect("should find original failure");
        assert_eq!(
            found.category, "act",
            "should find the act failure, not recover"
        );
        assert_eq!(found.failure_cause.as_deref(), Some("ELEMENT_NOT_FOUND"));
    }

    #[test]
    fn previous_failed_event_resolved_by_success() {
        let store = MemoryStore::new_in_memory().expect("init");

        let _ = store
            .record_event(&test_event("act", "failed", Some("ELEMENT_NOT_FOUND")))
            .expect("e1");
        let _ = store
            .record_event(&test_event("act", "ok", None))
            .expect("e2");
        let recover_id = store
            .record_event(&test_event("recover", "ok", None))
            .expect("e3");

        let found = store
            .previous_failed_event("s1", recover_id)
            .expect("query");
        assert!(
            found.is_none(),
            "failure resolved by subsequent success should not be returned"
        );
    }

    #[test]
    fn previous_action_event_consumed_by_recover() {
        let store = MemoryStore::new_in_memory().expect("init");

        let _ = store
            .record_event(&test_event("act", "ok", None))
            .expect("act");
        let _ = store
            .record_event(&test_event("recover", "ok", None))
            .expect("recover");
        let verify_id = store
            .record_event(&test_event("verify", "ok", None))
            .expect("verify");

        let found = store.previous_action_event("s1", verify_id).expect("query");
        assert!(
            found.is_none(),
            "act consumed by intervening recover should not be returned"
        );
    }

    #[test]
    fn previous_action_event_consumed_by_verify() {
        let store = MemoryStore::new_in_memory().expect("init");

        let _ = store
            .record_event(&test_event("act", "ok", None))
            .expect("act");
        let _ = store
            .record_event(&test_event("verify", "ok", None))
            .expect("verify1");
        let verify2_id = store
            .record_event(&test_event("verify", "ok", None))
            .expect("verify2");

        let found = store
            .previous_action_event("s1", verify2_id)
            .expect("query");
        assert!(
            found.is_none(),
            "act consumed by first verify should not be found by second verify"
        );
    }

    #[test]
    fn previous_action_event_unconsumed() {
        let store = MemoryStore::new_in_memory().expect("init");

        let act_id = store
            .record_event(&test_event("act", "ok", None))
            .expect("act");
        let verify_id = store
            .record_event(&test_event("verify", "ok", None))
            .expect("verify");

        let found = store
            .previous_action_event("s1", verify_id)
            .expect("query")
            .expect("should find unconsumed act");
        assert_eq!(found.id, act_id);
    }

    #[test]
    fn package_name_extraction() {
        assert_eq!(
            package_name_from_activity("com.android.settings/.Settings"),
            "com.android.settings"
        );
        assert_eq!(package_name_from_activity("com.x"), "com.x");
        assert_eq!(package_name_from_activity(""), "");
    }
}
