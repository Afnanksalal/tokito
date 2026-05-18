//! Natural-language → schematic JSON (ReplaceSchematic) via xAI.

use crate::error::{AppError, AppResult};
use crate::models::{ErcViolation, ReplaceSchematic};
use crate::router::AppState;
use crate::services::schematic_validate::{erc_light, validate_topology};
use crate::services::xai;
use crate::store::account;
use crate::store::{bom, designs, intent, parts, research};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

async fn assemble_grounding_context(
    pool: &PgPool,
    design_id: Uuid,
    user_prompt: &str,
) -> AppResult<Value> {
    let design = designs::get(pool, design_id).await?;
    let bom = bom::list_for_design(pool, design_id).await?;
    let mut part_ids: Vec<Uuid> = bom.iter().map(|l| l.part_id).collect();
    part_ids.sort_unstable();
    part_ids.dedup();
    let parts_map = parts::get_by_ids(pool, &part_ids).await?;
    let bom_enriched: Vec<Value> = bom
        .iter()
        .map(|line| {
            let p = parts_map.get(&line.part_id);
            json!({
                "part_id": line.part_id,
                "quantity": line.quantity,
                "mpn": p.map(|x| &x.mpn),
                "description": p.and_then(|x| x.description.as_ref()),
                "package_name": p.and_then(|x| x.package_name.as_ref()),
            })
        })
        .collect();

    let intent_json = match intent::get(pool, design_id).await? {
        Some(i) => json!({ "goal_text": i.goal_text, "constraints": i.constraints_json }),
        None => json!({ "goal_text": "", "constraints": {} }),
    };

    let artifacts = research::list_for_design(pool, design_id, 48).await?;
    let mpn_hints: Vec<&str> = bom
        .iter()
        .filter_map(|l| parts_map.get(&l.part_id).map(|p| p.mpn.as_str()))
        .collect();
    let excerpts = crate::services::research_retrieval::excerpts_for_prompt(
        &artifacts,
        user_prompt,
        &mpn_hints,
        32_000,
    );

    Ok(json!({
        "design_name": design.name,
        "design_description": design.description,
        "build_intent": intent_json,
        "bom_lines": bom_enriched,
        "research_excerpts_ranked": excerpts,
        "user_request": user_prompt,
    }))
}

/// Parses `{ ... }` from model output (handles ```json fences).
pub fn extract_llm_json_object(content: &str) -> AppResult<String> {
    let c = content.trim();
    if let Some(i) = c.find("```json") {
        let rest = &c[i + 7..];
        if let Some(j) = rest.find("```") {
            return Ok(rest[..j].trim().to_string());
        }
    }
    if let Some(i) = c.find("```") {
        let rest = &c[i + 3..];
        if let Some(j) = rest.find("```") {
            let mid = rest[..j].trim();
            if mid.starts_with('{') {
                return Ok(mid.to_string());
            }
        }
    }
    if let (Some(i), Some(j)) = (c.find('{'), c.rfind('}')) {
        if j >= i {
            return Ok(c[i..=j].to_string());
        }
    }
    Err(AppError::BadRequest(
        "model response did not contain JSON object".into(),
    ))
}

