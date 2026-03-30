mod api;
mod builder;
mod cli;
mod core;
mod memory;

use clap::Parser;
use crossbeam_channel::{Receiver, bounded, select};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process;
use std::time::Instant;

use crate::api::request::{ApiClient, ApiError, ApiErrorKind};
use crate::builder::ReqClientBuilder;
use crate::cli::{ActCommands, Cli, Commands, ObserveCommands, RecoverCommands, VerifyCommands};
use crate::core::error_code::ErrorCode;
use crate::memory::{TraceRecord, TraceStore};

fn main() -> anyhow::Result<()> {
    let ctrl_c_events = ctrl_channel()?;
    let cli = Cli::parse();
    let trace_store = if cli.no_trace {
        None
    } else {
        Some(TraceStore::new(cli.trace_db.clone())?)
    };

    let runtime = ReqClientBuilder::new(
        cli.url.trim_end_matches('/').to_string(),
        cli.timeout_ms,
        cli.proxy,
    )
    .with_token(cli.token.clone());

    let client = runtime.build()?;

    let started = Instant::now();
    let ref_scope = build_ref_scope(&runtime.base_url, runtime.token.as_deref());
    let result = run_command(
        &client,
        &runtime,
        &ctrl_c_events,
        &cli.command,
        trace_store.as_ref(),
        &ref_scope,
    );
    persist_trace(&trace_store, &cli, &runtime.session_id, &result, started.elapsed().as_millis());
    println!("{}", serde_json::to_string(&result)?);

    let exit_code = match result.get("status").and_then(|x| x.as_str()) {
        Some("ok") => 0,
        Some("interrupted") => 130,
        _ => 1,
    };
    if exit_code != 0 {
        process::exit(exit_code);
    }
    Ok(())
}

