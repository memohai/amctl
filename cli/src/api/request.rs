use std::error::Error;
use std::fmt::{Display, Formatter};

use crossbeam_channel::Receiver;
use reqwest::{Method, StatusCode, Url, blocking::Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::run_with_interrupt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiErrorKind {
    Interrupted,
    Auth,
    InvalidParams,
    Network,
    Server,
    BadResponse,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub kind: ApiErrorKind,
    pub message: String,
    pub retryable: bool,
    pub status: Option<u16>,
    pub raw: Option<String>,
}

impl ApiError {
    fn new(kind: ApiErrorKind, message: impl Into<String>) -> Self {
        let retryable = matches!(kind, ApiErrorKind::Interrupted | ApiErrorKind::Network);
        Self {
            kind,
            message: message.into(),
            retryable,
            status: None,
            raw: None,
        }
    }

    fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    fn with_raw(mut self, raw: Option<String>) -> Self {
        self.raw = raw;
        self
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlaySetRequest {
    pub enabled: bool,
    pub max_marks: usize,
    pub interactive_only: bool,
    pub auto_refresh: bool,
    pub refresh_interval_ms: u64,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenResponse {
    pub raw: String,
    pub mode: Option<String>,
    pub top_activity: Option<String>,
    pub rows: Vec<ScreenRow>,
    pub has_webview: bool,
    pub node_reliability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRefsResponse {
    #[serde(rename = "refVersion")]
    pub ref_version: u64,
    #[serde(rename = "refCount")]
    pub ref_count: usize,
    #[serde(rename = "updatedAtMs")]
    pub updated_at_ms: u64,
    pub mode: String,
    #[serde(rename = "hasWebView")]
    pub has_webview: bool,
    #[serde(rename = "nodeReliability")]
    pub node_reliability: String,
    pub rows: Vec<RefRow>,
    #[serde(rename = "topActivity")]
    pub top_activity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveResponse {
    #[serde(rename = "topActivity")]
    pub top_activity: Option<String>,
    pub mode: String,
    #[serde(rename = "hasWebView")]
    pub has_webview: bool,
    #[serde(rename = "nodeReliability")]
    pub node_reliability: String,
    pub screen: Option<ObserveScreenSlice>,
    pub refs: Option<ObserveRefsSlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveScreenSlice {
    #[serde(rename = "rowCount")]
    pub row_count: usize,
    pub rows: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveRefsSlice {
    #[serde(rename = "refVersion")]
    pub ref_version: u64,
    #[serde(rename = "refCount")]
    pub ref_count: usize,
    #[serde(rename = "updatedAtMs")]
    pub updated_at_ms: u64,
    pub rows: Vec<RefRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefRow {
    #[serde(rename = "ref")]
    pub ref_id: String,
    pub node_id: String,
    pub class_name: Option<String>,
    pub text: Option<String>,
    pub desc: Option<String>,
    pub res_id: Option<String>,
    pub bounds: String,
    pub flags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRow {
    pub node_id: String,
    pub class_name: String,
    pub text: Option<String>,
    pub desc: Option<String>,
    pub res_id: Option<String>,
    pub bounds: Option<String>,
    pub flags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResponse {
    pub base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayResponse {
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopActivityResponse {
    pub activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesFindResponse {
    pub raw: String,
    pub has_match: bool,
    pub matched_count: usize,
    pub nodes: Vec<NodeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub class_name: String,
    pub text: Option<String>,
    pub content_desc: Option<String>,
    pub resource_id: Option<String>,
    pub bounds: Option<String>,
}

#[derive(Deserialize)]
struct ApiEnvelope {
    ok: bool,
    data: Option<String>,
    error: Option<String>,
}

type Query<'a> = [(&'a str, String)];

pub struct ApiClient<'a> {
    client: &'a Client,
    base_url: &'a str,
    token: Option<&'a str>,
    ctrl_c_events: &'a Receiver<()>,
}

impl<'a> ApiClient<'a> {
    pub fn new(
        client: &'a Client,
        base_url: &'a str,
        token: Option<&'a str>,
        ctrl_c_events: &'a Receiver<()>,
    ) -> Self {
        Self {
            client,
            base_url,
            token,
            ctrl_c_events,
        }
    }

    pub fn health(&self) -> ApiResult<HealthResponse> {
        let payload = self.send_json(Method::GET, "/health", None, None, false)?;
        Ok(HealthResponse { payload })
    }

    pub fn tap(&self, x: f32, y: f32) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope("/api/tap", Some(json!({"x": x, "y": y})))?;
        Ok(ActionResponse { message })
    }

    pub fn tap_node(&self, by: &str, value: &str, exact_match: bool) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope(
            "/api/nodes/tap",
            Some(json!({"by": by, "value": value, "exact_match": exact_match})),
        )?;
        Ok(ActionResponse { message })
    }

    pub fn swipe(
        &self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        duration: i64,
    ) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope(
            "/api/swipe",
            Some(json!({"x1": x1, "y1": y1, "x2": x2, "y2": y2, "duration": duration})),
        )?;
        Ok(ActionResponse { message })
    }

    pub fn press_back(&self) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope("/api/press/back", Some(json!({})))?;
        Ok(ActionResponse { message })
    }

    pub fn press_home(&self) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope("/api/press/home", Some(json!({})))?;
        Ok(ActionResponse { message })
    }

    pub fn input_text(&self, text: &str) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope("/api/text", Some(json!({"text": text})))?;
        Ok(ActionResponse { message })
    }

    pub fn app_launch(&self, package_name: &str) -> ApiResult<ActionResponse> {
        let message = self.authed_post_envelope(
            "/api/app/launch",
            Some(json!({"package_name": package_name})),
        )?;
        Ok(ActionResponse { message })
    }

    pub fn app_stop(&self, package_name: &str) -> ApiResult<ActionResponse> {
        let message = self
            .authed_post_envelope("/api/app/stop", Some(json!({"package_name": package_name})))?;
        Ok(ActionResponse { message })
    }

    pub fn press_key(&self, key_code: i32) -> ApiResult<ActionResponse> {
        let message =
            self.authed_post_envelope("/api/press/key", Some(json!({"key_code": key_code})))?;
        Ok(ActionResponse { message })
    }

    pub fn screen(&self) -> ApiResult<ScreenResponse> {
        let raw = self.authed_get_envelope("/api/screen", None)?;
        let parsed = parse_screen_tsv(&raw);
        let has_webview = has_webview_nodes(&parsed.rows);
        let node_reliability = infer_node_reliability(has_webview, parsed.rows.len()).to_string();
        Ok(ScreenResponse {
            raw,
            mode: parsed.mode,
            top_activity: parsed.top_activity,
            rows: parsed.rows,
            has_webview,
            node_reliability,
        })
    }

    pub fn screen_refs(&self) -> ApiResult<ScreenRefsResponse> {
        let raw = self.authed_get_envelope("/api/screen/refs", None)?;
        serde_json::from_str::<ScreenRefsResponse>(&raw).map_err(|e| {
            ApiError::new(
                ApiErrorKind::BadResponse,
                format!("Unexpected /api/screen/refs payload format ({e})"),
            )
            .with_raw(Some(raw))
        })
    }

    pub fn observe(&self, include: &[&str], max_rows: Option<usize>) -> ApiResult<ObserveResponse> {
        let include_param = include.join(",");
        let mut params: Vec<(&str, String)> = vec![("include", include_param)];
        if let Some(mr) = max_rows {
            params.push(("max_rows", mr.to_string()));
        }
        let raw = self.authed_get_envelope("/api/observe", Some(&params))?;
        serde_json::from_str::<ObserveResponse>(&raw).map_err(|e| {
            ApiError::new(
                ApiErrorKind::BadResponse,
                format!("Unexpected /api/observe payload format ({e})"),
            )
            .with_raw(Some(raw))
        })
    }

    pub fn screenshot(
        &self,
        max_dim: i64,
        quality: i64,
        annotate: bool,
        hide_overlay: Option<bool>,
        max_marks: usize,
        interactive_only: bool,
    ) -> ApiResult<ScreenshotResponse> {
        let mut query = vec![
            ("max_dim", max_dim.to_string()),
            ("quality", quality.to_string()),
            ("annotate", annotate.to_string()),
            ("max_marks", max_marks.to_string()),
            ("interactive_only", interactive_only.to_string()),
        ];
        if let Some(v) = hide_overlay {
            query.push(("hide_overlay", v.to_string()));
        }
        let base64 = self.authed_get_envelope("/api/screenshot", Some(&query))?;
        Ok(ScreenshotResponse { base64 })
    }

    pub fn overlay_get(&self) -> ApiResult<OverlayResponse> {
        let raw = self.authed_get_envelope("/api/overlay", None)?;
        let payload = parse_embedded_json(&raw, "/api/overlay")?;
        Ok(OverlayResponse { payload })
    }

    pub fn overlay_set(&self, request: &OverlaySetRequest) -> ApiResult<OverlayResponse> {
        let mut body = serde_json::Map::new();
        body.insert("enabled".to_string(), json!(request.enabled));
        body.insert("max_marks".to_string(), json!(request.max_marks));
        body.insert(
            "interactive_only".to_string(),
            json!(request.interactive_only),
        );
        body.insert("auto_refresh".to_string(), json!(request.auto_refresh));
        body.insert(
            "refresh_interval_ms".to_string(),
            json!(request.refresh_interval_ms),
        );
        if let Some(v) = request.offset_x {
            body.insert("offset_x".to_string(), json!(v));
        }
        if let Some(v) = request.offset_y {
            body.insert("offset_y".to_string(), json!(v));
        }
        let raw = self.authed_post_envelope("/api/overlay", Some(Value::Object(body)))?;
        let payload = parse_embedded_json(&raw, "/api/overlay")?;
        Ok(OverlayResponse { payload })
    }

    pub fn top_activity(&self) -> ApiResult<TopActivityResponse> {
        let activity = self.authed_get_envelope("/api/app/top", None)?;
        Ok(TopActivityResponse { activity })
    }

    pub fn nodes_find(
        &self,
        by: &str,
        value: &str,
        exact_match: bool,
    ) -> ApiResult<NodesFindResponse> {
        let raw = self.authed_post_envelope(
            "/api/nodes/find",
            Some(json!({"by": by, "value": value, "exact_match": exact_match})),
        )?;
        let parsed = parse_nodes_find_message(&raw);
        Ok(NodesFindResponse {
            raw,
            has_match: parsed.matched_count > 0,
            matched_count: parsed.matched_count,
            nodes: parsed.nodes,
        })
    }

    fn authed_get_envelope(&self, path: &str, query: Option<&Query<'_>>) -> ApiResult<String> {
        self.send_envelope(Method::GET, path, query, None, true)
    }

    fn authed_post_envelope(&self, path: &str, body: Option<Value>) -> ApiResult<String> {
        self.send_envelope(Method::POST, path, None, body, true)
    }

    fn send_envelope(
        &self,
        method: Method,
        path: &str,
        query: Option<&Query<'_>>,
        body: Option<Value>,
        require_auth: bool,
    ) -> ApiResult<String> {
        let value = self.send_json(method, path, query, body, require_auth)?;
        unwrap_envelope(path, value)
    }

    fn send_json(
        &self,
        method: Method,
        path: &str,
        query: Option<&Query<'_>>,
        body: Option<Value>,
        require_auth: bool,
    ) -> ApiResult<Value> {
        if require_auth && self.token.is_none() {
            return Err(ApiError::new(
                ApiErrorKind::Auth,
                "token is required for this command",
            ));
        }

        let url = build_url(self.base_url, path).map_err(|e| {
            ApiError::new(
                ApiErrorKind::InvalidParams,
                format!("invalid url: {}{} ({e})", self.base_url, path),
            )
        })?;

        let mut req = self
            .client
            .request(method.clone(), url.clone())
            .header(header::CONTENT_TYPE, "application/json");

        if let Some(params) = query {
            req = req.query(params);
        }

        if let Some(t) = self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }

        if let Some(b) = body {
            req = req.json(&b);
        }

        let built_req = req.build().map_err(|e| {
            ApiError::new(
                ApiErrorKind::Internal,
                format!("failed to build request: {method} {url} ({e})"),
            )
        })?;

        let client_cloned = self.client.clone();
        let method_for_req = method.clone();
        let url_for_req = url.clone();
        let (status, text) = run_with_interrupt(self.ctrl_c_events, move || {
            let resp = client_cloned.execute(built_req).map_err(|e| {
                anyhow::anyhow!("request failed: {method_for_req} {url_for_req} ({e})")
            })?;
            let status = resp.status();
            let text = resp.text().map_err(|e| {
                anyhow::anyhow!("read response body failed: {method_for_req} {url_for_req} ({e})")
            })?;
            Ok((status, text))
        })
        .map_err(map_transport_error)?;

        if status == StatusCode::UNAUTHORIZED {
            return Err(ApiError::new(
                ApiErrorKind::Auth,
                "Unauthorized: invalid or missing bearer token",
            )
            .with_status(status.as_u16())
            .with_raw(Some(text)));
        }

        if !status.is_success() {
            let msg = extract_error_message(&text).unwrap_or_else(|| {
                format!(
                    "{} {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("")
                )
            });
            return Err(ApiError::new(ApiErrorKind::Server, msg)
                .with_status(status.as_u16())
                .with_raw(Some(text)));
        }

        serde_json::from_str::<Value>(&text).map_err(|e| {
            ApiError::new(
                ApiErrorKind::BadResponse,
                format!("invalid json response: {method} {url} ({e})"),
            )
            .with_status(status.as_u16())
            .with_raw(Some(text))
        })
    }
}

fn map_transport_error(err: anyhow::Error) -> ApiError {
    let msg = err.to_string();
    if msg.contains("Interrupted by SIGINT") {
        return ApiError::new(ApiErrorKind::Interrupted, msg);
    }
    ApiError::new(ApiErrorKind::Network, msg)
}

fn unwrap_envelope(op: &str, input: Value) -> ApiResult<String> {
    let env: ApiEnvelope = serde_json::from_value(input).map_err(|e| {
        ApiError::new(
            ApiErrorKind::BadResponse,
            format!("Unexpected {op} response format ({e})"),
        )
    })?;

    if !env.ok {
        return Err(ApiError::new(
            ApiErrorKind::Server,
            env.error.unwrap_or_else(|| format!("{op} failed")),
        ));
    }

    Ok(env.data.unwrap_or_default())
}

fn extract_error_message(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
        return Some(err.to_string());
    }
    if let Some(msg) = v.get("message").and_then(|x| x.as_str()) {
        return Some(msg.to_string());
    }
    None
}

fn parse_embedded_json(raw: &str, op: &str) -> ApiResult<Value> {
    serde_json::from_str::<Value>(raw).map_err(|e| {
        ApiError::new(
            ApiErrorKind::BadResponse,
            format!("Unexpected {op} payload format ({e})"),
        )
        .with_raw(Some(raw.to_string()))
    })
}

fn build_url(base_url: &str, path: &str) -> anyhow::Result<Url> {
    Ok(Url::parse(&format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        path
    ))?)
}

#[derive(Default)]
struct ParsedNodes {
    matched_count: usize,
    nodes: Vec<NodeSummary>,
}

#[derive(Default)]
struct ParsedScreen {
    mode: Option<String>,
    top_activity: Option<String>,
    rows: Vec<ScreenRow>,
}

fn parse_nodes_find_message(raw: &str) -> ParsedNodes {
    if raw.starts_with("No nodes found") {
        return ParsedNodes::default();
    }
    let mut lines = raw.lines();
    let Some(header) = lines.next() else {
        return ParsedNodes::default();
    };
    let matched_count = parse_found_count(header).unwrap_or(0);
    let nodes = lines.filter_map(parse_node_line).collect::<Vec<_>>();
    ParsedNodes {
        matched_count: matched_count.max(nodes.len()),
        nodes,
    }
}

fn parse_found_count(header: &str) -> Option<usize> {
    // Expected: "Found <N> node(s):"
    if !header.starts_with("Found ") {
        return None;
    }
    let rest = header.trim_start_matches("Found ");
    let n = rest.split_whitespace().next()?;
    n.parse::<usize>().ok()
}

fn parse_node_line(line: &str) -> Option<NodeSummary> {
    // Expected columns:
    // id \t class \t text=... \t desc=... \t res=... \t bounds=...
    let cols = line.split('\t').collect::<Vec<_>>();
    if cols.len() < 2 {
        return None;
    }
    let id = cols[0].trim().to_string();
    let class_name = cols[1].trim().to_string();
    let mut node = NodeSummary {
        id,
        class_name,
        text: None,
        content_desc: None,
        resource_id: None,
        bounds: None,
    };
    for col in cols.iter().skip(2) {
        if let Some(v) = col.strip_prefix("text=") {
            node.text = parse_optional_field(v);
            continue;
        }
        if let Some(v) = col.strip_prefix("desc=") {
            node.content_desc = parse_optional_field(v);
            continue;
        }
        if let Some(v) = col.strip_prefix("res=") {
            node.resource_id = parse_optional_field(v);
            continue;
        }
        if let Some(v) = col.strip_prefix("bounds=") {
            node.bounds = parse_optional_field(v);
        }
    }
    Some(node)
}

fn parse_optional_field(v: &str) -> Option<String> {
    let value = v.trim();
    if value.is_empty() || value == "null" || value == "-" {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_screen_tsv(raw: &str) -> ParsedScreen {
    let mut parsed = ParsedScreen::default();
    let mut in_node_table = false;
    let mut in_hierarchy = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(mode) = parse_mode_line(trimmed) {
            parsed.mode = Some(mode);
            continue;
        }

        if let Some(activity) = parse_top_activity_line(trimmed) {
            parsed.top_activity = Some(activity);
            continue;
        }

        if trimmed.starts_with("--- window:") {
            in_node_table = false;
            in_hierarchy = false;
            continue;
        }

        if trimmed == "node_id\tclass\ttext\tdesc\tres_id\tbounds\tflags" {
            in_node_table = true;
            in_hierarchy = false;
            continue;
        }

        if trimmed == "hierarchy:" {
            in_hierarchy = true;
            in_node_table = false;
            continue;
        }

        if in_hierarchy {
            continue;
        }

        if in_node_table {
            if let Some(row) = parse_screen_row(trimmed) {
                parsed.rows.push(row);
            }
        }
    }

    parsed
}

fn parse_mode_line(line: &str) -> Option<String> {
    if line.starts_with("[mode: ") && line.ends_with(']') {
        return Some(
            line.trim_start_matches("[mode: ")
                .trim_end_matches(']')
                .to_string(),
        );
    }
    None
}

fn parse_top_activity_line(line: &str) -> Option<String> {
    if line.starts_with("[topActivity: ") && line.ends_with(']') {
        let val = line
            .trim_start_matches("[topActivity: ")
            .trim_end_matches(']')
            .to_string();
        if val.is_empty() {
            return None;
        }
        return Some(val);
    }
    None
}

fn parse_screen_row(line: &str) -> Option<ScreenRow> {
    let cols = line.split('\t').collect::<Vec<_>>();
    if cols.len() != 7 {
        return None;
    }
    Some(ScreenRow {
        node_id: cols[0].trim().to_string(),
        class_name: cols[1].trim().to_string(),
        text: parse_optional_field(cols[2]),
        desc: parse_optional_field(cols[3]),
        res_id: parse_optional_field(cols[4]),
        bounds: parse_optional_field(cols[5]),
        flags: parse_optional_field(cols[6]),
    })
}

fn has_webview_nodes(rows: &[ScreenRow]) -> bool {
    rows.iter()
        .any(|row| row.class_name.to_ascii_lowercase().contains("webview"))
}

fn infer_node_reliability(has_webview: bool, row_count: usize) -> &'static str {
    if has_webview || row_count == 0 {
        "low"
    } else {
        "high"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_envelope_ok_returns_data() {
        let input = json!({"ok": true, "data": "hello"});
        let out = unwrap_envelope("/api/test", input).expect("envelope should parse");
        assert_eq!(out, "hello");
    }

    #[test]
    fn unwrap_envelope_error_returns_message() {
        let input = json!({"ok": false, "error": "bad"});
        let err = unwrap_envelope("/api/test", input).expect_err("expected error");
        assert_eq!(err.kind, ApiErrorKind::Server);
        assert!(err.to_string().contains("bad"));
    }

    #[test]
    fn extract_error_message_prefers_error_then_message() {
        assert_eq!(
            extract_error_message(r#"{"error":"x","message":"y"}"#),
            Some("x".to_string())
        );
        assert_eq!(
            extract_error_message(r#"{"message":"y"}"#),
            Some("y".to_string())
        );
        assert_eq!(extract_error_message("not-json"), None);
    }

    #[test]
    fn build_url_handles_trailing_slash() {
        let url = build_url("http://127.0.0.1:8081/", "/api/x").expect("url should build");
        assert_eq!(url.as_str(), "http://127.0.0.1:8081/api/x");
    }

    #[test]
    fn map_transport_error_interrupted() {
        let err = map_transport_error(anyhow::anyhow!("Interrupted by SIGINT (Ctrl+C)"));
        assert_eq!(err.kind, ApiErrorKind::Interrupted);
        assert!(err.retryable);
    }

    #[test]
    fn parse_nodes_find_message_handles_no_match() {
        let p = parse_nodes_find_message("No nodes found matching text='abc'");
        assert_eq!(p.matched_count, 0);
        assert!(p.nodes.is_empty());
    }

    #[test]
    fn parse_nodes_find_message_parses_rows() {
        let raw = "Found 2 node(s):\nnode1\tandroid.widget.TextView\ttext=Hello\tdesc=\tres=com.app:id/title\tbounds=[0,0][100,40]\nnode2\tandroid.widget.Button\ttext=\tdesc=Go\tres=\tbounds=[0,50][100,90]";
        let p = parse_nodes_find_message(raw);
        assert_eq!(p.matched_count, 2);
        assert_eq!(p.nodes.len(), 2);
        assert_eq!(p.nodes[0].id, "node1");
        assert_eq!(p.nodes[0].class_name, "android.widget.TextView");
        assert_eq!(p.nodes[0].text.as_deref(), Some("Hello"));
        assert_eq!(p.nodes[0].resource_id.as_deref(), Some("com.app:id/title"));
        assert_eq!(p.nodes[1].content_desc.as_deref(), Some("Go"));
        assert_eq!(p.nodes[1].text, None);
    }

    #[test]
    fn parse_screen_tsv_parses_mode_and_rows() {
        let raw = "[mode: V2_SHIZUKU]\nnote:structural-only nodes are omitted from the tree\nscreen:1080x1920 density:420 orientation:PORTRAIT\n--- window:1 type:APPLICATION pkg:com.demo title:Demo layer:0 focused:true ---\nnode_id\tclass\ttext\tdesc\tres_id\tbounds\tflags\nn1\tTextView\tHello\t-\tcom.demo:id/title\t0,0,100,50\ton,clk,ena\nn2\tButton\t-\tGo\t-\t0,60,100,120\ton,clk,ena\nhierarchy:\nn1\n  n2";
        let p = parse_screen_tsv(raw);
        assert_eq!(p.mode.as_deref(), Some("V2_SHIZUKU"));
        assert_eq!(p.rows.len(), 2);
        assert_eq!(p.rows[0].node_id, "n1");
        assert_eq!(p.rows[0].text.as_deref(), Some("Hello"));
        assert_eq!(p.rows[1].desc.as_deref(), Some("Go"));
        assert_eq!(p.rows[1].text, None);
    }

    #[test]
    fn webview_nodes_reduce_reliability() {
        let rows = vec![
            ScreenRow {
                node_id: "1".to_string(),
                class_name: "android.webkit.WebView".to_string(),
                text: None,
                desc: None,
                res_id: None,
                bounds: None,
                flags: None,
            },
            ScreenRow {
                node_id: "2".to_string(),
                class_name: "android.widget.TextView".to_string(),
                text: Some("A".to_string()),
                desc: None,
                res_id: None,
                bounds: None,
                flags: None,
            },
        ];
        assert!(has_webview_nodes(&rows));
        assert_eq!(infer_node_reliability(true, rows.len()), "low");
    }

    #[test]
    fn non_webview_rows_keep_high_reliability() {
        let rows = vec![ScreenRow {
            node_id: "1".to_string(),
            class_name: "android.widget.Button".to_string(),
            text: None,
            desc: None,
            res_id: None,
            bounds: None,
            flags: None,
        }];
        assert!(!has_webview_nodes(&rows));
        assert_eq!(infer_node_reliability(false, rows.len()), "high");
        assert_eq!(infer_node_reliability(false, 0), "low");
    }
}
