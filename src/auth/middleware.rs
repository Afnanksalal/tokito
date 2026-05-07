use crate::auth::jwt::decode_session_jwt;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::router::AppState;
use crate::store::account;
use axum::extract::Request;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use hex;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub async fn require_auth(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let auth_header = match req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
    {
        Some(h) => h,
        None => {
            return AppError::Unauthorized("missing Authorization header".into()).into_response();
        }
    };
    let bearer = match auth_header.strip_prefix("Bearer ") {
        Some(b) => b.trim(),
        None => {
            return AppError::Unauthorized("expected Bearer token".into()).into_response();
        }
    };

    let auth_user = if bearer.starts_with("tokito_sk_") {
        let mut hasher = Sha256::new();
        hasher.update(bearer.as_bytes());
        let hash = hex::encode(hasher.finalize());
        match account::resolve_api_key(&state.pool, &hash).await {
            Ok(u) => u,
            Err(e) => return e.into_response(),
        }
    } else {
        let claims = match decode_session_jwt(bearer, &state.jwt_secret) {
            Ok(c) => c,
            Err(e) => return e.into_response(),
        };
        let uid = match Uuid::parse_str(&claims.sub) {
            Ok(id) => id,
            Err(_) => {
                return AppError::Unauthorized("invalid token subject".into()).into_response();
            }
        };
        AuthUser {
            user_id: uid,
            email: claims.email,
        }
    };

    req.extensions_mut().insert(auth_user);
    next.run(req).await
}
