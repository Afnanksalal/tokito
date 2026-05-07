//! Persisted build intent per design.

use crate::error::{AppError, AppResult};
use crate::models::DesignIntent;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get(pool: &PgPool, design_id: Uuid) -> AppResult<Option<DesignIntent>> {
    sqlx::query_as::<_, DesignIntent>(
        r#"
        SELECT design_id, goal_text, constraints_json, updated_at
        FROM design_intents
        WHERE design_id = $1
        "#,
    )
    .bind(design_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn upsert(
    pool: &PgPool,
    design_id: Uuid,
    goal_text: &str,
    constraints: Value,
) -> AppResult<DesignIntent> {
    if goal_text.chars().count() > 100_000 {
        return Err(AppError::BadRequest(
            "goal_text exceeds maximum length (100000 unicode chars)".into(),
        ));
    }
    if !constraints.is_object() {
        return Err(AppError::BadRequest(
            "constraints must be a JSON object at the root".into(),
        ));
    }
    sqlx::query_as::<_, DesignIntent>(
        r#"
        INSERT INTO design_intents (design_id, goal_text, constraints_json, updated_at)
        VALUES ($1, $2, $3, now())
        ON CONFLICT (design_id) DO UPDATE SET
          goal_text = EXCLUDED.goal_text,
          constraints_json = EXCLUDED.constraints_json,
          updated_at = now()
        RETURNING design_id, goal_text, constraints_json, updated_at
        "#,
    )
    .bind(design_id)
    .bind(goal_text)
    .bind(constraints)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

/// Default row shape when no intent exists yet (not persisted).
pub fn empty_intent(design_id: Uuid) -> DesignIntent {
    DesignIntent {
        design_id,
        goal_text: String::new(),
        constraints_json: json!({}),
        updated_at: Utc::now(),
    }
}
