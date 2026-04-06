use crate::cli::{ActCommands, Cli, Commands, ObserveCommands, RecoverCommands, VerifyCommands};
use crate::memory::{
    EventRecord, FingerprintRow, MemoryStore, PageContext, build_page_fingerprint,
    package_name_from_activity,
};
use serde_json::{Value, json};

/// Whether this command should be recorded as an event.
pub fn should_record_event(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Act { .. } | Commands::Verify { .. } | Commands::Recover { .. }
    )
}

/// Whether this command can update session observation cache.
pub fn should_update_session_cache(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Observe {
            command: ObserveCommands::Top
                | ObserveCommands::Screen { .. }
                | ObserveCommands::Refs { .. }
                | ObserveCommands::Page { .. },
            ..
        }
    )
}

/// Update session observation cache from an observe result.
///
/// Quality-based overwrite rules:
/// - `observe top`:    only updates app + activity; never clears existing fingerprint
/// - `observe screen`: strongest fingerprint, always overwrites
/// - `observe refs`:   generates fingerprint, overwrites only if current source != "screen"
///   or fingerprint is empty
pub fn update_session_cache(
    store: &MemoryStore,
    session: &str,
    command: &Commands,
    result: &Value,
) {
    if result.get("status").and_then(Value::as_str) != Some("ok") {
        return;
    }
    let data = match result.get("data") {
        Some(d) => d,
        None => result,
    };
    let now = chrono::Utc::now().to_rfc3339();

    match command {
        Commands::Observe {
            command: ObserveCommands::Top,
            ..
        } => {
            // observe top returns: { "topActivity": "pkg/act" }
            if let Some(activity) = data.get("topActivity").and_then(Value::as_str) {
                let app = package_name_from_activity(activity);
                if let Err(e) = store.update_session_activity(session, &app, activity, &now) {
                    eprintln!("warn: session_state update failed: {e}");
                }
            }
        }
        Commands::Observe {
            command: ObserveCommands::Screen { .. },
            ..
        } => {
            let mode = data.get("mode").and_then(Value::as_str).unwrap_or("");
            let has_webview = data
                .get("hasWebView")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let node_reliability = data
                .get("nodeReliability")
                .and_then(Value::as_str)
                .unwrap_or("");

            let existing = store.get_session_state(session).ok().flatten();
            let activity = data
                .get("topActivity")
                .and_then(Value::as_str)
                .or_else(|| existing.as_ref().map(|c| c.activity.as_str()));
            let Some(activity) = activity else {
                return;
            };
            let app = package_name_from_activity(activity);

            let raw_rows = data.get("rows").and_then(Value::as_array);
            let (page_fingerprint, fingerprint_source) = if let Some(raw_rows) = raw_rows {
                let rows = extract_fingerprint_rows_from_screen(raw_rows);
                (
                    build_page_fingerprint(activity, mode, has_webview, &rows),
                    "screen".to_string(),
                )
            } else if let Some(ctx) = &existing {
                (ctx.page_fingerprint.clone(), ctx.fingerprint_source.clone())
            } else {
                return;
            };

            let ctx = PageContext {
                app,
                activity: activity.to_string(),
                page_fingerprint,
                fingerprint_source,
                mode: mode.into(),
                has_webview,
                node_reliability: node_reliability.into(),
                ref_version: existing.as_ref().and_then(|c| c.ref_version),
                observed_at: now,
            };
            if let Err(e) = store.update_session_state(session, &ctx) {
                eprintln!("warn: session_state update (screen) failed: {e}");
            }
        }
        Commands::Observe {
            command: ObserveCommands::Refs { .. },
            ..
        } => {
            let mode = data.get("mode").and_then(Value::as_str).unwrap_or("");
            let has_webview = data
                .get("hasWebView")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let node_reliability = data
                .get("nodeReliability")
                .and_then(Value::as_str)
                .unwrap_or("");
            let ref_version = data.get("refVersion").and_then(Value::as_u64);

            let existing = store.get_session_state(session).ok().flatten();
            let explicit_activity = data
                .get("topActivity")
                .and_then(Value::as_str)
                .map(str::to_string);
            let activity_owned = explicit_activity.clone().or_else(|| {
                existing
                    .as_ref()
                    .map(|c| c.activity.clone())
                    .filter(|activity| !activity.is_empty())
            });
            let Some(activity_owned) = activity_owned else {
                return;
            };
            let app = package_name_from_activity(&activity_owned);
            let activity_changed = existing
                .as_ref()
                .map(|ctx| ctx.activity != activity_owned)
                .unwrap_or(false);
            let should_overwrite = match &existing {
                None => true,
                Some(ctx) => {
                    activity_changed
                        || ctx.fingerprint_source != "screen"
                        || ctx.page_fingerprint.is_empty()
                }
            };

            if should_overwrite {
                let empty = vec![];
                let raw_rows = data.get("rows").and_then(Value::as_array).unwrap_or(&empty);
                let rows = extract_fingerprint_rows_from_refs(raw_rows);
                let fp = build_page_fingerprint(&activity_owned, mode, has_webview, &rows);

                let ctx = PageContext {
                    app,
                    activity: activity_owned,
                    page_fingerprint: fp,
                    fingerprint_source: "refs".into(),
                    mode: mode.into(),
                    has_webview,
                    node_reliability: node_reliability.into(),
                    ref_version,
                    observed_at: now,
                };
                if let Err(e) = store.update_session_state(session, &ctx) {
                    eprintln!("warn: session_state update (refs) failed: {e}");
                }
            } else if let Some(mut ctx) = existing {
                ctx.activity = activity_owned;
                ctx.app = app;
                ctx.ref_version = ref_version;
                if explicit_activity.is_some() {
                    ctx.observed_at = now;
                }
                if let Err(e) = store.update_session_state(session, &ctx) {
                    eprintln!("warn: session_state ref_version update failed: {e}");
                }
            }
        }
        Commands::Observe {
            command: ObserveCommands::Page { .. },
            ..
        } => {
            let activity = data
                .get("topActivity")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty());
            let Some(activity) = activity else {
                return;
            };
            let app = package_name_from_activity(activity);

            let mode_str = data.get("mode").and_then(Value::as_str).unwrap_or("");
            let has_webview = data
                .get("hasWebView")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let node_reliability = data
                .get("nodeReliability")
                .and_then(Value::as_str)
                .unwrap_or("");

            let ref_version = data
                .get("refs")
                .and_then(|r| r.get("refVersion"))
                .and_then(Value::as_u64);

            // Build fingerprint: prefer screen rows (strongest), fall back to refs rows
            let screen_rows = data
                .get("screen")
                .and_then(|s| s.get("rows"))
                .and_then(Value::as_array);
            let refs_rows = data
                .get("refs")
                .and_then(|r| r.get("rows"))
                .and_then(Value::as_array);

            let (fp, source) = if let Some(rows) = screen_rows {
                let fr = extract_fingerprint_rows_from_screen(rows);
                (
                    build_page_fingerprint(activity, mode_str, has_webview, &fr),
                    "screen",
                )
            } else if let Some(rows) = refs_rows {
                let fr = extract_fingerprint_rows_from_refs(rows);
                (
                    build_page_fingerprint(activity, mode_str, has_webview, &fr),
                    "refs",
                )
            } else {
                (String::new(), "top")
            };

            let ctx = PageContext {
                app,
                activity: activity.to_string(),
                page_fingerprint: fp,
                fingerprint_source: source.into(),
                mode: mode_str.into(),
                has_webview,
                node_reliability: node_reliability.into(),
                ref_version,
                observed_at: now,
            };
            if let Err(e) = store.update_session_state(session, &ctx) {
                eprintln!("warn: session_state update (page) failed: {e}");
            }
        }
        _ => {}
    }
}