fn persist_trace(
    trace_store: &Option<TraceStore>,
    cli: &Cli,
    trace_id: &str,
    result: &Value,
    duration_ms: u128,
) {
    let Some(store) = trace_store else {
        return;
    };

    let status = result
        .get("status")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let output_json = match serde_json::to_string(result) {
        Ok(v) => v,
        Err(e) => format!(r#"{{"traceSerializeError":"{e}"}}"#),
    };
    let command = format!("{:?}", cli.command);
    let record = TraceRecord {
        created_at: chrono::Utc::now().to_rfc3339(),
        session: cli.session.clone(),
        trace_id: trace_id.to_string(),
        command,
        status,
        output_json,
        duration_ms,
    };
    if let Err(e) = store.record(&record) {
        eprintln!("warn: failed to persist trace: {e}");
    }
}

fn run_command(
    client: &Client,
    runtime: &ReqClientBuilder,
    ctrl_c_events: &Receiver<()>,
    cmd: &Commands,
    trace_store: Option<&TraceStore>,
    ref_scope: &str,
) -> Value {
    let api = ApiClient::new(
        client,
        runtime.base_url.as_str(),
        runtime.token.as_deref(),
        ctrl_c_events,
    );

    match cmd {
        Commands::Health => {
            let op = "health";
            into_output(&runtime.session_id, "health", op, handle_health(&api))
        }
        Commands::Act { command } => match command {
            ActCommands::Tap {
                x,
                y,
                by,
                value,
                exact_match,
            } => into_output(
                &runtime.session_id,
                "act",
                "tap",
                handle_act_tap(
                    &api,
                    *x,
                    *y,
                    by.as_ref().map(|s| s.as_str()),
                    value.as_ref().map(|s| s.as_str()),
                    *exact_match,
                ),
            ),
            ActCommands::Swipe { coords, duration } => into_output(
                &runtime.session_id,
                "act",
                "swipe",
                handle_act_swipe(
                    &api,
                    coords[0],
                    coords[1],
                    coords[2],
                    coords[3],
                    *duration,
                ),
            ),
            ActCommands::Back => {
                into_output(&runtime.session_id, "act", "back", handle_act_back(&api))
            }
            ActCommands::Home => {
                into_output(&runtime.session_id, "act", "home", handle_act_home(&api))
            }
            ActCommands::Text { text } => into_output(
                &runtime.session_id,
                "act",
                "text",
                handle_act_text(&api, text),
            ),
            ActCommands::Launch { package_name } => into_output(
                &runtime.session_id,
                "act",
                "launch",
                handle_act_launch(&api, package_name),
            ),
            ActCommands::Stop { package_name } => into_output(
                &runtime.session_id,
                "act",
                "stop",
                handle_act_stop(&api, package_name),
            ),
            ActCommands::Key { key_code } => into_output(
                &runtime.session_id,
                "act",
                "key",
                handle_act_key(&api, *key_code),
            ),
        },
        Commands::Observe { command } => match command {
            ObserveCommands::Screen { mode, max_rows, fields } => into_output(
                &runtime.session_id,
                "observe",
                "screen",
                handle_observe_screen(&api, mode, *max_rows, fields),
            ),
            ObserveCommands::Overlay {
                enabled,
                max_marks,
                interactive_only,
                auto_refresh,
                refresh_interval_ms,
                offset_x,
                offset_y,
            } => into_output(
                &runtime.session_id,
                "observe",
                "overlay",
                handle_observe_overlay(
                    &api,
                    *enabled,
                    *max_marks,
                    interactive_only.unwrap_or(false),
                    auto_refresh.unwrap_or(true),
                    *refresh_interval_ms,
                    *offset_x,
                    *offset_y,
                ),
            ),
            ObserveCommands::Screenshot {
                max_dim,
                quality,
                annotate,
                hide_overlay,
                max_marks,
                interactive_only,
            } => into_output(
                &runtime.session_id,
                "observe",
                "screenshot",
                handle_observe_screenshot(
                    &api,
                    *max_dim,
                    *quality,
                    *annotate,
                    *hide_overlay,
                    *max_marks,
                    interactive_only.unwrap_or(false),
                ),
            ),
            ObserveCommands::Top => into_output(
                &runtime.session_id,
                "observe",
                "top",
                handle_observe_top(&api),
            ),
            ObserveCommands::Refs { max_rows } => into_output(
                &runtime.session_id,
                "observe",
                "refs",
                handle_observe_refs(&api, *max_rows, trace_store, ref_scope),
            ),
        },
        Commands::Verify { command } => match command {
            VerifyCommands::TextContains { text, ignore_case } => into_output(
                &runtime.session_id,
                "verify",
                "text-contains",
                handle_verify_text_contains(&api, text, *ignore_case),
            ),
            VerifyCommands::TopActivity { expected, mode } => into_output(
                &runtime.session_id,
                "verify",
                "top-activity",
                handle_verify_top_activity(&api, expected, mode),
            ),
            VerifyCommands::NodeExists {
                by,
                value,
                exact_match,
            } => into_output(
                &runtime.session_id,
                "verify",
                "node-exists",
                handle_verify_node_exists(&api, by, value, *exact_match),
            ),
        },
        Commands::Recover { command } => match command {
            RecoverCommands::Back { times } => into_output(
                &runtime.session_id,
                "recover",
                "back",
                handle_recover_back(&api, *times),
            ),
            RecoverCommands::Home => into_output(
                &runtime.session_id,
                "recover",
                "home",
                handle_recover_home(&api),
            ),
            RecoverCommands::Relaunch { package_name } => into_output(
                &runtime.session_id,
                "recover",
                "relaunch",
                handle_recover_relaunch(&api, package_name),
            ),
        },
    }
}

#[derive(Debug)]
struct CommandError {
    code: ErrorCode,
    message: String,
    retryable: bool,
    status: Option<u16>,
    raw: Option<String>,
    details: Option<Value>,
}

impl CommandError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            retryable: false,
            status: None,
            raw: None,
            details: None,
        }
    }

    fn assertion_failed_with_details(message: impl Into<String>, details: Value) -> Self {
        Self {
            code: ErrorCode::AssertionFailed,
            message: message.into(),
            retryable: false,
            status: None,
            raw: None,
            details: Some(details),
        }
    }
}

impl From<ApiError> for CommandError {
    fn from(value: ApiError) -> Self {
        Self {
            code: map_api_error_kind(value.kind),
            message: value.message,
            retryable: value.retryable,
            status: value.status,
            raw: value.raw,
            details: None,
        }
    }
}

