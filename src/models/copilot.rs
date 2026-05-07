//! Design intent and research artifacts (copilot grounding).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DesignIntent {
    pub design_id: Uuid,
    pub goal_text: String,
    pub constraints_json: Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PutDesignIntent {
    pub goal_text: String,
    /// JSON object (e.g. `{"vin_v":12,"vout_v":5}`). Omit or null for `{}`.
    #[serde(default)]
    pub constraints: Option<Value>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DesignResearchArtifact {
    pub id: Uuid,
    pub design_id: Uuid,
    pub kind: String,
    pub title: Option<String>,
    pub source_url: Option<String>,
    pub content_text: String,
    pub metadata_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ScrapeResearchUrls {
    #[serde(default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResearchWeb {
    pub query: String,
    #[serde(default)]
    pub limit: Option<u32>,
}
