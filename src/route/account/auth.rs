use std::fmt::{Display};
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::sync::Arc;
use crate::{epic, serializers, user, util, GlyphState};
use axum::{Form, Json};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use chrono::{TimeDelta, Utc};
use log::{error, warn};
use serde::Deserialize;
use uuid::Uuid;
use crate::auth_manager::{OAuthManager};
use crate::util::UuidString;

#[derive(Debug, PartialEq, Deserialize)]
pub enum GrantType {
    #[serde(rename = "client_credentials")]
    ClientCredentials,
    #[serde(rename = "exchange_code")]
    ExchangeCode,
    #[serde(rename = "password")]
    Password,
    #[serde(rename = "refresh_token")]
    RefreshToken,
}

impl Display for GrantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            GrantType::ClientCredentials => "client_credentials".to_string(),
            GrantType::ExchangeCode => "exchange_code".to_string(),
            GrantType::Password => "password".to_string(),
            GrantType::RefreshToken => "refresh_token".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl FromStr for GrantType {
    type Err = ();

    fn from_str(input: &str) -> Result<GrantType, Self::Err> {
        match input {
            "client_credentials" => Ok(GrantType::ClientCredentials),
            "exchange_code" => Ok(GrantType::ExchangeCode),
            "password" => Ok(GrantType::Password),
            "refresh_token" => Ok(GrantType::RefreshToken),
            _ => Err(()),
        }
    }
}

#[derive(Deserialize)]
pub struct OAuthForm {
    grant_type: GrantType,
    exchange_code: Option<String>,
    refresh_token: Option<String>,
    include_perms: Option<bool>,
    token_type: String,
}

#[derive(serde::Serialize)]
pub struct ClientCredentialsAuthResponse {
    access_token: String,
    expires_in: u16,
    #[serde(serialize_with = "serializers::serialize_datetime")]
    expires_at: chrono::DateTime<Utc>,
    token_type: String,
    client_id: String,
    internal_client: bool,
    client_service: String,
}

#[derive(serde::Serialize)]
pub struct AuthResponse {
    access_token: String,
    expires_in: i64,
    #[serde(serialize_with = "serializers::serialize_datetime")]
    expires_at: chrono::DateTime<Utc>,
    token_type: String,
    refresh_token: String,
    refresh_expires: i64,
    #[serde(serialize_with = "serializers::serialize_datetime")]
    refresh_expires_at: chrono::DateTime<Utc>,
    account_id: String,
    client_id: String,
    internal_client: bool,
    client_service: String,
    #[serde(rename = "displayName")]
    display_name: String,
    app: String,
    in_app_id: String,
    device_id: String,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum OAuthResponse {
    ClientCredentialsResponse(ClientCredentialsAuthResponse),
    ExchangeCodeResponse(AuthResponse),
    PasswordResponse(epic::epic_error::ErrorResponse),
    RefreshTokenResponse(AuthResponse),
    Err(epic::epic_error::ErrorResponse),
}

fn internal_server_err() -> (StatusCode, Option<HeaderMap>, Json<OAuthResponse>) {
    let err = epic::epic_error::make_epic_err(
        "errors.com.epicgames.common.internal_server_error",
        "Something went wrong",
        &*vec![],
        -1,
        StatusCode::INTERNAL_SERVER_ERROR,
    );

    (err.0, err.1, Json(OAuthResponse::Err { 0: err.2 }))
}

pub async fn oauth(
    State(state): State<Arc<GlyphState>>,
    headers: HeaderMap,
    Form(form): Form<OAuthForm>,
) -> (StatusCode, Option<HeaderMap>, Json<OAuthResponse>) {
    if form.token_type != "eg1" {
        warn!("Auth token type was not `eg1`")
    }

    let client_id = match util::extract_client_id(&headers) {
        Ok(val) => val,
        Err(_) => {
            let err = epic::epic_error::make_epic_err(
                "errors.com.epicgames.common.oauth.invalid_client",
                "It appears that your Authorization header may be invalid or not present, please verify that you are sending the correct headers.",
                &*vec![],
                1011,
                StatusCode::BAD_REQUEST,
            );

            return (err.0, err.1, Json(OAuthResponse::Err { 0: err.2 }));
        }
    };

    match form.grant_type {
        GrantType::ClientCredentials => { client_credentials_oauth(state, client_id) }
        GrantType::ExchangeCode => { exchange_code_auth(state, form, client_id).await }
        GrantType::RefreshToken => { refresh_token_auth(state, form, client_id).await }
        GrantType::Password => { password_oauth() }
    }
}

fn client_credentials_oauth(
    state: Arc<GlyphState>,
    client_id: String,
) -> (StatusCode, Option<HeaderMap>, Json<OAuthResponse>) {
    let token = match state.auth_manager.make_client_token(&client_id) {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to make a client token: {}", e);
            return internal_server_err();
        }
    };

