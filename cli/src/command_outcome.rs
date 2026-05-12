use crate::cli::ObserveCommands;
use crate::memory::FingerprintRow;
use crate::output::{CommandError, CommandResult, into_output};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub category: &'static str,
    pub op: &'static str,
    pub result: CommandResult,
    pub observation: Option<ObservationUpdate>,
}

impl CommandOutcome {
    pub fn new(category: &'static str, op: &'static str, result: CommandResult) -> Self {
        Self {
            category,
            op,
            result,
            observation: None,
        }
    }

    pub fn with_observation(mut self, observation: Option<ObservationUpdate>) -> Self {
        self.observation = observation;
        self
    }

    pub fn render(&self, invocation_id: &str) -> Value {
        into_output(invocation_id, self.category, self.op, self.result.clone())
    }

    pub fn status(&self) -> OutcomeStatus {
        match &self.result {
            Ok(_) => OutcomeStatus::Ok,
            Err(err) if err.code == crate::core::error_code::ErrorCode::Interrupted => {
                OutcomeStatus::Interrupted
            }
            Err(_) => OutcomeStatus::Failed,
        }
    }

    pub fn data(&self) -> Option<&Value> {
        self.result.as_ref().ok()
    }

    pub fn error(&self) -> Option<&CommandError> {
        self.result.as_ref().err()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeStatus {
    Ok,
    Failed,
    Interrupted,
}

impl OutcomeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OutcomeStatus::Ok => "ok",
            OutcomeStatus::Failed => "failed",
            OutcomeStatus::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ObservationUpdate {
    Top(TopObservation),
    Screen(PageObservation),
    Refs(PageObservation),
    Page(PageObservation),
}

#[derive(Debug, Clone)]
pub struct TopObservation {
    pub activity: String,
}

#[derive(Debug, Clone)]
pub struct PageObservation {
    pub activity: Option<String>,
    pub mode: String,
    pub has_webview: bool,
    pub node_reliability: String,
    pub ref_version: Option<u64>,
    pub fingerprint_rows: Option<FingerprintRows>,
}

#[derive(Debug, Clone)]
pub enum FingerprintRows {
    Screen(Vec<FingerprintRowOwned>),
    Refs(Vec<FingerprintRowOwned>),
}

impl FingerprintRows {
    pub fn as_borrowed(&self) -> Vec<FingerprintRow<'_>> {
        match self {
            FingerprintRows::Screen(rows) | FingerprintRows::Refs(rows) => rows
                .iter()
                .map(|row| FingerprintRow {
                    class_name: row.class_name.as_deref(),
                    res_id: row.res_id.as_deref(),
                })
                .collect(),
        }
    }

    pub fn source(&self) -> &'static str {
        match self {
            FingerprintRows::Screen(_) => "screen",
            FingerprintRows::Refs(_) => "refs",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FingerprintRowOwned {
    pub class_name: Option<String>,
    pub res_id: Option<String>,
}

impl FingerprintRowOwned {
    pub fn new(class_name: Option<String>, res_id: Option<String>) -> Option<Self> {
        if class_name.is_none() && res_id.is_none() {
            None
        } else {
            Some(Self { class_name, res_id })
        }
    }
}

impl ObservationUpdate {
    pub fn from_observe_data(command: &ObserveCommands, data: &Value) -> Option<Self> {
        match command {
            ObserveCommands::Top => {
                data.get("topActivity")
                    .and_then(Value::as_str)
                    .map(|activity| {
                        ObservationUpdate::Top(TopObservation {
                            activity: activity.to_string(),
                        })
                    })
            }
            ObserveCommands::Screen { .. } => Some(ObservationUpdate::Screen(PageObservation {
                activity: data
                    .get("topActivity")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                mode: data
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                has_webview: data
                    .get("hasWebView")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                node_reliability: data
                    .get("nodeReliability")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                ref_version: None,
                fingerprint_rows: data
                    .get("rows")
                    .and_then(Value::as_array)
                    .or_else(|| data.get("fingerprintRows").and_then(Value::as_array))
                    .map(|rows| FingerprintRows::Screen(fingerprint_rows_from_screen(rows))),
            })),
            ObserveCommands::Refs { .. } => Some(ObservationUpdate::Refs(PageObservation {
                activity: data
                    .get("topActivity")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                mode: data
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                has_webview: data
                    .get("hasWebView")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                node_reliability: data
                    .get("nodeReliability")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                ref_version: data.get("refVersion").and_then(Value::as_u64),
                fingerprint_rows: Some(FingerprintRows::Refs(fingerprint_rows_from_refs(
                    data.get("rows")
                        .and_then(Value::as_array)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ))),
            })),
            ObserveCommands::Page { .. } => {
                let activity = data
                    .get("topActivity")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
                let screen_rows = data
                    .get("screen")
                    .and_then(|s| s.get("rows").or_else(|| s.get("fingerprintRows")))
                    .and_then(Value::as_array);
                let refs_rows = data
                    .get("refs")
                    .and_then(|r| r.get("rows"))
                    .and_then(Value::as_array);
                let fingerprint_rows = screen_rows
                    .map(|rows| FingerprintRows::Screen(fingerprint_rows_from_screen(rows)))
                    .or_else(|| {
                        refs_rows
                            .map(|rows| FingerprintRows::Refs(fingerprint_rows_from_refs(rows)))
                    });

                Some(ObservationUpdate::Page(PageObservation {
                    activity,
                    mode: data
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    has_webview: data
                        .get("hasWebView")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    node_reliability: data
                        .get("nodeReliability")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    ref_version: data
                        .get("refs")
                        .and_then(|r| r.get("refVersion"))
                        .and_then(Value::as_u64),
                    fingerprint_rows,
                }))
            }
            ObserveCommands::Overlay { .. } | ObserveCommands::Screenshot { .. } => None,
        }
    }
}

fn fingerprint_rows_from_screen(rows: &[Value]) -> Vec<FingerprintRowOwned> {
    rows.iter()
        .filter_map(|row| {
            FingerprintRowOwned::new(
                row.get("class_name")
                    .or_else(|| row.get("class"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                row.get("res_id")
                    .or_else(|| row.get("resId"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            )
        })
        .collect()
}

fn fingerprint_rows_from_refs(rows: &[Value]) -> Vec<FingerprintRowOwned> {
    rows.iter()
        .filter_map(|row| {
            FingerprintRowOwned::new(
                row.get("class").and_then(Value::as_str).map(str::to_string),
                row.get("resId").and_then(Value::as_str).map(str::to_string),
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct RecordingInput<'a> {
    pub cli: &'a crate::cli::Cli,
    pub outcome: &'a CommandOutcome,
    pub duration_ms: u128,
}

pub fn output_error_value(error: &CommandError) -> Value {
    json!({
        "code": error.code,
        "message": error.message,
        "retryable": error.retryable,
        "status": error.status,
        "raw": error.raw,
        "details": error.details
    })
}