fn map_api_error_kind(kind: ApiErrorKind) -> ErrorCode {
    match kind {
        ApiErrorKind::Interrupted => ErrorCode::Interrupted,
        ApiErrorKind::Auth => ErrorCode::AuthError,
        ApiErrorKind::InvalidParams => ErrorCode::InvalidParams,
        ApiErrorKind::Network => ErrorCode::NetworkError,
        ApiErrorKind::Server => ErrorCode::ServerError,
        ApiErrorKind::BadResponse => ErrorCode::ServerError,
        ApiErrorKind::Internal => ErrorCode::InternalError,
    }
}

type CommandResult = Result<Value, CommandError>;

fn into_output(trace_id: &str, category: &str, op: &str, result: CommandResult) -> Value {
    match result {
        Ok(data) => json!({
            "traceId": trace_id,
            "status": "ok",
            "category": category,
            "op": op,
            "data": data
        }),
        Err(err) => {
            let status = if err.code == ErrorCode::Interrupted {
                "interrupted"
            } else {
                "failed"
            };
            json!({
                "traceId": trace_id,
                "status": status,
                "category": category,
                "op": op,
                "error": {
                    "code": err.code,
                    "message": err.message,
                    "retryable": err.retryable,
                    "status": err.status,
                    "raw": err.raw,
                    "details": err.details
                }
            })
        }
    }
}

fn handle_health(api: &ApiClient<'_>) -> CommandResult {
    let health = api.health().map_err(CommandError::from)?;
    Ok(json!({"health": health.payload}))
}

fn handle_act_tap(
    api: &ApiClient<'_>,
    x: Option<f32>,
    y: Option<f32>,
    by: Option<&str>,
    value: Option<&str>,
    exact_match: bool,
) -> CommandResult {
    match (x, y, by, value) {
        (Some(xv), Some(yv), None, None) => {
            let msg = api.tap(xv, yv).map_err(CommandError::from)?;
            Ok(json!({"mode": "coords", "x": xv, "y": yv, "result": msg.message}))
        }
        (None, None, Some(by_raw), Some(value_raw)) => {
            if value_raw.trim().is_empty() {
                return Err(CommandError::invalid_params("value must not be empty"));
            }
            let by_api = normalize_semantic_tap_by(by_raw)?;
            let msg = api
                .tap_node(&by_api, value_raw, exact_match)
                .map_err(CommandError::from)?;
            Ok(
                json!({"mode": "semantic", "by": by_raw, "byNormalized": by_api, "value": value_raw, "exactMatch": exact_match, "result": msg.message}),
            )
        }
        _ => Err(CommandError::invalid_params(
            "tap requires either (--x and --y) or (--by and --value), and these two modes cannot be mixed",
        )),
    }
}

fn handle_act_swipe(
    api: &ApiClient<'_>,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    duration: i64,
) -> CommandResult {
    let msg = api
        .swipe(x1, y1, x2, y2, duration)
        .map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
}

fn handle_act_back(api: &ApiClient<'_>) -> CommandResult {
    let msg = api.press_back().map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
}

fn handle_act_home(api: &ApiClient<'_>) -> CommandResult {
    let msg = api.press_home().map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
}

fn handle_act_text(api: &ApiClient<'_>, text: &str) -> CommandResult {
    if text.is_empty() {
        return Err(CommandError::invalid_params("text must not be empty"));
    }
    let msg = api.input_text(text).map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
}

fn handle_act_launch(api: &ApiClient<'_>, package_name: &str) -> CommandResult {
    let msg = api.app_launch(package_name).map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
}

fn handle_act_stop(api: &ApiClient<'_>, package_name: &str) -> CommandResult {
    let msg = api.app_stop(package_name).map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
}

fn handle_act_key(api: &ApiClient<'_>, key_code: i32) -> CommandResult {
    let msg = api.press_key(key_code).map_err(CommandError::from)?;
    Ok(json!({"keyCode": key_code, "result": msg.message}))
}