/// Record an event for act/verify/recover and handle deferred transition/recovery closing.
pub fn record_event_and_close(
    store: &MemoryStore,
    cli: &Cli,
    _invocation_id: &str,
    result: &Value,
    duration_ms: u128,
) {
    let (app, activity, page_fingerprint) = match store.get_session_state(&cli.session) {
        Ok(Some(ctx)) => (
            ctx.app.clone(),
            ctx.activity.clone(),
            ctx.page_fingerprint.clone(),
        ),
        _ => (String::new(), String::new(), String::new()),
    };

    let shape = describe_command(&cli.command);
    let status = result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let error_code = result
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(Value::as_str);
    let failure_cause_owned = extract_failure_cause(result);
    let failure_cause = failure_cause_owned.as_deref();
    let evidence_json = build_evidence_json(&cli.command, result);

    let event_id = match store.record_event(&EventRecord {
        session: &cli.session,
        app: &app,
        activity: &activity,
        page_fingerprint: &page_fingerprint,
        category: &shape.category,
        op: &shape.op,
        args_json: &shape.args_json,
        status,
        error_code,
        failure_cause,
        evidence_json: &evidence_json,
        duration_ms: duration_ms as i64,
    }) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("warn: failed to record event: {e}");
            return;
        }
    };

    match &cli.command {
        Commands::Act { .. } | Commands::Recover { .. } => {
            if let Err(e) = store.invalidate_session_page(&cli.session) {
                eprintln!("warn: failed to invalidate session page: {e}");
            }
        }
        _ => {}
    }

    match &cli.command {
        Commands::Verify { .. } => {
            try_close_transition(store, &cli.session, event_id, status == "ok");
        }
        Commands::Recover { .. } => {
            try_record_recovery(store, &cli.session, event_id, status);
        }
        _ => {}
    }
}