    let response = OAuthResponse::ClientCredentialsResponse(ClientCredentialsAuthResponse {
        access_token: format!("eg1~{}", token),
        expires_in: 3600,
        expires_at: Utc::now().add(TimeDelta::hours(4)),
        token_type: "bearer".to_string(),
        client_id,
        internal_client: true,
        client_service: "prod-fn".to_string(),
    });

    (StatusCode::OK, None, Json(response))
}

async fn exchange_code_auth(
    state: Arc<GlyphState>,
    form: OAuthForm,
    client_id: String,
) -> (StatusCode, Option<HeaderMap>, Json<OAuthResponse>) {
    let Some(supplied_code) = &form.exchange_code else {
        let err = epic::epic_error::make_epic_err(
            "errors.com.epicgames.common.oauth.invalid_request",
            "Exchange code is required.",
            &*vec![],
            1013,
            StatusCode::BAD_REQUEST,
        );

        return (err.0, err.1, Json(OAuthResponse::Err { 0: err.2 }));
    };
    let supplied_code = supplied_code.replace("eg1~", "");

    let exchange_code = match OAuthManager::get_exchange_code(&state.mongo, &supplied_code).await {
        Ok(Some(val)) => val,
        Ok(None) => {
            let err = epic::epic_error::make_epic_err(
                "errors.com.epicgames.account.oauth.exchange_code_not_found",
                "Sorry the exchange code you supplied was not found. It is possible that it was no longer valid",
                &*vec![],
                18057,
                StatusCode::UNAUTHORIZED,
            );

            return (err.0, err.1, Json(OAuthResponse::Err { 0: err.2 }));
        }
        Err(e) => {
            error!("Failed to check exchange code validity: {}", e);
            return internal_server_err();
        }
    };

    let user = match user::get_user(&state.mongo, &exchange_code.account_id).await {
        Ok(Some(val)) => val,
        Ok(None) => unreachable!("An account should exist if an exchange code exists"),
        Err(e) => {
            error!("Failed to get a user: {}", e);
            return internal_server_err();
        }
    };

    let device_id = Uuid::new_v4().to_stripped_string();
    let access_token = match state.auth_manager.make_access_token(&state.mongo, &user, &client_id, &device_id, &form.grant_type, None).await {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to make an access token: {}", e);
            return internal_server_err();
        }
    };
    let refresh_token = match state.auth_manager.make_refresh_token(&state.mongo, &user, &client_id, &device_id, &form.grant_type, None).await {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to make a refresh token: {}", e);
            return internal_server_err();
        }
    };

    let response = OAuthResponse::ExchangeCodeResponse(AuthResponse {
        access_token: format!("eg1~{}", access_token.token.clone()),
        expires_in: access_token.expires_at.sub(Utc::now()).num_seconds(),
        expires_at: access_token.expires_at,
        token_type: "bearer".to_string(),
        refresh_token: format!("eg1~{}", refresh_token.token.clone()),
        refresh_expires: refresh_token.expires_at.sub(Utc::now()).num_seconds(),
        refresh_expires_at: refresh_token.expires_at,
        account_id: user.account_id.to_stripped_string(),
        client_id,
        internal_client: true,
        client_service: "prod-fn".to_string(),
        display_name: user.display_name,
        app: "fortnite".to_string(),
        in_app_id: user.account_id.to_stripped_string(),
        device_id,
    });

    (StatusCode::OK, None, Json(response))
}