fn handle_observe_screen(
    api: &ApiClient<'_>,
    mode: &str,
    max_rows: usize,
    fields: &str,
) -> CommandResult {
    let full = match mode {
        "compact" => false,
        "full" => true,
        _ => {
            return Err(CommandError::invalid_params(
                "mode must be compact or full",
            ));
        }
    };
    let screen = api.screen().map_err(CommandError::from)?;
    let total_rows = screen.rows.len();
    if full {
        return Ok(
            json!({
                "mode": screen.mode,
                "rowCount": total_rows,
                "rows": screen.rows,
                "raw": screen.raw,
                "full": true,
                "hasWebView": screen.has_webview,
                "nodeReliability": screen.node_reliability
            }),
        );
    }

    let selected_fields = parse_screen_fields(fields)?;
    let rows = screen
        .rows
        .into_iter()
        .take(max_rows)
        .map(|r| compact_row_json(r, &selected_fields))
        .collect::<Vec<_>>();
    Ok(
        json!({
            "mode": screen.mode,
            "rowCount": total_rows,
            "returnedRows": rows.len(),
            "truncated": total_rows > rows.len(),
            "rows": rows,
            "full": false,
            "fields": selected_fields,
            "hasWebView": screen.has_webview,
            "nodeReliability": screen.node_reliability
        }),
    )
}

fn handle_observe_overlay(
    api: &ApiClient<'_>,
    enabled: Option<bool>,
    max_marks: usize,
    interactive_only: bool,
    auto_refresh: bool,
    refresh_interval_ms: u64,
    offset_x: Option<i32>,
    offset_y: Option<i32>,
) -> CommandResult {
    let state = if let Some(target) = enabled {
        api.overlay_set(
            target,
            max_marks,
            interactive_only,
            auto_refresh,
            refresh_interval_ms,
            offset_x,
            offset_y,
        )
            .map_err(CommandError::from)?
    } else {
        api.overlay_get().map_err(CommandError::from)?
    };
    Ok(state.payload)
}

fn handle_observe_screenshot(
    api: &ApiClient<'_>,
    max_dim: i64,
    quality: i64,
    annotate: bool,
    hide_overlay: Option<bool>,
    max_marks: usize,
    interactive_only: bool,
) -> CommandResult {
    let shot = api
        .screenshot(
            max_dim,
            quality,
            annotate,
            hide_overlay,
            max_marks,
            interactive_only,
        )
        .map_err(CommandError::from)?;
    Ok(json!({
        "screenshotBase64": shot.base64,
        "maxDim": max_dim,
        "quality": quality,
        "annotate": annotate,
        "hideOverlay": hide_overlay,
        "maxMarks": max_marks,
        "interactiveOnly": interactive_only
    }))
}

fn handle_observe_top(api: &ApiClient<'_>) -> CommandResult {
    let top = api.top_activity().map_err(CommandError::from)?;
    Ok(json!({"topActivity": top.activity}))
}

fn handle_observe_refs(
    api: &ApiClient<'_>,
    max_rows: usize,
    _trace_store: Option<&TraceStore>,
    _ref_scope: &str,
) -> CommandResult {
    let refs = api.screen_refs().map_err(CommandError::from)?;
    let rows = refs
        .rows
        .into_iter()
        .take(max_rows)
        .map(|r| {
            json!({
                "ref": r.ref_id,
                "id": r.node_id,
                "class": r.class_name,
                "text": r.text,
                "desc": r.desc,
                "resId": r.res_id,
                "bounds": r.bounds,
                "flags": r.flags
            })
        })
        .collect::<Vec<_>>();
    Ok(
        json!({
            "refVersion": refs.ref_version,
            "refCount": refs.ref_count,
            "updatedAtMs": refs.updated_at_ms,
            "mode": refs.mode,
            "hasWebView": refs.has_webview,
            "nodeReliability": refs.node_reliability,
            "returnedRows": rows.len(),
            "rows": rows
        }),
    )
}

