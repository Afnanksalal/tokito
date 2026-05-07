use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::{
    AppendBom, CreateDesign, DesignListParams, PatchDesign, ReplaceBom, ReplaceSchematic,
};
use crate::models::{SchematicSuggestResponse, SchematicValidationReport};
use crate::router::AppState;
use crate::store::{bom, designs as design_store, intent, research, schematic};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

pub async fn list_designs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<DesignListParams>,
) -> AppResult<Json<Vec<crate::models::Design>>> {
    let rows =
        design_store::list_for_user(&state.pool, auth.user_id, params.limit.unwrap_or(50)).await?;
    Ok(Json(rows))
}

pub async fn create_design(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateDesign>,
) -> AppResult<Json<crate::models::Design>> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let row = design_store::create(&state.pool, body, auth.user_id).await?;
    Ok(Json(row))
}

pub async fn get_design(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<crate::models::Design>> {
    let row = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    Ok(Json(row))
}

pub async fn patch_design(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchDesign>,
) -> AppResult<Json<crate::models::Design>> {
    if body
        .name
        .as_deref()
        .map(str::trim)
        .map(|s| s.is_empty())
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest("name cannot be empty".into()));
    }
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let row = design_store::patch(&state.pool, id, body).await?;
    Ok(Json(row))
}

pub async fn export_design(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Query(q): Query<ExportQuery>,
) -> AppResult<Response> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let fmt = q.format.as_deref().unwrap_or("json");
    if fmt == "csv" {
        let csv = bom::csv_export(&state.pool, id).await?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"design-{id}-bom.csv\""),
            )
            .body(Body::from(csv))
            .unwrap())
    } else if fmt == "netlist" {
        let schematic_view = schematic::get_view(&state.pool, id).await?;
        let text = crate::services::netlist::connectivity_text(&schematic_view);
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"design-{id}-netlist.txt\""),
            )
            .body(Body::from(text))
            .unwrap())
    } else {
        let design = design_store::get(&state.pool, id).await?;
        let bom_lines = bom::list_for_design(&state.pool, id).await?;
        let schematic_view = schematic::get_view(&state.pool, id).await?;
        let intent_row = intent::get(&state.pool, id).await?;
        let research_rows = research::list_for_design(&state.pool, id, 128).await?;
        let body = serde_json::json!({
            "design": design,
            "bom": bom_lines,
            "schematic": schematic_view,
            "intent": intent_row,
            "research_artifacts": research_rows,
        });
        Ok(Json(body).into_response())
    }
}

pub async fn get_bom(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<crate::models::BomLine>>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let rows = bom::list_for_design(&state.pool, id).await?;
    Ok(Json(rows))
}

pub async fn put_bom(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplaceBom>,
) -> AppResult<Json<Vec<crate::models::BomLine>>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    for line in &body.lines {
        if line.quantity <= 0.0 {
            return Err(AppError::BadRequest(
                "each line must have quantity > 0".into(),
            ));
        }
    }
    let rows = bom::replace_validated(&state.pool, id, body).await?;
    Ok(Json(rows))
}

pub async fn append_bom(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<AppendBom>,
) -> AppResult<Json<Vec<crate::models::BomLine>>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    for line in &body.lines {
        if line.quantity <= 0.0 {
            return Err(AppError::BadRequest(
                "each line must have quantity > 0".into(),
            ));
        }
    }
    let rows = bom::append_lines(&state.pool, id, &body.lines).await?;
    Ok(Json(rows))
}

pub async fn get_schematic(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<crate::models::SchematicView>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let view = schematic::get_view(&state.pool, id).await?;
    Ok(Json(view))
}

pub async fn put_schematic(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplaceSchematic>,
) -> AppResult<Json<serde_json::Value>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let erc = crate::services::schematic_validate::erc_light(&body);
    schematic::replace(&state.pool, id, body).await?;
    Ok(Json(serde_json::json!({ "ok": true, "erc_warnings": erc })))
}

#[derive(Debug, Deserialize)]
pub struct SuggestSchematicBody {
    pub prompt: String,
}

/// Full pipeline: xAI plan → Firecrawl web search → resolve parts into Postgres → grounded schematic JSON (not auto-saved).
pub async fn suggest_schematic(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<SuggestSchematicBody>,
) -> AppResult<Json<SchematicSuggestResponse>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    let prompt = body.prompt.trim();
    if prompt.is_empty() {
        return Err(AppError::BadRequest("prompt must not be empty".into()));
    }
    let (schematic, erc_warnings) = crate::services::design_pipeline::build_design_from_prompt(
        &state,
        &state.pool,
        auth.user_id,
        id,
        prompt,
    )
    .await?;
    Ok(Json(SchematicSuggestResponse {
        schematic,
        erc_warnings,
    }))
}

pub async fn validate_schematic_payload(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplaceSchematic>,
) -> AppResult<Json<SchematicValidationReport>> {
    let _ = design_store::assert_visible(&state.pool, id, auth.user_id).await?;
    Ok(Json(
        crate::services::schematic_validate::validation_report(&body),
    ))
}
