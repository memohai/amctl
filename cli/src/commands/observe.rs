use crate::api::request::{ApiClient, OverlaySetRequest};
use crate::cli::{MarkScope, PageSliceArg, ScreenFieldArg};
use crate::commands::common::{OverlaySetOptions, compact_row_json, parse_screen_fields};
use crate::output::{CommandError, CommandResult};
use serde_json::{Value, json};

pub fn handle_screen(
    api: &ApiClient<'_>,
    full: bool,
    max_rows: Option<usize>,
    fields: &[ScreenFieldArg],
) -> CommandResult {
    let selected_fields = parse_screen_fields(fields);
    let max_rows = max_rows.unwrap_or(120);
    let screen = api.screen().map_err(CommandError::from)?;
    let total_rows = screen.rows.len();
    if full {
        return Ok(json!({
            "mode": screen.mode,
            "rowCount": total_rows,
            "rows": screen.rows,
            "raw": screen.raw,
            "full": true,
            "hasWebView": screen.has_webview,
            "nodeReliability": screen.node_reliability,
            "topActivity": screen.top_activity
        }));
    }

    let rows = screen
        .rows
        .into_iter()
        .take(max_rows)
        .map(|row| compact_row_json(row, &selected_fields))
        .collect::<Vec<_>>();
    Ok(json!({
        "mode": screen.mode,
        "rowCount": total_rows,
        "returnedRows": rows.len(),
        "truncated": total_rows > rows.len(),
        "rows": rows,
        "full": false,
        "fields": selected_fields,
        "hasWebView": screen.has_webview,
        "nodeReliability": screen.node_reliability,
        "topActivity": screen.top_activity
    }))
}

pub fn handle_overlay_get(api: &ApiClient<'_>) -> CommandResult {
    let state = api.overlay_get().map_err(CommandError::from)?;
    Ok(state.payload)
}

pub fn handle_overlay_set(api: &ApiClient<'_>, options: OverlaySetOptions) -> CommandResult {
    if matches!(options.refresh, crate::cli::RefreshMode::Off)
        && options.refresh_interval_ms.is_some()
    {
        return Err(CommandError::invalid_params(
            "--refresh-interval-ms cannot be used when --refresh off",
        ));
    }
    let request = OverlaySetRequest {
        enabled: options.enabled,
        max_marks: options.max_marks,
        interactive_only: matches!(options.mark_scope, MarkScope::Interactive),
        auto_refresh: matches!(options.refresh, crate::cli::RefreshMode::On),
        refresh_interval_ms: options.refresh_interval_ms.unwrap_or(800),
        offset_x: options.offset_x,
        offset_y: options.offset_y,
    };
    let state = api.overlay_set(&request).map_err(CommandError::from)?;
    Ok(state.payload)
}

pub fn handle_screenshot(
    api: &ApiClient<'_>,
    max_dim: i64,
    quality: i64,
    annotate: bool,
    hide_overlay: bool,
    max_marks: Option<usize>,
    mark_scope: Option<MarkScope>,
) -> CommandResult {
    let interactive_only = matches!(mark_scope.unwrap_or(MarkScope::All), MarkScope::Interactive);
    let max_marks = max_marks.unwrap_or(120);
    let shot = api
        .screenshot(
            max_dim,
            quality,
            annotate,
            if hide_overlay { Some(true) } else { None },
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
        "markScope": if interactive_only { "interactive" } else { "all" }
    }))
}

pub fn handle_top(api: &ApiClient<'_>) -> CommandResult {
    let top = api.top_activity().map_err(CommandError::from)?;
    Ok(json!({"topActivity": top.activity}))
}

pub fn handle_refs(api: &ApiClient<'_>, max_rows: usize) -> CommandResult {
    let refs = api.screen_refs().map_err(CommandError::from)?;
    let rows = refs
        .rows
        .into_iter()
        .take(max_rows)
        .map(|row| {
            json!({
                "ref": row.ref_id,
                "id": row.node_id,
                "class": row.class_name,
                "text": row.text,
                "desc": row.desc,
                "resId": row.res_id,
                "bounds": row.bounds,
                "flags": row.flags
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "refVersion": refs.ref_version,
        "refCount": refs.ref_count,
        "updatedAtMs": refs.updated_at_ms,
        "mode": refs.mode,
        "hasWebView": refs.has_webview,
        "nodeReliability": refs.node_reliability,
        "returnedRows": rows.len(),
        "rows": rows,
        "topActivity": refs.top_activity
    }))
}

pub fn handle_page(api: &ApiClient<'_>, fields: &[PageSliceArg], max_rows: usize) -> CommandResult {
    let mut slices: Vec<&str> = vec!["top"];
    if fields.is_empty() {
        slices.push("screen");
    } else {
        for f in fields {
            match f {
                PageSliceArg::Screen => slices.push("screen"),
                PageSliceArg::Refs => slices.push("refs"),
            }
        }
    }
    let resp = api
        .observe(&slices, Some(max_rows))
        .map_err(CommandError::from)?;

    let mut out = serde_json::Map::new();

    out.insert("topActivity".into(), json!(resp.top_activity));
    out.insert("mode".into(), json!(resp.mode));
    out.insert("hasWebView".into(), json!(resp.has_webview));
    out.insert("nodeReliability".into(), json!(resp.node_reliability));

    if let Some(screen) = &resp.screen {
        out.insert(
            "screen".into(),
            json!({
                "rowCount": screen.row_count,
                "returnedRows": screen.rows.len(),
                "rows": screen.rows,
            }),
        );
    }

    if let Some(refs) = &resp.refs {
        let ref_rows: Vec<Value> = refs
            .rows
            .iter()
            .map(|row| {
                json!({
                    "ref": row.ref_id,
                    "id": row.node_id,
                    "class": row.class_name,
                    "text": row.text,
                    "desc": row.desc,
                    "resId": row.res_id,
                    "bounds": row.bounds,
                    "flags": row.flags
                })
            })
            .collect();
        out.insert(
            "refs".into(),
            json!({
                "refVersion": refs.ref_version,
                "refCount": refs.ref_count,
                "updatedAtMs": refs.updated_at_ms,
                "returnedRows": ref_rows.len(),
                "rows": ref_rows,
            }),
        );
    }

    Ok(Value::Object(out))
}
