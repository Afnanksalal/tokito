//! JWT + API-key authentication helpers.

mod jwt;
pub mod middleware;

pub use jwt::{encode_session_jwt, Claims};
pub use middleware::require_auth;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: String,
}
