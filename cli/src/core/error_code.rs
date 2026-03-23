use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Ok,
    Interrupted,
    InvalidParams,
    AuthError,
    NetworkError,
    ServerError,
    AssertionFailed,
    InternalError,
}
