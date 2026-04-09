use crate::api::request::{ApiClient, OverlaySetRequest};
use crate::artifact::{ArtifactManager, SaveRequest, json_bytes, subdir};
use crate::cli::{MarkScope, PageSliceArg, ScreenFieldArg};
use crate::commands::common::{OverlaySetOptions, compact_row_json, parse_screen_fields};
use crate::output::{CommandError, CommandResult};
use base64::Engine;
use serde_json::{Value, json};
use std::path::Path;

pub struct ScreenshotOptions<'a> {
    pub max_dim: i64,
    pub quality: i64,
    pub annotate: bool,
    pub hide_overlay: bool,
    pub max_marks: Option<usize>,
    pub mark_scope: Option<MarkScope>,
    pub save_file: Option<&'a Path>,
}

fn fingerprint_rows_screen(rows: &[crate::api::request::ScreenRow]) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            json!({
                "class": row.class_name,
                "resId": row.res_id,
            })
        })
        .collect()
}

fn fingerprint_rows_page(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            json!({
                "class": row.get("class").cloned().unwrap_or(Value::Null),
                "resId": row.get("resId").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

pub fn handle_screen(
    api: &ApiClient<'_>,
    artifacts: &ArtifactManager<'_>,
    full: bool,
    save_file: Option<&Path>,
    max_rows: Option<usize>,
    fields: &[ScreenFieldArg],
) -> CommandResult {
    if !full && save_file.is_some() {
        return Err(CommandError::invalid_params(
            "--save-file requires --full for observe screen",
        ));
    }
    let selected_fields = parse_screen_fields(fields);
    let max_rows = max_rows.unwrap_or(120);
    let screen = api.screen().map_err(CommandError::from)?;
    let total_rows = screen.rows.len();
    if full {
        let fingerprint_rows = fingerprint_rows_screen(&screen.rows);
        let full_payload = json!({
            "mode": screen.mode,
            "rowCount": total_rows,
            "rows": screen.rows,
            "raw": screen.raw,
            "full": true,
            "hasWebView": screen.has_webview,
            "nodeReliability": screen.node_reliability,
            "topActivity": screen.top_activity
        });
        let bytes = json_bytes(&full_payload)?;
        let default_dir = subdir(artifacts.artifact_dir, "screen-full");
        let artifact = artifacts.save(SaveRequest {
            kind: "screen_full_json",
            category: "observe",
            op: "screen",
            mime_type: "application/json",
            extension: "json",
            bytes: &bytes,
            explicit_file: save_file,
            default_file: artifacts.screen_file,
            default_dir: &default_dir,
        })?;
        return Ok(json!({
            "mode": screen.mode,
            "rowCount": total_rows,
            "full": true,
            "hasWebView": screen.has_webview,
            "nodeReliability": screen.node_reliability,
            "topActivity": screen.top_activity,
            "fingerprintRows": fingerprint_rows,
            "artifact": artifact
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
    artifacts: &ArtifactManager<'_>,
    options: ScreenshotOptions<'_>,
) -> CommandResult {
    let interactive_only = matches!(
        options.mark_scope.unwrap_or(MarkScope::All),
        MarkScope::Interactive
    );
    let max_marks = options.max_marks.unwrap_or(120);
    let shot = api
        .screenshot(
            options.max_dim,
            options.quality,
            options.annotate,
            if options.hide_overlay {
                Some(true)
            } else {
                None
            },
            max_marks,
            interactive_only,
        )
        .map_err(CommandError::from)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(shot.base64.as_bytes())
        .map_err(|e| CommandError::internal(format!("decode screenshot base64 failed: {e}")))?;
    let artifact = artifacts.save(SaveRequest {
        kind: "screenshot_image",
        category: "observe",
        op: "screenshot",
        mime_type: "image/jpeg",
        extension: "jpg",
        bytes: &bytes,
        explicit_file: options.save_file,
        default_file: artifacts.screenshot_file,
        default_dir: &subdir(artifacts.artifact_dir, "screenshots"),
    })?;
    Ok(json!({
        "maxDim": options.max_dim,
        "quality": options.quality,
        "annotate": options.annotate,
        "hideOverlay": options.hide_overlay,
        "maxMarks": max_marks,
        "markScope": if interactive_only { "interactive" } else { "all" },
        "artifact": artifact
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

pub fn handle_page(
    api: &ApiClient<'_>,
    artifacts: &ArtifactManager<'_>,
    save_dir: Option<&Path>,
    fields: &[PageSliceArg],
    max_rows: usize,
) -> CommandResult {
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
        let fingerprint_rows = fingerprint_rows_page(&screen.rows);
        let screen_payload = json!({
            "rowCount": screen.row_count,
            "returnedRows": screen.rows.len(),
            "rows": screen.rows,
        });
        let bytes = json_bytes(&screen_payload)?;
        let artifact = artifacts.save(SaveRequest {
            kind: "page_screen_json",
            category: "observe",
            op: "page",
            mime_type: "application/json",
            extension: "json",
            bytes: &bytes,
            explicit_file: None,
            default_file: None,
            default_dir: save_dir.unwrap_or(artifacts.page_dir),
        })?;
        out.insert(
            "screen".into(),
            json!({
                "rowCount": screen.row_count,
                "returnedRows": screen.rows.len(),
                "fingerprintRows": fingerprint_rows,
                "artifact": artifact,
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