/// After a verify event, look back to find the preceding act → close transition.
///
/// Safety: only closes if session_state was refreshed (observe) after the action.
/// If no fresh observation exists, the post-context would be stale and we skip.
fn try_close_transition(store: &MemoryStore, session: &str, verify_event_id: i64, verified: bool) {
    let verify_event = match store.get_event_by_id(verify_event_id) {
        Ok(Some(ev)) => ev,
        _ => return,
    };

    let action_event = match store.previous_action_event(session, verify_event_id) {
        Ok(Some(ev)) => ev,
        _ => return,
    };

    let session_state = match store.get_session_state(session) {
        Ok(Some(ctx)) => ctx,
        _ => return,
    };

    if session_state.observed_at <= action_event.created_at {
        return;
    }

    let pre_ctx = PageContext {
        app: action_event.app.clone(),
        activity: action_event.activity.clone(),
        page_fingerprint: action_event.page_fingerprint.clone(),
        fingerprint_source: String::new(),
        mode: String::new(),
        has_webview: false,
        node_reliability: String::new(),
        ref_version: None,
        observed_at: String::new(),
    };

    let post_ctx = PageContext {
        app: verify_event.app.clone(),
        activity: verify_event.activity.clone(),
        page_fingerprint: verify_event.page_fingerprint.clone(),
        fingerprint_source: String::new(),
        mode: String::new(),
        has_webview: false,
        node_reliability: String::new(),
        ref_version: None,
        observed_at: String::new(),
    };

    if let Err(e) =
        store.upsert_transition(&pre_ctx, &action_event, &post_ctx, &verify_event, verified)
    {
        eprintln!("warn: transition upsert failed: {e}");
    }
}

/// After a recover event, look back for the preceding failure and record recovery.
fn try_record_recovery(store: &MemoryStore, session: &str, recover_event_id: i64, status: &str) {
    let recovery_event = match store.get_event_by_id(recover_event_id) {
        Ok(Some(ev)) => ev,
        _ => return,
    };

    let failed_event = match store.previous_failed_event(session, recover_event_id) {
        Ok(Some(ev)) => ev,
        _ => return,
    };

    let cause = failed_event
        .failure_cause
        .as_deref()
        .or(failed_event.error_code.as_deref())
        .unwrap_or("unknown");

    let pre_ctx = PageContext {
        app: failed_event.app.clone(),
        activity: failed_event.activity.clone(),
        page_fingerprint: failed_event.page_fingerprint.clone(),
        fingerprint_source: String::new(),
        mode: String::new(),
        has_webview: false,
        node_reliability: String::new(),
        ref_version: None,
        observed_at: String::new(),
    };

    let mut recovery_with_status = recovery_event;
    recovery_with_status.status = if status == "ok" {
        "ok".into()
    } else {
        "failed".into()
    };

    if let Err(e) = store.upsert_recovery(&pre_ctx, cause, &recovery_with_status) {
        eprintln!("warn: recovery upsert failed: {e}");
    }
}

