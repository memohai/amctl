use crate::api::request::{ApiError, ApiErrorKind};
use crate::cli::OutputFormat;
use crate::core::error_code::ErrorCode;
use serde_json::{Value, json};

#[derive(Debug)]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    pub status: Option<u16>,
    pub raw: Option<String>,
    pub details: Option<Value>,
}

impl CommandError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            retryable: false,
            status: None,
            raw: None,
            details: None,
        }
    }

    pub fn invalid_params_with_details(message: impl Into<String>, details: Value) -> Self {
        Self {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            retryable: false,
            status: None,
            raw: None,
            details: Some(details),
        }
    }

    pub fn assertion_failed_with_details(message: impl Into<String>, details: Value) -> Self {
        Self {
            code: ErrorCode::AssertionFailed,
            message: message.into(),
            retryable: false,
            status: None,
            raw: None,
            details: Some(details),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: message.into(),
            retryable: false,
            status: None,
            raw: None,
            details: None,
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

pub type CommandResult = Result<Value, CommandError>;

pub fn into_output(invocation_id: &str, category: &str, op: &str, result: CommandResult) -> Value {
    match result {
        Ok(data) => json!({
            "traceId": invocation_id,
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
                "traceId": invocation_id,
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

pub fn render_output(value: &Value, format: OutputFormat) -> anyhow::Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string(value)?),
        OutputFormat::Text => Ok(render_text(value)),
    }
}

fn render_text(value: &Value) -> String {
    let mut lines = Vec::new();
    for key in ["status", "category", "op", "traceId"] {
        if let Some(v) = value.get(key) {
            push_flattened(&mut lines, key.to_string(), v);
        }
    }
    if let Some(obj) = value.as_object() {
        let mut keys = obj.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            if matches!(key.as_str(), "status" | "category" | "op" | "traceId") {
                continue;
            }
            push_flattened(&mut lines, key.clone(), &obj[&key]);
        }
    } else {
        push_flattened(&mut lines, "value".to_string(), value);
    }
    lines.join("\n")
}

fn push_flattened(lines: &mut Vec<String>, prefix: String, value: &Value) {
    match value {
        Value::Null => lines.push(format!("{prefix}=null")),
        Value::Bool(v) => lines.push(format!("{prefix}={v}")),
        Value::Number(v) => lines.push(format!("{prefix}={v}")),
        Value::String(v) => lines.push(format!("{prefix}={v}")),
        Value::Array(values) => {
            if values.is_empty() {
                lines.push(format!("{prefix}=[]"));
                return;
            }
            for (idx, item) in values.iter().enumerate() {
                push_flattened(lines, format!("{prefix}[{idx}]"), item);
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                lines.push(format!("{prefix}={{}}"));
                return;
            }
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                push_flattened(lines, format!("{prefix}.{key}"), &map[&key]);
            }
        }
    }
}
