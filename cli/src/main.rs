mod api;
mod builder;
mod cli;
mod core;

use clap::Parser;
use crossbeam_channel::{Receiver, bounded, select};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::process;

use crate::api::request::{ApiClient, ApiError, ApiErrorKind};
use crate::builder::ReqClientBuilder;
use crate::cli::{ActCommands, Cli, Commands, ObserveCommands, RecoverCommands, VerifyCommands};
use crate::core::error_code::ErrorCode;

fn main() -> anyhow::Result<()> {
    let ctrl_c_events = ctrl_channel()?;
    let cli = Cli::parse();

    let runtime = ReqClientBuilder::new(
        cli.url.trim_end_matches('/').to_string(),
        cli.timeout_ms,
        cli.proxy,
    )
    .with_token(cli.token.clone());

    let client = runtime.build()?;

    let result = run_command(&client, &runtime, &ctrl_c_events, &cli.command);
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

fn run_command(
    client: &Client,
    runtime: &ReqClientBuilder,
    ctrl_c_events: &Receiver<()>,
    cmd: &Commands,
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
            ActCommands::Tap { x, y } => into_output(
                &runtime.session_id,
                "act",
                "tap",
                handle_act_tap(&api, *x, *y),
            ),
            ActCommands::Swipe {
                x1,
                y1,
                x2,
                y2,
                duration,
            } => into_output(
                &runtime.session_id,
                "act",
                "swipe",
                handle_act_swipe(&api, *x1, *y1, *x2, *y2, *duration),
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
            ObserveCommands::Screen {
                full,
                max_rows,
                fields,
            } => into_output(
                &runtime.session_id,
                "observe",
                "screen",
                handle_observe_screen(&api, *full, *max_rows, fields),
            ),
            ObserveCommands::Screenshot { max_dim, quality } => into_output(
                &runtime.session_id,
                "observe",
                "screenshot",
                handle_observe_screenshot(&api, *max_dim, *quality),
            ),
            ObserveCommands::Top => into_output(
                &runtime.session_id,
                "observe",
                "top",
                handle_observe_top(&api),
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

fn handle_act_tap(api: &ApiClient<'_>, x: f32, y: f32) -> CommandResult {
    let msg = api.tap(x, y).map_err(CommandError::from)?;
    Ok(json!({"result": msg.message}))
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
    full: bool,
    max_rows: usize,
    fields: &str,
) -> CommandResult {
    let screen = api.screen().map_err(CommandError::from)?;
    let total_rows = screen.rows.len();
    if full {
        return Ok(
            json!({"mode": screen.mode, "rowCount": total_rows, "rows": screen.rows, "raw": screen.raw, "full": true}),
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
        json!({"mode": screen.mode, "rowCount": total_rows, "returnedRows": rows.len(), "truncated": total_rows > rows.len(), "rows": rows, "full": false, "fields": selected_fields}),
    )
}

fn handle_observe_screenshot(api: &ApiClient<'_>, max_dim: i64, quality: i64) -> CommandResult {
    let shot = api
        .screenshot(max_dim, quality)
        .map_err(CommandError::from)?;
    Ok(json!({"screenshotBase64": shot.base64, "maxDim": max_dim, "quality": quality}))
}

fn handle_observe_top(api: &ApiClient<'_>) -> CommandResult {
    let top = api.top_activity().map_err(CommandError::from)?;
    Ok(json!({"topActivity": top.activity}))
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
    let valid = ["id", "text", "desc", "class", "resource_id"];
    if !valid.contains(&by_norm.as_str()) {
        return Err(CommandError::invalid_params(
            "by must be one of: id,text,desc,class,resource_id",
        ));
    }
    let found = api
        .nodes_find(&by_norm, value, exact_match)
        .map_err(CommandError::from)?;
    if !found.has_match {
        return Err(CommandError::assertion_failed_with_details(
            format!("node not found: by={by}, value={value}"),
            json!({
                "check": "node_exists",
                "by": by,
                "value": value,
                "exactMatch": exact_match,
                "matched": false,
                "matchedCount": found.matched_count,
                "nodes": found.nodes,
                "raw": found.raw
            }),
        ));
    }
    Ok(
        json!({"matched": true, "by": by, "value": value, "exactMatch": exact_match, "matchedCount": found.matched_count, "nodes": found.nodes, "raw": found.raw}),
    )
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
