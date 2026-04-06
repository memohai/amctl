use crate::api::request::ApiClient;
use crate::commands::common::matches_text;
use crate::output::{CommandError, CommandResult};
use serde_json::json;

pub fn handle_text_contains(api: &ApiClient<'_>, text: &str, ignore_case: bool) -> CommandResult {
    let screen = api.screen().map_err(CommandError::from)?;
    let searchable_raw = searchable_screen_raw(&screen.raw);
    let matched_rows = screen
        .rows
        .iter()
        .filter(|row| {
            matches_text(row.text.as_deref(), text, ignore_case)
                || matches_text(row.desc.as_deref(), text, ignore_case)
                || matches_text(row.res_id.as_deref(), text, ignore_case)
        })
        .cloned()
        .collect::<Vec<_>>();
    let matched_in_rows = !matched_rows.is_empty();
    let matched_in_raw = matches_text(Some(&searchable_raw), text, ignore_case);
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
    Ok(json!({
        "matched": true,
        "text": text,
        "ignoreCase": ignore_case,
        "matchedInRows": matched_in_rows,
        "matchedInRaw": matched_in_raw,
        "matchedRows": matched_rows
    }))
}

fn searchable_screen_raw(raw: &str) -> String {
    raw.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("[mode: ")
                && !trimmed.starts_with("[topActivity: ")
                && !trimmed.starts_with("screen:")
                && !trimmed.starts_with("--- window:")
                && trimmed != "node_id\tclass\ttext\tdesc\tres_id\tbounds\tflags"
                && trimmed != "hierarchy:"
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn handle_top_activity(api: &ApiClient<'_>, expected: &str, mode: &str) -> CommandResult {
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

pub fn handle_node_exists(
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
            .and_then(|meta| meta.get("hasWebView"))
            .and_then(|value| value.as_bool())
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
    Ok(json!({
        "matched": true,
        "by": by,
        "byNormalized": by_api,
        "value": value,
        "exactMatch": exact_match,
        "matchedCount": found.matched_count,
        "nodes": found.nodes,
        "raw": found.raw
    }))
}

#[cfg(test)]
mod tests {
    use super::searchable_screen_raw;

    #[test]
    fn searchable_screen_raw_strips_metadata_headers() {
        let raw = "[mode: V2_SHIZUKU]\n[topActivity: com.android.settings/.Settings]\nscreen:1080x1920\n--- window:1 type:APPLICATION ---\nnode_id\tclass\ttext\tdesc\tres_id\tbounds\tflags\nn1\tTextView\tWi-Fi\t-\tcom.a:id/title\t0,0,10,10\tclk\nhierarchy:\nn1";
        let searchable = searchable_screen_raw(raw);
        assert!(
            !searchable.contains("topActivity"),
            "metadata header should be removed"
        );
        assert!(!searchable.contains("V2_SHIZUKU"));
        assert!(
            searchable.contains("Wi-Fi"),
            "screen row content should remain"
        );
    }
}
