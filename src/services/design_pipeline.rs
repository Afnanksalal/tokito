//! AI build: plan → Firecrawl research → BOM resolution → schematic proposal.

use crate::error::{AppError, AppResult};
use crate::models::{
    BomLineInput, CreateManufacturer, CreatePart, ErcViolation, ReplaceBom, ReplaceSchematic,
};
use crate::router::AppState;
use crate::services::llm;
use crate::services::research_pipeline;
use crate::services::schematic_gen::{extract_llm_json_object, suggest_from_prompt};
use crate::store::{bom, designs, intent, manufacturers, parts, research};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

fn check_cancel(cancel: Option<&Arc<AtomicBool>>) -> AppResult<()> {
    if cancel.is_some_and(|f| f.load(Ordering::Relaxed)) {
        return Err(AppError::BadRequest("build cancelled".into()));
    }
    Ok(())
}

const MAX_FIRECRAWL_QUERIES: usize = 8;
const FIRECRAWL_LIMIT: u32 = 4;
const MAX_RESEARCH_EXCERPT_CHARS: usize = 48_000;

#[derive(Debug, Clone)]
pub struct BuildPipelineOutcome {
    pub schematic: ReplaceSchematic,
    pub erc_warnings: Vec<ErcViolation>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPlan {
    #[serde(default)]
    pub firecrawl_queries: Vec<String>,
    #[serde(default)]
    pub candidate_parts: Vec<CandidatePart>,
}

#[derive(Debug, Deserialize)]
struct PlanPhase {
    #[serde(default)]
    firecrawl_queries: Vec<String>,
    #[serde(default)]
    candidate_parts: Vec<CandidatePart>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidatePart {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPart {
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

    let resp = llm::metered_chat_completion(state, pool, user_id, body, 24_000).await?;

    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Upstream("AI response missing message content (plan)".into()))?;

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
    let artifacts = research::list_for_design(pool, design_id, 48).await?;
    let mpn_hints: Vec<&str> = plan
        .candidate_parts
        .iter()
        .map(|c| c.mpn.as_str())
        .collect();
    let excerpts = crate::services::research_retrieval::excerpts_for_prompt(
        &artifacts,
        user_prompt,
        &mpn_hints,
        MAX_RESEARCH_EXCERPT_CHARS,
    );
    let blob = serde_json::to_string_pretty(&excerpts).unwrap_or_default();

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

    let resp = llm::metered_chat_completion(state, pool, user_id, body, 48_000).await?;

    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::Upstream("AI response missing message content (resolve)".into())
        })?;

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

fn resolve_from_candidates(plan: &PlanPhase) -> Vec<ResolvedPart> {
    plan.candidate_parts
        .iter()
        .filter(|c| !c.mpn.trim().is_empty())
        .map(|c| ResolvedPart {
            mpn: c.mpn.trim().to_string(),
            manufacturer_name: c
                .manufacturer_name
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
            description: c.functional_role.clone(),
            package_name: None,
            quantity: 1.0,
            datasheet_notes: None,
        })
        .collect()
}

/// Full pipeline: intent → plan → Firecrawl search (ingest) → resolve parts → BOM → schematic (strict `part_id`).
pub async fn build_design_from_prompt(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
    incremental_research: bool,
    cancel: Option<Arc<AtomicBool>>,
) -> AppResult<BuildPipelineOutcome> {
    check_cancel(cancel.as_ref())?;
    if state.llm.is_none() {
        return Err(AppError::Unavailable(
            "AI provider is not configured".into(),
        ));
    }
    if state.firecrawl.is_none() {
        return Err(AppError::Unavailable(format!(
            "{} (required for datasheet search)",
            crate::user_messages::FIRECRAWL_NOT_CONFIGURED
        )));
    }

    let prompt = user_prompt.trim();
    if prompt.is_empty() {
        return Err(AppError::BadRequest("prompt must not be empty".into()));
    }

    intent::upsert(pool, design_id, prompt, json!({})).await?;
    check_cancel(cancel.as_ref())?;

    let plan = llm_plan(state, pool, user_id, design_id, prompt).await?;
    check_cancel(cancel.as_ref())?;

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

    let mut warnings = Vec::new();
    let mut pages_ingested = 0usize;
    for q in &queries {
        if incremental_research && research::has_search_for_query(pool, design_id, q).await? {
            warnings.push(format!("Skipped Firecrawl (cached): {q}"));
            continue;
        }
        check_cancel(cancel.as_ref())?;
        match research_pipeline::search_web_into_design(
            state,
            pool,
            user_id,
            design_id,
            q,
            Some(FIRECRAWL_LIMIT),
        )
        .await
        {
            Ok(ids) => pages_ingested += ids.len(),
            Err(e) => warnings.push(format!("Firecrawl query failed ({q}): {e}")),
        }
    }
    check_cancel(cancel.as_ref())?;

    let resolved = if pages_ingested == 0 {
        if plan.candidate_parts.is_empty() {
            return Err(AppError::BadRequest(
                "Firecrawl returned no pages and the plan had no candidate parts — refine the prompt or check API keys.".into(),
            ));
        }
        warnings.push("No new research pages; BOM built from planner candidate parts only.".into());
        resolve_from_candidates(&plan)
    } else {
        match llm_resolve_parts(state, pool, user_id, design_id, prompt, &plan).await {
            Ok(r) if !r.is_empty() => r,
            Ok(_) => {
                warnings.push("Model returned no resolved parts; using planner candidates.".into());
                resolve_from_candidates(&plan)
            }
            Err(e) => {
                warnings.push(format!(
                    "Part resolve failed ({e}); using planner candidates."
                ));
                resolve_from_candidates(&plan)
            }
        }
    };

    upsert_bom_from_resolved(pool, design_id, &resolved).await?;
    check_cancel(cancel.as_ref())?;

    let (schematic, erc) =
        suggest_from_prompt(state, pool, user_id, design_id, prompt, true).await?;
    check_cancel(cancel.as_ref())?;
    Ok(BuildPipelineOutcome {
        schematic,
        erc_warnings: erc,
        warnings,
    })
}

/// Pipeline stage: LLM plan (agent tool `plan_build`).
pub async fn plan_build(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
) -> AppResult<BuildPlan> {
    let p = llm_plan(state, pool, user_id, design_id, user_prompt).await?;
    Ok(BuildPlan {
        firecrawl_queries: p.firecrawl_queries,
        candidate_parts: p.candidate_parts,
    })
}

/// Pipeline stage: Firecrawl ingest for plan queries.
pub async fn run_research_for_plan(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    plan: &BuildPlan,
    incremental: bool,
) -> AppResult<(usize, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut pages = 0usize;
    let queries = dedupe_queries(plan.firecrawl_queries.clone());
    for q in &queries {
        if incremental && research::has_search_for_query(pool, design_id, q).await? {
            warnings.push(format!("Skipped Firecrawl (cached): {q}"));
            continue;
        }
        match research_pipeline::search_web_into_design(
            state,
            pool,
            user_id,
            design_id,
            q,
            Some(FIRECRAWL_LIMIT),
        )
        .await
        {
            Ok(ids) => pages += ids.len(),
            Err(e) => warnings.push(format!("Firecrawl query failed ({q}): {e}")),
        }
    }
    Ok((pages, warnings))
}

/// Pipeline stage: resolve BOM parts from plan + research.
pub async fn resolve_bom_parts(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
    plan: &BuildPlan,
    pages_ingested: usize,
) -> AppResult<Vec<ResolvedPart>> {
    let plan_phase = PlanPhase {
        firecrawl_queries: plan.firecrawl_queries.clone(),
        candidate_parts: plan.candidate_parts.clone(),
    };
    if pages_ingested == 0 {
        if plan.candidate_parts.is_empty() {
            return Err(AppError::BadRequest(
                "no research pages and no candidate parts".into(),
            ));
        }
        return Ok(resolve_from_candidates(&plan_phase));
    }
    match llm_resolve_parts(state, pool, user_id, design_id, user_prompt, &plan_phase).await {
        Ok(r) if !r.is_empty() => Ok(r),
        Ok(_) => Ok(resolve_from_candidates(&plan_phase)),
        Err(_) => Ok(resolve_from_candidates(&plan_phase)),
    }
}

/// Pipeline stage: schematic suggestion.
pub async fn suggest_schematic_stage(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
) -> AppResult<(ReplaceSchematic, Vec<ErcViolation>)> {
    suggest_from_prompt(state, pool, user_id, design_id, user_prompt, true).await
}

/// Compare DB BOM vs schematic rollup (agent tool `reconcile_bom`).
pub async fn reconcile_bom(pool: &PgPool, design_id: Uuid) -> AppResult<serde_json::Value> {
    let lines = bom::list_for_design(pool, design_id).await?;
    let doc = crate::store::schematic_document::get(pool, design_id)
        .await?
        .unwrap_or_else(crate::models::SchematicDocument::empty);
    let (body, _) = doc.to_replace_schematic();
    let summary = crate::services::bom_sync::diff_summary(&lines, &body);
    serde_json::to_value(summary).map_err(|e| AppError::Any(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_from_candidates_when_firecrawl_returns_no_pages() {
        let plan: PlanPhase = serde_json::from_str(
            r#"{"candidate_parts":[{"mpn":"TPS54302","manufacturer_name":"TI","functional_role":"buck"}]}"#,
        )
        .unwrap();
        let resolved = resolve_from_candidates(&plan);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].mpn, "TPS54302");
        assert_eq!(resolved[0].manufacturer_name, "TI");
    }
}
