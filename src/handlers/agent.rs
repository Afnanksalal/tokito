use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::router::AppState;
use crate::services::agent::{run as run_agent_loop, AgentRunInput};
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AgentRunBody {
    #[serde(default)]
    pub messages: Vec<Value>,
    pub design_id: Option<Uuid>,
    #[serde(default)]
    pub model: Option<String>,
}

pub async fn run_agent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<AgentRunBody>,
) -> AppResult<Json<Value>> {
    let out = run_agent_loop(
        &state,
        auth,
        AgentRunInput {
            messages: body.messages,
            design_id: body.design_id,
            model: body.model,
        },
    )
    .await?;
    Ok(Json(out))
}