fn handle_verify_text_contains(
    api: &ApiClient<'_>,
    text: &str,
    ignore_case: bool,
) -> CommandResult {
    let screen = api.screen().map_err(CommandError::from)?;
    let matched_rows = screen
        .rows
        .iter()
        .filter(|r| {
            matches_text(r.text.as_deref(), text, ignore_case)
                || matches_text(r.desc.as_deref(), text, ignore_case)
                || matches_text(r.res_id.as_deref(), text, ignore_case)
        })
        .cloned()
        .collect::<Vec<_>>();
    let matched_in_rows = !matched_rows.is_empty();
    let matched_in_raw = matches_text(Some(&screen.raw), text, ignore_case);
    let matched = matched_in_rows || matched_in_raw;
    if !matched {
        return Err(CommandError::assertion_failed_with_details(
            format!("text not found in screen: {text}"),
            json!({
                "check": "text_contains",
                "expectedText": text,
                "ignoreCase": ignore_case,
                "actualContains": false,
                "searchTargets": ["row.text", "row.desc", "row.res_id", "raw"],
                "rowCount": screen.rows.len(),
                "mode": screen.mode
            }),
        ));
    }
    Ok(
        json!({"matched": true, "text": text, "ignoreCase": ignore_case, "matchedInRows": matched_in_rows, "matchedInRaw": matched_in_raw, "matchedRows": matched_rows}),
    )
}

fn handle_verify_top_activity(api: &ApiClient<'_>, expected: &str, mode: &str) -> CommandResult {
    if mode != "contains" && mode != "equals" {
        return Err(CommandError::invalid_params(
            "mode must be contains or equals",
        ));
    }
    let top = api.top_activity().map_err(CommandError::from)?;
    let matched = if mode == "equals" {
        top.activity == expected
    } else {
        top.activity.contains(expected)
    };
    if !matched {
        return Err(CommandError::assertion_failed_with_details(
            format!(
                "top activity mismatch: expected {mode} {expected}, got {}",
                top.activity
            ),
            json!({
                "check": "top_activity",
                "mode": mode,
                "expected": expected,
                "actual": top.activity
            }),
        ));
    }
    Ok(json!({"matched": true, "expected": expected, "actual": top.activity, "mode": mode}))
}

fn handle_verify_node_exists(
    api: &ApiClient<'_>,
    by: &str,
    value: &str,
    exact_match: bool,
) -> CommandResult {
    let by_norm = by.to_lowercase();
    let by_api = match by_norm.as_str() {
        "text" | "resource_id" | "content_desc" | "class_name" => by_norm.clone(),
        "desc" => "content_desc".to_string(),
        "class" => "class_name".to_string(),
        _ => {
            return Err(CommandError::invalid_params(
                "by must be one of: text,content_desc,resource_id,class_name (aliases: desc,class)",
            ));
        }
    };
    let found = api
        .nodes_find(&by_api, value, exact_match)
        .map_err(CommandError::from)?;
    if !found.has_match {
        let screen_meta = api.screen().ok().map(|screen| {
            json!({
                "mode": screen.mode,
                "rowCount": screen.rows.len(),
                "hasWebView": screen.has_webview,
                "nodeReliability": screen.node_reliability
            })
        });
        let hint = match screen_meta
            .as_ref()
            .and_then(|m| m.get("hasWebView"))
            .and_then(|v| v.as_bool())
        {
            Some(true) => {
                "WEBVIEW_LIMITATION_POSSIBLE: try verify text-contains or switch to native page"
            }
            _ => "TRY_OBSERVE_SCREEN_AND_ADJUST_MATCH_STRATEGY",
        };
        return Err(CommandError::assertion_failed_with_details(
            format!("node not found: by={by}, value={value}"),
            json!({
                "check": "node_exists",
                "by": by,
                "byNormalized": by_api,
                "value": value,
                "exactMatch": exact_match,
                "matched": false,
                "matchedCount": found.matched_count,
                "nodes": found.nodes,
                "raw": found.raw,
                "hint": hint,
                "screenMeta": screen_meta
            }),
        ));
    }
    Ok(
        json!({"matched": true, "by": by, "byNormalized": by_api, "value": value, "exactMatch": exact_match, "matchedCount": found.matched_count, "nodes": found.nodes, "raw": found.raw}),
    )
}

fn normalize_semantic_tap_by(by: &str) -> Result<String, CommandError> {
    let by_norm = by.to_lowercase();
    match by_norm.as_str() {
        "text" => Ok("text".to_string()),
        "desc" | "content_desc" => Ok("content_desc".to_string()),
        "resid" | "resource_id" | "res_id" => Ok("resource_id".to_string()),
        "ref" => Ok("ref".to_string()),
        _ => Err(CommandError::invalid_params(
            "tap --by must be one of: text,desc,resid,ref (aliases: content_desc,resource_id,res_id)",
        )),
    }
}

