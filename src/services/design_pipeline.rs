//! Prompt → plan (xAI) → Firecrawl web search → research artifacts →
//! Design pipeline: plan, research, resolve BOM, generate schematic.
//!
//! AI build pipeline: intent → research → BOM → schematic proposal.
//! datasheet-backed catalog population first.

use crate::error::{AppError, AppResult};
use crate::models::{
    BomLineInput, CreateManufacturer, CreatePart, ErcViolation, ReplaceBom, ReplaceSchematic,
};
use crate::router::AppState;
use crate::services::research_pipeline;
use crate::services::schematic_gen::{extract_llm_json_object, suggest_from_prompt};
use crate::services::xai;
use crate::store::account;
use crate::store::{bom, designs, intent, manufacturers, parts, research};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

const MAX_FIRECRAWL_QUERIES: usize = 8;
const FIRECRAWL_LIMIT: u32 = 4;
const MAX_RESEARCH_EXCERPT_CHARS: usize = 48_000;

#[derive(Debug, Deserialize)]
struct PlanPhase {
    #[serde(default)]
    firecrawl_queries: Vec<String>,
    #[serde(default)]
    candidate_parts: Vec<CandidatePart>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CandidatePart {
    mpn: String,
    #[serde(default)]
    manufacturer_name: Option<String>,
    #[serde(default)]
    functional_role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResolvePhase {
    resolved_parts: Vec<ResolvedPart>,
}

#[derive(Debug, Deserialize)]
struct ResolvedPart {
    mpn: String,
    manufacturer_name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    package_name: Option<String>,
    #[serde(default)]
    quantity: f64,
    #[serde(default)]
    datasheet_notes: Option<String>,
}

async fn ensure_manufacturer(pool: &PgPool, name: &str) -> AppResult<Uuid> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "manufacturer_name must not be empty for catalog parts".into(),
        ));
    }
    let slug = manufacturers::slugify(name);
    if let Some(m) = manufacturers::get_by_slug(pool, &slug).await? {
        return Ok(m.id);
    }
    match manufacturers::create(
        pool,
        CreateManufacturer {
            name: name.to_string(),
            slug: None,
        },
    )
    .await
    {
        Ok(m) => Ok(m.id),
        Err(AppError::Conflict(_)) => manufacturers::get_by_slug(pool, &slug)
            .await?
            .map(|m| m.id)
            .ok_or_else(|| AppError::Conflict("manufacturer slug race".into())),
        Err(e) => Err(e),
    }
}

async fn llm_plan(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
) -> AppResult<PlanPhase> {
    account::ensure_llm_quota(pool, user_id, 24_000).await?;
    let design = designs::get(pool, design_id).await?;
    let user_payload = json!({
        "design_name": design.name,
        "design_description": design.description,
        "user_request": user_prompt,
        "task": "Plan datasheet discovery and candidate BOM for this board.",
    });

    let system = r#"You are a hardware design planner. Output ONLY valid JSON (no markdown fences, no commentary) with this shape:
{
  "firecrawl_queries": ["string"],
  "candidate_parts": [
    { "mpn": "LM2596S-5.0/NOPB", "manufacturer_name": "Texas Instruments", "functional_role": "step-down regulator" }
  ]
}
Rules:
- firecrawl_queries: 3–8 concise web search strings that will surface official datasheets (vendor PDFs, product folders). Include key parameters from the user request (Vin/Vout/Iload/topology).
- candidate_parts: concrete manufacturer MPNs you expect on the BOM for this design (not vague descriptions).
- Prefer vendor MPNs over generics when possible."#;

    let body = json!({
        "model": state.agent.default_model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_payload.to_string()},
        ],
        "stream": false,
        "temperature": 0.25,
    });

    let resp = xai::chat_completion(state, body).await?;
    let (pt, ct) = xai::usage_tokens(&resp);
    account::record_llm_usage(pool, user_id, pt, ct).await?;

    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Upstream("xAI missing message content (plan)".into()))?;

    let raw = extract_llm_json_object(content)?;
    serde_json::from_str::<PlanPhase>(&raw)
        .map_err(|e| AppError::BadRequest(format!("invalid plan JSON from model: {e}")))
}

fn dedupe_queries(qs: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for q in qs {
        let t = q.trim().to_string();
        if t.is_empty() {
            continue;
        }
        let k = t.to_lowercase();
        if seen.insert(k) {
            out.push(t);
        }
        if out.len() >= MAX_FIRECRAWL_QUERIES {
            break;
        }
    }
    out
}