/// Single LLM call; returns ReplaceSchematic (not persisted).
///
/// When `enforce_catalog_part_ids` is true (catalog pipeline), every instance must carry a
/// non-null `part_id` present on the design BOM.
pub async fn suggest_from_prompt(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    design_id: Uuid,
    user_prompt: &str,
    enforce_catalog_part_ids: bool,
) -> AppResult<(ReplaceSchematic, Vec<ErcViolation>)> {
    account::ensure_llm_quota(pool, user_id, 96_000).await?;

    let bom = bom::list_for_design(pool, design_id).await?;
    if enforce_catalog_part_ids && bom.is_empty() {
        return Err(AppError::BadRequest(
            "BOM must be populated before strict schematic generation".into(),
        ));
    }

    let ctx = assemble_grounding_context(pool, design_id, user_prompt).await?;

    let system = if enforce_catalog_part_ids {
        r#"You are Tokito's schematic authoring assistant.
The user JSON includes build_intent, bom_lines (authoritative catalog lines for this design),
and research_excerpts_newest_first (markdown/text from datasheets or web pages).

Grounding rules:
- bom_lines lists the ONLY catalog parts you may place; map each placed symbol to the correct line by MPN/description/role.
- Prefer pin naming and electrical hints from research_excerpts when they match those parts.
- Never invent exact timing/LC numbers unless excerpts contain them.

Return ONLY valid JSON (no markdown fences, no commentary) with this exact shape:
{
  "instances": [
    { "ref_des": "U1", "part_id": "paste-uuid-from-bom_lines", "position": { "x": 120, "y": 140 }, "rotation": 0 }
  ],
  "nets": [ { "name": "VCC" }, { "name": "GND" } ],
  "pins": [
    { "instance_ref": "U1", "pin_name": "u1_vin", "net_name": "VIN" }
  ]
}
Rules:
- ref_des must be unique (standard EDA). Prefixes: **R** resistor, **C** capacitor, **L** inductor, **D** diode, **Q** transistor/FET, **U** IC/regulator/controller.
- Every net_name used in pins must appear in nets.
- Every instance_ref in pins must appear in instances.ref_des.
- **part_id is REQUIRED on every instance** — copy the UUID string exactly from bom_lines[].part_id (never null).
- Positions on a 40px grid; rotation in degrees.
- Provide enough pins for intended connectivity."#
    } else {
        r#"You are Tokito's schematic authoring assistant.
The user JSON includes build_intent (what they want to build), bom_lines (catalog-backed parts already chosen),
and research_excerpts_newest_first (markdown/text scraped from datasheets or web pages).

Grounding rules:
- Prefer facts from research_excerpts when they mention pin names, limits, typical applications, or reference designs.
- Artifacts may come from URL scrapes (`firecrawl_scrape`) or query-based web search (`firecrawl_search`).
- Respect numeric constraints in build_intent.constraints when present (voltages, currents, topology hints).
- When research_excerpts are empty or irrelevant, still output a plausible schematic using conservative defaults and standard net naming for the topology implied by user_request + build_intent.
- Never invent exact datasheet timing/LC numbers unless they appear in research_excerpts; otherwise omit fine-grained values.

Return ONLY valid JSON (no markdown fences, no commentary) with this exact shape:
{
  "instances": [
    { "ref_des": "U1", "part_id": null, "position": { "x": 120, "y": 140 }, "rotation": 0 }
  ],
  "nets": [ { "name": "VCC" }, { "name": "GND" } ],
  "pins": [
    { "instance_ref": "U1", "pin_name": "u1_vcc", "net_name": "VCC" }
  ]
}
Rules:
- ref_des must be unique (e.g. U1,R3,C2). Use standard prefixes so the canvas can render symbols: **R** resistor, **C** capacitor, **L** inductor, **D** diode, **Q** transistor/FET, **U** IC/regulator/controller.
- Every net_name used in pins must appear in nets.
- Every instance_ref in pins must appear in instances.ref_des.
- part_id must be null OR one of the UUID strings listed in bom_lines[].part_id when that component is used.
- Positions should land on a 40px grid (e.g. 80,120,160…); rotation is degrees.
- Provide enough pins so connectivity matches the user's circuit intent (multiple pins may share a net).
Keep nets reasonably named (VIN,VOUT,VCC,GND,SW,BST,FB,SCL,SDA,...)."#
    };

    let body = json!({
        "model": state.agent.default_model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": ctx.to_string()},
        ],
        "stream": false,
        "temperature": 0.2,
    });

    let resp = xai::chat_completion(state, body).await?;
    let (pt, ct) = xai::usage_tokens(&resp);
    account::record_llm_usage(pool, user_id, pt, ct).await?;

    let choice = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| AppError::Upstream("xAI missing choices".into()))?;
    let content = choice
        .pointer("/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Upstream("xAI missing message content".into()))?;

    let json_raw = extract_llm_json_object(content)?;
    let parsed: ReplaceSchematic = serde_json::from_str(&json_raw)
        .map_err(|e| AppError::BadRequest(format!("invalid schematic JSON: {e}")))?;
    if enforce_catalog_part_ids {
        let bom_ids: HashSet<Uuid> = bom.iter().map(|l| l.part_id).collect();
        for inst in &parsed.instances {
            let pid = inst.part_id.ok_or_else(|| {
                AppError::BadRequest(format!(
                    "pipeline schematic requires part_id on instance {}",
                    inst.ref_des
                ))
            })?;
            if !bom_ids.contains(&pid) {
                return Err(AppError::BadRequest(format!(
                    "instance {} part_id {} is not on the design BOM",
                    inst.ref_des, pid
                )));
            }
        }
    }
    validate_topology(&parsed)?;
    let erc = erc_light(&parsed);
    Ok((parsed, erc))
}