/// Extract a structured failure cause from the error output.
///
/// Tries these sources in order:
/// 1. `error.message` prefix before `:` (e.g. "REF_ALIAS_STALE: ref=@n5" → "REF_ALIAS_STALE")
/// 2. `error.code` (e.g. "SERVER_ERROR", "ASSERTION_FAILED")
fn extract_failure_cause(result: &Value) -> Option<String> {
    let error = result.get("error")?;
    if let Some(msg) = error.get("message").and_then(Value::as_str) {
        if let Some(prefix) = msg.split(':').next() {
            let trimmed = prefix.trim();
            if trimmed.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    error
        .get("code")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Build evidence JSON capturing execution details for later analysis.
fn build_evidence_json(command: &Commands, result: &Value) -> String {
    let error_obj = result.get("error").cloned();
    match command {
        Commands::Verify { command: vc, .. } => {
            let data = result.get("data").cloned().unwrap_or(json!({}));
            match vc {
                VerifyCommands::TextContains { text, .. } => {
                    json!({"type": "text-contains", "expected": text, "data": data, "error": error_obj}).to_string()
                }
                VerifyCommands::TopActivity { expected, mode } => {
                    json!({"type": "top-activity", "expected": expected, "mode": mode, "data": data, "error": error_obj}).to_string()
                }
                VerifyCommands::NodeExists { by, value, exact_match } => {
                    json!({"type": "node-exists", "by": by, "value": value, "exactMatch": exact_match, "data": data, "error": error_obj}).to_string()
                }
            }
        }
        Commands::Act { .. } | Commands::Recover { .. } => {
            if let Some(err) = error_obj {
                let data = result.get("data").cloned();
                json!({"error": err, "data": data}).to_string()
            } else {
                "{}".into()
            }
        }
        _ => "{}".into(),
    }
}

fn extract_fingerprint_rows_from_screen<'a>(rows: &'a [Value]) -> Vec<FingerprintRow<'a>> {
    rows.iter()
        .filter_map(|row| {
            let class_name = row
                .get("class_name")
                .or_else(|| row.get("class"))
                .and_then(Value::as_str);
            let res_id = row
                .get("res_id")
                .or_else(|| row.get("resId"))
                .and_then(Value::as_str);
            if class_name.is_none() && res_id.is_none() {
                return None;
            }
            Some(FingerprintRow { class_name, res_id })
        })
        .collect()
}

fn extract_fingerprint_rows_from_refs<'a>(rows: &'a [Value]) -> Vec<FingerprintRow<'a>> {
    rows.iter()
        .filter_map(|row| {
            let class_name = row.get("class").and_then(Value::as_str);
            let res_id = row.get("resId").and_then(Value::as_str);
            if class_name.is_none() && res_id.is_none() {
                return None;
            }
            Some(FingerprintRow { class_name, res_id })
        })
        .collect()
}

struct CommandShape {
    category: String,
    op: String,
    args_json: String,
}