fn build_ref_scope(base_url: &str, token: Option<&str>) -> String {
    let mut hasher = DefaultHasher::new();
    base_url.hash(&mut hasher);
    token.unwrap_or("").hash(&mut hasher);
    format!("{}|{:016x}", base_url, hasher.finish())
}

fn handle_recover_back(api: &ApiClient<'_>, times: u32) -> CommandResult {
    if times == 0 {
        return Err(CommandError::invalid_params("times must be >= 1"));
    }
    for _ in 0..times {
        let _ = api.press_back().map_err(CommandError::from)?;
    }
    Ok(json!({"times": times}))
}

fn handle_recover_home(api: &ApiClient<'_>) -> CommandResult {
    let _ = api.press_home().map_err(CommandError::from)?;
    Ok(json!({}))
}

fn handle_recover_relaunch(api: &ApiClient<'_>, package_name: &str) -> CommandResult {
    let _ = api.press_home().map_err(CommandError::from)?;
    let launch = api.app_launch(package_name).map_err(CommandError::from)?;
    Ok(json!({"packageName": package_name, "launchResult": launch.message}))
}

fn parse_screen_fields(fields: &str) -> Result<Vec<String>, CommandError> {
    let supported = ["id", "class", "text", "desc", "resId", "flags", "bounds"];
    let mut parsed = fields
        .split(',')
        .map(|f| f.trim())
        .filter(|f| !f.is_empty())
        .map(normalize_field_name)
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        parsed = vec![
            "id".to_string(),
            "class".to_string(),
            "text".to_string(),
            "desc".to_string(),
            "resId".to_string(),
            "flags".to_string(),
        ];
    }
    for p in &parsed {
        if !supported.contains(&p.as_str()) {
            return Err(CommandError::invalid_params(format!(
                "unsupported field '{p}', supported: id,class,text,desc,resId,flags,bounds"
            )));
        }
    }
    let mut out = Vec::<String>::new();
    for p in parsed {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    Ok(out)
}

fn normalize_field_name(field: &str) -> String {
    match field {
        "nodeId" => "id".to_string(),
        "className" => "class".to_string(),
        "res_id" => "resId".to_string(),
        x => x.to_string(),
    }
}

fn compact_row_json(row: crate::api::request::ScreenRow, fields: &[String]) -> Value {
    let mut obj = serde_json::Map::new();
    for f in fields {
        match f.as_str() {
            "id" => {
                obj.insert("id".to_string(), json!(row.node_id));
            }
            "class" => {
                obj.insert("class".to_string(), json!(row.class_name));
            }
            "text" => {
                obj.insert("text".to_string(), json!(row.text));
            }
            "desc" => {
                obj.insert("desc".to_string(), json!(row.desc));
            }
            "resId" => {
                obj.insert("resId".to_string(), json!(row.res_id));
            }
            "flags" => {
                obj.insert("flags".to_string(), json!(row.flags));
            }
            "bounds" => {
                obj.insert("bounds".to_string(), json!(row.bounds));
            }
            _ => {}
        }
    }
    Value::Object(obj)
}

fn matches_text(value: Option<&str>, expected: &str, ignore_case: bool) -> bool {
    let Some(v) = value else {
        return false;
    };
    if ignore_case {
        v.to_lowercase().contains(&expected.to_lowercase())
    } else {
        v.contains(expected)
    }
}

pub(crate) fn run_with_interrupt<T, F>(ctrl_c_events: &Receiver<()>, work: F) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    let (done_tx, done_rx) = bounded::<anyhow::Result<T>>(1);
    std::thread::spawn(move || {
        let _ = done_tx.send(work());
    });

    select! {
        recv(ctrl_c_events) -> _ => Err(anyhow::anyhow!("Interrupted by SIGINT (Ctrl+C)")),
        recv(done_rx) -> msg => {
            match msg {
                Ok(res) => res,
                Err(_) => Err(anyhow::anyhow!("worker channel closed unexpectedly")),
            }
        }
    }
}

fn ctrl_channel() -> Result<Receiver<()>, ctrlc::Error> {
    let (sender, receiver) = bounded(100);
    ctrlc::set_handler(move || {
        let _ = sender.send(());
    })?;
    Ok(receiver)
}
