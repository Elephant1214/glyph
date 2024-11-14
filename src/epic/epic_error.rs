use axum::http::{HeaderMap, StatusCode};

#[derive(serde::Serialize, Debug)]
pub struct ErrorResponse {
    #[serde(rename = "errorCode")]
    error_code: String,
    #[serde(rename = "errorMessage")]
    error_msg: String,
    #[serde(rename = "messageVars")]
    message_vars: Vec<String>,
    #[serde(rename = "numericErrorCode")]
    numeric_error_code: i16,
    #[serde(rename = "originatingService")]
    originating_service: String,
    intent: String,
}

pub fn make_epic_err(
    error_code: &str,
    error_msg: &str,
    message_vars: &[String],
    numeric_error_code: i16,
    status_code: StatusCode,
) -> (StatusCode, Option<HeaderMap>, ErrorResponse) {
    let mut headers = HeaderMap::new();
    headers.insert("X-Epic-Error-Name", error_code.parse().unwrap());
    headers.insert("X-Epic-Error-Code", numeric_error_code.to_string().parse().unwrap());

    let error_response = ErrorResponse {
        error_code: error_code.into(),
        error_msg: error_msg.into(),
        message_vars: message_vars.into(),
        numeric_error_code,
        originating_service: "any".into(),
        intent: "prod".into(),
    };

    (status_code, Some(headers), error_response)
}