fn describe_command(command: &Commands) -> CommandShape {
    match command {
        Commands::Act { command, .. } => match command {
            ActCommands::Tap {
                xy,
                by,
                value,
                exact_match,
            } => CommandShape {
                category: "act".into(),
                op: "tap".into(),
                args_json: json!({"xy": xy, "by": by, "value": value, "exactMatch": exact_match})
                    .to_string(),
            },
            ActCommands::Swipe { from, to, duration } => CommandShape {
                category: "act".into(),
                op: "swipe".into(),
                args_json: json!({"from": from, "to": to, "duration": duration}).to_string(),
            },
            ActCommands::Back => CommandShape {
                category: "act".into(),
                op: "back".into(),
                args_json: "{}".into(),
            },
            ActCommands::Home => CommandShape {
                category: "act".into(),
                op: "home".into(),
                args_json: "{}".into(),
            },
            ActCommands::Text { text } => CommandShape {
                category: "act".into(),
                op: "text".into(),
                args_json: json!({"text": text}).to_string(),
            },
            ActCommands::Launch { package_name } => CommandShape {
                category: "act".into(),
                op: "launch".into(),
                args_json: json!({"packageName": package_name}).to_string(),
            },
            ActCommands::Stop { package_name } => CommandShape {
                category: "act".into(),
                op: "stop".into(),
                args_json: json!({"packageName": package_name}).to_string(),
            },
            ActCommands::Key { key_code } => CommandShape {
                category: "act".into(),
                op: "key".into(),
                args_json: json!({"keyCode": key_code}).to_string(),
            },
        },
        Commands::Verify { command, .. } => match command {
            VerifyCommands::TextContains { text, ignore_case } => CommandShape {
                category: "verify".into(),
                op: "text-contains".into(),
                args_json: json!({"text": text, "ignoreCase": ignore_case}).to_string(),
            },
            VerifyCommands::TopActivity { expected, mode } => CommandShape {
                category: "verify".into(),
                op: "top-activity".into(),
                args_json: json!({"expected": expected, "mode": mode}).to_string(),
            },
            VerifyCommands::NodeExists {
                by,
                value,
                exact_match,
            } => CommandShape {
                category: "verify".into(),
                op: "node-exists".into(),
                args_json: json!({
                    "by": match by.as_str() {
                        "desc" => "content_desc",
                        "class" => "class_name",
                        other => other,
                    },
                    "value": value,
                    "exactMatch": exact_match,
                })
                .to_string(),
            },
        },
        Commands::Recover { command, .. } => match command {
            RecoverCommands::Back { times } => CommandShape {
                category: "recover".into(),
                op: "back".into(),
                args_json: json!({"times": times}).to_string(),
            },
            RecoverCommands::Home => CommandShape {
                category: "recover".into(),
                op: "home".into(),
                args_json: "{}".into(),
            },
            RecoverCommands::Relaunch { package_name } => CommandShape {
                category: "recover".into(),
                op: "relaunch".into(),
                args_json: json!({"packageName": package_name}).to_string(),
            },
        },
        _ => CommandShape {
            category: String::new(),
            op: String::new(),
            args_json: "{}".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse_cli(args: &str) -> Cli {
        let mut parts = vec!["af".to_string()];
        let mut iter = args.split_whitespace();
        let first = iter.next().expect("at least one command token");
        parts.push(first.to_string());
        if first != "memory" {
            parts.push("--url".to_string());
            parts.push("http://127.0.0.1:18080".to_string());
            if first != "health" {
                parts.push("--token".to_string());
                parts.push("demo-token".to_string());
            }
        }
        parts.extend(iter.map(str::to_string));
        Cli::parse_from(parts)
    }

    #[test]
    fn only_act_verify_recover_are_recorded() {
        let cases = [
            ("health", false),
            ("act back", true),
            ("verify text-contains --text x", true),
            ("recover home", true),
            ("observe top", false),
            ("memory stats", false),
        ];
        for (args, expected) in cases {
            let cli = parse_cli(args);
            assert_eq!(
                should_record_event(&cli.command),
                expected,
                "failed for: {args}"
            );
        }
    }

    #[test]
    fn observe_top_screen_refs_update_session_cache() {
        let cases = [
            ("observe top", true),
            ("observe screen", true),
            ("observe refs --max-rows 80", true),
            ("observe screenshot", false),
            ("act back", false),
        ];
        for (args, expected) in cases {
            let cli = parse_cli(args);
            assert_eq!(
                should_update_session_cache(&cli.command),
                expected,
                "failed for: {args}"
            );
        }
    }

    #[test]
    fn observe_top_extracts_top_activity_field() {
        let store = MemoryStore::new_in_memory().expect("init");
        let cli = parse_cli("observe top");
        let result =
            json!({"status": "ok", "data": {"topActivity": "com.android.settings/.Settings"}});
        update_session_cache(&store, &cli.session, &cli.command, &result);
        let ctx = store
            .get_session_state(&cli.session)
            .expect("get")
            .expect("exists");
        assert_eq!(ctx.app, "com.android.settings");
        assert_eq!(ctx.activity, "com.android.settings/.Settings");
        assert!(
            ctx.page_fingerprint.is_empty(),
            "top should not generate fingerprint"
        );
    }

    #[test]
    fn observe_screen_generates_fingerprint() {
        let store = MemoryStore::new_in_memory().expect("init");
        // Pre-seed activity from observe top
        store
            .update_session_activity("default", "com.a", "com.a/.Main", "2026-01-01T00:00:00Z")
            .expect("seed");

        let cli = parse_cli("observe screen");
        let result = json!({
            "status": "ok",
            "data": {
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "rows": [
                    {"class_name": "android.widget.FrameLayout", "res_id": "com.a:id/main_pane"},
                    {"class_name": "androidx.recyclerview.widget.RecyclerView", "res_id": "com.a:id/list"},
                    {"class_name": "android.widget.TextView", "text": "Hello"}
                ]
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);
        let ctx = store
            .get_session_state(&cli.session)
            .expect("get")
            .expect("exists");
        assert_eq!(ctx.fingerprint_source, "screen");
        assert!(ctx.page_fingerprint.contains("act=com.a/.Main"));
        assert!(ctx.page_fingerprint.contains("rid="));
        assert!(ctx.page_fingerprint.contains("main_pane"));
    }

    #[test]
    fn observe_screen_uses_top_activity_from_result() {
        let store = MemoryStore::new_in_memory().expect("init");
        store
            .update_session_activity("default", "com.a", "com.a/.Main", "2026-01-01T00:00:00Z")
            .expect("seed old activity");

        let cli = parse_cli("observe screen");
        let result = json!({
            "status": "ok",
            "data": {
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "topActivity": "com.b/.New",
                "rows": [
                    {"class_name": "android.widget.FrameLayout", "res_id": "com.b:id/content"}
                ]
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(got.activity, "com.b/.New");
        assert_eq!(got.app, "com.b");
        assert!(
            got.page_fingerprint.contains("act=com.b/.New"),
            "topActivity from result should be in fingerprint, got: {}",
            got.page_fingerprint
        );
    }

    #[test]
    fn observe_screen_falls_back_to_cached_activity() {
        let store = MemoryStore::new_in_memory().expect("init");
        store
            .update_session_activity("default", "com.a", "com.a/.Main", "2026-01-01T00:00:00Z")
            .expect("seed");

        let cli = parse_cli("observe screen");
        let result = json!({
            "status": "ok",
            "data": {
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "rows": [
                    {"class_name": "android.widget.FrameLayout", "res_id": "com.a:id/main_pane"}
                ]
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert!(
            got.page_fingerprint.contains("act=com.a/.Main"),
            "should fall back to cached activity when topActivity absent, got: {}",
            got.page_fingerprint
        );
    }

    #[test]
    fn observe_refs_does_not_overwrite_screen_fingerprint() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|mode=SYSTEM_API|wv=0|rid=main_pane".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "2026-01-01T00:00:00Z".into(),
        };
        store.update_session_state("default", &ctx).expect("seed");

        let cli = parse_cli("observe refs --max-rows 80");
        let result = json!({
            "status": "ok",
            "data": {
                "refVersion": 5,
                "refCount": 10,
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "rows": [{"ref": "@n1", "id": "1", "class": "android.widget.Button", "resId": "com.a:id/btn"}]
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(
            got.fingerprint_source, "screen",
            "refs should not overwrite screen fingerprint"
        );
        assert_eq!(got.ref_version, Some(5), "ref_version should be updated");
    }

    #[test]
    fn observe_refs_with_new_top_activity_recomputes_fingerprint() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|mode=SYSTEM_API|wv=0|rid=main_pane".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: Some(1),
            observed_at: "2026-01-01T00:00:00Z".into(),
        };
        store.update_session_state("default", &ctx).expect("seed");

        let cli = parse_cli("observe refs --max-rows 80");
        let result = json!({
            "status": "ok",
            "data": {
                "topActivity": "com.b/.Settings",
                "refVersion": 5,
                "refCount": 10,
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "rows": [{"ref": "@n1", "id": "1", "class": "android.widget.Button", "resId": "com.b:id/btn"}]
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(got.activity, "com.b/.Settings");
        assert_eq!(got.app, "com.b");
        assert_eq!(got.fingerprint_source, "refs");
        assert!(
            got.page_fingerprint.contains("act=com.b/.Settings"),
            "fingerprint should be rebuilt for the new activity"
        );
        assert_eq!(got.ref_version, Some(5));
    }

    #[test]
    fn failed_observe_does_not_update_cache() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0|rid=list".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "2026-01-01T00:00:00Z".into(),
        };
        store.update_session_state("default", &ctx).expect("seed");

        let cli = parse_cli("observe screen");
        let result = json!({
            "status": "failed",
            "error": {"code": "NETWORK_ERROR", "message": "connection refused"}
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(
            got.page_fingerprint, "act=com.a/.Main|wv=0|rid=list",
            "failed observe should not overwrite fingerprint"
        );
        assert_eq!(got.fingerprint_source, "screen");
    }

    #[test]
    fn observe_page_without_top_activity_preserves_existing_context() {
        let store = MemoryStore::new_in_memory().expect("init");
        let ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|mode=SYSTEM_API|wv=0|rid=main_pane".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: Some(1),
            observed_at: "2026-01-01T00:00:00Z".into(),
        };
        store.update_session_state("default", &ctx).expect("seed");

        let cli = parse_cli("observe page --field refs");
        let result = json!({
            "status": "ok",
            "data": {
                "topActivity": null,
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "refs": {
                    "refVersion": 9,
                    "refCount": 2,
                    "updatedAtMs": 1000,
                    "rows": [
                        {"ref": "@n1", "id": "1", "class_name": "android.widget.Button", "res_id": "com.b:id/btn", "bounds": "0,0,100,100", "flags": "clk"}
                    ]
                }
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(got.activity, "com.a/.Main");
        assert_eq!(got.app, "com.a");
        assert_eq!(
            got.page_fingerprint, "act=com.a/.Main|mode=SYSTEM_API|wv=0|rid=main_pane",
            "missing topActivity should not overwrite existing identity context"
        );
        assert_eq!(got.ref_version, Some(1));
    }

    #[test]
    fn extract_failure_cause_from_message_prefix() {
        let result = json!({
            "status": "failed",
            "error": {
                "code": "SERVER_ERROR",
                "message": "REF_ALIAS_STALE: ref=@n5"
            }
        });
        let cause = extract_failure_cause(&result);
        assert_eq!(cause.as_deref(), Some("REF_ALIAS_STALE"));
    }

    #[test]
    fn extract_failure_cause_falls_back_to_code() {
        let result = json!({
            "status": "failed",
            "error": {
                "code": "ASSERTION_FAILED",
                "message": "expected text not found"
            }
        });
        let cause = extract_failure_cause(&result);
        assert_eq!(cause.as_deref(), Some("ASSERTION_FAILED"));
    }

    #[test]
    fn extract_failure_cause_returns_none_for_ok() {
        let result = json!({"status": "ok", "data": {}});
        assert!(extract_failure_cause(&result).is_none());
    }

    #[test]
    fn observe_page_updates_session_with_atomic_data() {
        let store = MemoryStore::new_in_memory().expect("init");

        let cli = parse_cli("observe page --field screen --field refs");
        let result = json!({
            "status": "ok",
            "data": {
                "topActivity": "com.example/.HomePage",
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "screen": {
                    "rowCount": 2,
                    "rows": [
                        {"class": "android.widget.FrameLayout", "res_id": "com.example:id/root"},
                        {"class": "android.widget.RecyclerView", "res_id": "com.example:id/list"}
                    ]
                },
                "refs": {
                    "refVersion": 7,
                    "refCount": 3,
                    "updatedAtMs": 1000,
                    "rows": [
                        {"ref": "@n1", "id": "1", "class_name": "android.widget.Button", "res_id": "com.example:id/btn", "bounds": "0,0,100,100", "flags": "clk"}
                    ]
                }
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(got.activity, "com.example/.HomePage");
        assert_eq!(got.app, "com.example");
        assert_eq!(got.fingerprint_source, "screen");
        assert!(got.page_fingerprint.contains("act=com.example/.HomePage"));
        assert!(got.page_fingerprint.contains("rid="));
        assert_eq!(got.ref_version, Some(7));
        assert!(!got.has_webview);
    }

    #[test]
    fn observe_page_refs_only_uses_refs_fingerprint() {
        let store = MemoryStore::new_in_memory().expect("init");

        let cli = parse_cli("observe page --field refs");
        let result = json!({
            "status": "ok",
            "data": {
                "topActivity": "com.b/.Settings",
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "refs": {
                    "refVersion": 2,
                    "refCount": 5,
                    "updatedAtMs": 2000,
                    "rows": [
                        {"ref": "@n1", "id": "1", "class_name": "android.widget.Switch", "res_id": "com.b:id/toggle", "bounds": "0,0,50,50", "flags": "clk"}
                    ]
                }
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(got.fingerprint_source, "refs");
        assert!(got.page_fingerprint.contains("act=com.b/.Settings"));
        assert_eq!(got.ref_version, Some(2));
    }

    #[test]
    fn observe_page_no_rows_builds_minimal_fingerprint() {
        let store = MemoryStore::new_in_memory().expect("init");

        let cli = parse_cli("observe page");
        let result = json!({
            "status": "ok",
            "data": {
                "topActivity": "com.c/.Splash",
                "mode": "SYSTEM_API",
                "hasWebView": false,
                "nodeReliability": "high",
                "screen": {
                    "rowCount": 0,
                    "rows": []
                }
            }
        });
        update_session_cache(&store, &cli.session, &cli.command, &result);

        let got = store
            .get_session_state("default")
            .expect("get")
            .expect("exists");
        assert_eq!(got.activity, "com.c/.Splash");
        assert_eq!(got.mode, "SYSTEM_API");
        assert!(!got.has_webview);
        assert_eq!(got.fingerprint_source, "screen");
    }
}
