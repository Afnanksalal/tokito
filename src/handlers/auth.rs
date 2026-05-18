use crate::auth::encode_session_jwt;
use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::router::AppState;
use crate::store::account;
use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use bcrypt::{hash, verify, DEFAULT_COST};
use hex;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use uuid::Uuid;

const AUTH_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(300);
const AUTH_RATE_LIMIT_MAX_ATTEMPTS: usize = 8;

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyBody {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String,
    pub key_hint: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyListRow {
    pub id: Uuid,
    pub name: String,
    pub key_hint: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> AppResult<Json<AuthTokenResponse>> {
    let email = body.email.trim().to_lowercase();
    enforce_auth_rate_limit(&state, &email)?;
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("valid email required".into()));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    let pw = hash(body.password.as_bytes(), DEFAULT_COST).map_err(|e| AppError::Any(e.into()))?;
    let row = account::create_user(&state.pool, &email, &pw, body.display_name.as_deref()).await?;
    let token = encode_session_jwt(row.id, &row.email, &state.jwt_secret)?;
    Ok(Json(AuthTokenResponse {
        access_token: token,
        token_type: "Bearer",
        user_id: row.id,
        email: row.email,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<AuthTokenResponse>> {
    let email = body.email.trim().to_lowercase();
    enforce_auth_rate_limit(&state, &email)?;
    let Some(row) = account::find_user_by_email(&state.pool, &email).await? else {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    };
    let ok = verify(body.password.as_bytes(), &row.password_hash)
        .map_err(|e| AppError::Any(e.into()))?;
    if !ok {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }
    let token = encode_session_jwt(row.id, &row.email, &state.jwt_secret)?;
    Ok(Json(AuthTokenResponse {
        access_token: token,
        token_type: "Bearer",
        user_id: row.id,
        email: row.email,
    }))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateApiKeyBody>,
) -> AppResult<Json<CreateApiKeyResponse>> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let secret = hex::encode(bytes);
    let full_key = format!("tokito_sk_{secret}");
    let mut hasher = Sha256::new();
    hasher.update(full_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());
    let key_hint = format!("...{}", &secret[secret.len() - 8..]);
    let id = account::insert_api_key(&state.pool, auth.user_id, name, &key_hash, &key_hint).await?;
    Ok(Json(CreateApiKeyResponse {
        id,
        key: full_key,
        key_hint,
    }))
}

fn enforce_auth_rate_limit(state: &AppState, key: &str) -> AppResult<()> {
    let now = Instant::now();
    let bucket = if key.trim().is_empty() {
        "<empty>".to_string()
    } else {
        key.to_string()
    };
    let mut attempts = state
        .auth_attempts
        .lock()
        .map_err(|_| AppError::Any(anyhow::anyhow!("auth rate limiter lock poisoned")))?;
    let entry = attempts.entry(bucket).or_default();
    entry.retain(|t| now.duration_since(*t) <= AUTH_RATE_LIMIT_WINDOW);
    if entry.len() >= AUTH_RATE_LIMIT_MAX_ATTEMPTS {
        return Err(AppError::Forbidden(
            "too many auth attempts; try again later".into(),
        ));
    }
    entry.push(now);
    Ok(())
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<ApiKeyListRow>>> {
    let rows = account::list_api_keys(&state.pool, auth.user_id).await?;
    let out = rows
        .into_iter()
        .map(|(id, name, key_hint, created_at)| ApiKeyListRow {
            id,
            name,
            key_hint,
            created_at,
        })
        .collect();
    Ok(Json(out))
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let n = account::delete_api_key(&state.pool, auth.user_id, id).await?;
    if n == 0 {
        return Err(AppError::NotFound("API key not found".into()));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