async fn llm_resolve_parts(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
    plan: &PlanPhase,
) -> AppResult<Vec<ResolvedPart>> {
    account::ensure_llm_quota(pool, user_id, 48_000).await?;

    let artifacts = research::list_for_design(pool, design_id, 48).await?;
    let mut blob = String::new();
    for a in artifacts {
        blob.push_str("\n---\n");
        if let Some(u) = &a.source_url {
            blob.push_str(&format!("source_url: {u}\n"));
        }
        let slice: String = a.content_text.chars().take(14_000).collect();
        blob.push_str(&slice);
        if blob.len() >= MAX_RESEARCH_EXCERPT_CHARS {
            break;
        }
    }

    let user_payload = json!({
        "user_request": user_prompt,
        "candidate_parts": plan.candidate_parts,
        "research_excerpts_concat": blob,
        "task": "Resolve a BOM grounded in excerpts; normalize manufacturer names and MPNs.",
    });

    let system = r#"You are a hardware librarian. Output ONLY valid JSON (no markdown fences, no commentary):
{
  "resolved_parts": [
    {
      "mpn": "exact orderable MPN",
      "manufacturer_name": "full vendor name",
      "description": "short catalog description",
      "package_name": "optional package string",
      "quantity": 1,
      "datasheet_notes": "pin names, key limits, application hints copied from excerpts when present"
    }
  ]
}
Rules:
- Every resolved part MUST be justified by candidate_parts and/or research_excerpts_concat (no fantasy MPNs).
- quantity ≥ 1 when applicable (default 1).
- Include all distinct catalog lines needed for the topology implied by the user request (regulator, inductor, diodes, input/output caps, feedback divider resistors, etc.).
- If excerpts omit a value, omit fine-grained numbers in datasheet_notes rather than inventing."#;

    let body = json!({
        "model": state.agent.default_model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_payload.to_string()},
        ],
        "stream": false,
        "temperature": 0.2,
    });

    let resp = xai::chat_completion(state, body).await?;
    let (pt, ct) = xai::usage_tokens(&resp);
    account::record_llm_usage(pool, user_id, pt, ct).await?;

    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Upstream("xAI missing message content (resolve)".into()))?;

    let raw = extract_llm_json_object(content)?;
    let parsed: ResolvePhase = serde_json::from_str(&raw)
        .map_err(|e| AppError::BadRequest(format!("invalid resolve JSON from model: {e}")))?;

    if parsed.resolved_parts.is_empty() {
        return Err(AppError::BadRequest(
            "model returned no resolved_parts — add detail to the prompt or check Firecrawl results".into(),
        ));
    }

    Ok(parsed.resolved_parts)
}

async fn upsert_bom_from_resolved(
    pool: &PgPool,
    design_id: Uuid,
    resolved: &[ResolvedPart],
) -> AppResult<()> {
    let mut lines = Vec::with_capacity(resolved.len());
    for (i, rp) in resolved.iter().enumerate() {
        let mpn = rp.mpn.trim();
        if mpn.is_empty() {
            return Err(AppError::BadRequest("resolved part has empty mpn".into()));
        }
        let mfg_id = ensure_manufacturer(pool, &rp.manufacturer_name).await?;

        let part_id =
            if let Some(p) = parts::find_by_manufacturer_and_mpn(pool, mfg_id, mpn).await? {
                p.id
            } else {
                let attrs = json!({
                    "tokito_catalog_pipeline": true,
                    "datasheet_notes": rp.datasheet_notes.clone().unwrap_or_default(),
                });
                match parts::create(
                    pool,
                    CreatePart {
                        manufacturer_id: mfg_id,
                        mpn: mpn.to_string(),
                        description: rp.description.clone(),
                        package_name: rp.package_name.clone(),
                        attributes: Some(attrs),
                    },
                )
                .await
                {
                    Ok(p) => p.id,
                    Err(AppError::Conflict(_)) => {
                        parts::find_by_manufacturer_and_mpn(pool, mfg_id, mpn)
                            .await?
                            .map(|p| p.id)
                            .ok_or_else(|| AppError::Conflict("part insert race".into()))?
                    }
                    Err(e) => return Err(e),
                }
            };

        let qty = if rp.quantity.is_finite() && rp.quantity > 0.0 {
            rp.quantity
        } else {
            1.0
        };

        lines.push(BomLineInput {
            part_id,
            quantity: qty,
            sort_order: i as i32,
            notes: None,
        });
    }

    bom::replace_validated(pool, design_id, ReplaceBom { lines }).await?;
    Ok(())
}

/// Full pipeline: intent → plan → Firecrawl search (ingest) → resolve parts → BOM → schematic (strict `part_id`).
pub async fn build_design_from_prompt(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
) -> AppResult<(ReplaceSchematic, Vec<ErcViolation>)> {
    if state.xai.is_none() {
        return Err(AppError::Unavailable(
            "xAI is not configured; set TOKITO_XAI_API_KEY".into(),
        ));
    }
    if state.firecrawl.is_none() {
        return Err(AppError::Unavailable(
            "Firecrawl is not configured; set TOKITO_FIRECRAWL_API_KEY (required for datasheet search)".into(),
        ));
    }

    let prompt = user_prompt.trim();
    if prompt.is_empty() {
        return Err(AppError::BadRequest("prompt must not be empty".into()));
    }

    intent::upsert(pool, design_id, prompt, json!({})).await?;

    let plan = llm_plan(state, pool, user_id, design_id, prompt).await?;

    let mut queries = dedupe_queries(plan.firecrawl_queries.clone());
    if queries.is_empty() {
        for c in &plan.candidate_parts {
            let m = c.mpn.trim();
            if !m.is_empty() {
                queries.push(format!("{m} datasheet PDF"));
            }
        }
        queries = dedupe_queries(queries);
    }

    if queries.is_empty() {
        return Err(AppError::BadRequest(
            "plan produced no Firecrawl queries or candidate MPNs — refine the prompt".into(),
        ));
    }

    let mut pages_ingested = 0usize;
    for q in queries {
        let ids = research_pipeline::search_web_into_design(
            state,
            pool,
            user_id,
            design_id,
            &q,
            Some(FIRECRAWL_LIMIT),
        )
        .await?;
        pages_ingested += ids.len();
    }

    if pages_ingested == 0 {
        return Err(AppError::BadRequest(
            "Firecrawl returned no ingestible pages — check API key, quotas, or queries.".into(),
        ));
    }

    let resolved = llm_resolve_parts(state, pool, user_id, design_id, prompt, &plan).await?;

    upsert_bom_from_resolved(pool, design_id, &resolved).await?;

    suggest_from_prompt(state, pool, user_id, design_id, prompt, true).await
}