async fn refresh_token_auth(
    state: Arc<GlyphState>,
    form: OAuthForm,
    client_id: String,
) -> (StatusCode, Option<HeaderMap>, Json<OAuthResponse>) {
    let Some(supplied_code) = form.refresh_token else {
        let err = epic::epic_error::make_epic_err(
            "errors.com.epicgames.common.oauth.invalid_request",
            "Refresh token is required.",
            &*vec![],
            1013,
            StatusCode::BAD_REQUEST,
        );

        return (err.0, err.1, Json(OAuthResponse::Err { 0: err.2 }));
    };

    let supplied_token = match OAuthManager::get_refresh_token(&state.mongo, &supplied_code.replace("eg1~", "")).await {
        Ok(Some(val)) => val,
        Ok(None) => {
            let err = epic::epic_error::make_epic_err(
                "errors.com.epicgames.account.auth_token.invalid_refresh_token",
                "Sorry the refresh token '${refresh_token}' is invalid",
                &*vec![],
                18036,
                StatusCode::BAD_REQUEST,
            );

            return (err.0, err.1, Json(OAuthResponse::Err { 0: err.2 }));
        }
        Err(e) => {
            error!("Failed to check refresh token validity: {}", e);
            return internal_server_err();
        }
    };

    let user = match user::get_user(&state.mongo, &supplied_token.account_id).await {
        Ok(Some(val)) => val,
        Ok(None) => unreachable!("An account should exist if an exchange code exists"),
        Err(e) => {
            error!("Failed to get a user: {}", e);
            return internal_server_err();
        }
    };

    let device_id = Uuid::new_v4().to_stripped_string();
    let access_token = match state.auth_manager.make_access_token(&state.mongo, &user, &client_id, &device_id, &form.grant_type, None).await {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to make an access token: {}", e);
            return internal_server_err();
        }
    };
    let refresh_token = match state.auth_manager.make_refresh_token(&state.mongo, &user, &client_id, &device_id, &form.grant_type, None).await {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to make a refresh token: {}", e);
            return internal_server_err();
        }
    };

    let response = OAuthResponse::RefreshTokenResponse(AuthResponse {
        access_token: format!("eg1~{}", access_token.token.clone()),
        expires_in: access_token.expires_at.sub(Utc::now()).num_seconds(),
        expires_at: access_token.expires_at,
        token_type: "bearer".to_string(),
        refresh_token: format!("eg1~{}", refresh_token.token.clone()),
        refresh_expires: refresh_token.expires_at.sub(Utc::now()).num_seconds(),
        refresh_expires_at: refresh_token.expires_at,
        account_id: user.account_id.to_stripped_string(),
        client_id,
        internal_client: true,
        client_service: "prod-fn".to_string(),
        display_name: user.display_name,
        app: "fortnite".to_string(),
        in_app_id: user.account_id.to_stripped_string(),
        device_id,
    });

    (StatusCode::OK, None, Json(response))
}

fn password_oauth() -> (StatusCode, Option<HeaderMap>, Json<OAuthResponse>) {
    let err = epic::epic_error::make_epic_err(
        "errors.com.epicgames.common.oauth.unsupported_grant_type",
        "Sorry password auth is not supported. Try logging in again from the Glyph launcher.",
        &*vec![],
        1016,
        StatusCode::UNAUTHORIZED,
    );

    (err.0, err.1, Json(OAuthResponse::PasswordResponse { 0: err.2 }))
}
