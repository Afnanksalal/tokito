//! Multi-step tool loop with cost caps and structured logging.

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::router::AppState;
use crate::services::{firecrawl, lcsc, llm, nexar};
use crate::store::{account, bom, designs, offers, parts};
use serde_json::{json, Value};
use uuid::Uuid;

pub struct AgentRunInput {
    pub messages: Vec<Value>,
    pub design_id: Option<Uuid>,
    pub model: Option<String>,
}

fn tools_schema() -> Value {
    json!([
      {
        "type": "function",
        "function": {
          "name": "search_parts",
          "description": "Search the local Tokito parts catalog by MPN or description substring.",
          "parameters": {
            "type": "object",
            "properties": { "q": { "type": "string" }, "limit": { "type": "integer" } },
            "required": ["q"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "scrape_url",
          "description": "Fetch and extract readable content from a URL via Firecrawl (counts toward scrape quota).",
          "parameters": {
            "type": "object",
            "properties": { "url": { "type": "string" }, "formats": { "type": "array", "items": { "type": "string" } } },
            "required": ["url"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "sync_part_offers",
          "description": "Refresh distributor offers for a part. Sources: nexar (Octopart supply graph), lcsc (best-effort web), octopart (alias of nexar).",
          "parameters": {
            "type": "object",
            "properties": {
              "part_id": { "type": "string" },
              "sources": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["part_id"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "get_design_bom",
          "description": "Return BOM lines for a design.",
          "parameters": {
            "type": "object",
            "properties": { "design_id": { "type": "string" } },
            "required": ["design_id"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "plan_build",
          "description": "LLM planning phase: Firecrawl queries and candidate parts for a design.",
          "parameters": {
            "type": "object",
            "properties": {
              "design_id": { "type": "string" },
              "prompt": { "type": "string" }
            },
            "required": ["design_id", "prompt"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "resolve_bom",
          "description": "Resolve BOM parts from plan JSON and ingested research.",
          "parameters": {
            "type": "object",
            "properties": {
              "design_id": { "type": "string" },
              "prompt": { "type": "string" },
              "plan": { "type": "object" },
              "pages_ingested": { "type": "integer" }
            },
            "required": ["design_id", "prompt", "plan"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "suggest_schematic",
          "description": "Propose a schematic ReplaceSchematic for the design (not auto-saved).",
          "parameters": {
            "type": "object",
            "properties": {
              "design_id": { "type": "string" },
              "prompt": { "type": "string" }
            },
            "required": ["design_id", "prompt"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "reconcile_bom",
          "description": "Compare database BOM lines with schematic instance counts.",
          "parameters": {
            "type": "object",
            "properties": { "design_id": { "type": "string" } },
            "required": ["design_id"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "run_research",
          "description": "Run Firecrawl search for a query and attach artifacts to the design.",
          "parameters": {
            "type": "object",
            "properties": {
              "design_id": { "type": "string" },
              "query": { "type": "string" }
            },
            "required": ["design_id", "query"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "append_bom_lines",
          "description": "Append BOM lines (does not delete existing lines). Each line references an existing part_id.",
          "parameters": {
            "type": "object",
            "properties": {
              "design_id": { "type": "string" },
              "lines": {
                "type": "array",
                "items": {
                  "type": "object",
                  "properties": {
                    "part_id": { "type": "string" },
                    "quantity": { "type": "number" },
                    "notes": { "type": "string" }
                  },
                  "required": ["part_id", "quantity"]
                }
              }
            },
            "required": ["design_id", "lines"]
          }
        }
      }
    ])
}

async fn execute_tool(
    state: &AppState,
    auth: &AuthUser,
    default_design: Option<Uuid>,
    name: &str,
    args: &Value,
) -> AppResult<Value> {
    match name {
        "search_parts" => {
            let q = args
                .get("q")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let limit = args.get("limit").and_then(|x| x.as_i64()).unwrap_or(25);
            let rows = parts::search(
                &state.pool,
                crate::models::PartSearchParams {
                    q: Some(q),
                    limit: Some(limit.clamp(1, 50)),
                },
            )
            .await?;
            serde_json::to_value(rows).map_err(|e| AppError::Any(e.into()))
        }
        "scrape_url" => {
            let url = args
                .get("url")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if url.trim().is_empty() {
                return Ok(json!({"error":"missing url"}));
            }
            account::reserve_scrapes(&state.pool, auth.user_id, 1).await?;
            let formats = args
                .get("formats")
                .cloned()
                .unwrap_or_else(|| json!(["markdown"]));
            let body = json!({ "url": url, "formats": formats });
            let res = firecrawl::scrape(state, body).await?;
            Ok(res)
        }
        "sync_part_offers" => {
            let pid = args
                .get("part_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or_else(|| AppError::BadRequest("part_id must be a UUID".into()))?;
            let sources: Vec<String> = args
                .get("sources")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                        .collect()
                })
                .unwrap_or_else(|| vec!["nexar".into(), "lcsc".into()]);
            let part = parts::get_by_id(&state.pool, pid).await?;
            let mut count = 0usize;
            for s in sources {
                match s.as_str() {
                    "nexar" | "octopart" => {
                        count += nexar::sync_offers_for_part(state, pid, &part.mpn).await?;
                    }
                    "lcsc" => {
                        let offs = lcsc::search_offers(state, &part.mpn).await;
                        for o in offs {
                            offers::upsert(&state.pool, pid, o).await?;
                            count += 1;
                        }
                    }
                    _ => {}
                }
            }
            Ok(json!({ "part_id": pid, "upserted_offers": count }))
        }
        "plan_build" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            let prompt = args
                .get("prompt")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            let plan = crate::services::design_pipeline::plan_build(
                state,
                &state.pool,
                auth.user_id,
                did,
                &prompt,
            )
            .await?;
            serde_json::to_value(plan).map_err(|e| AppError::Any(e.into()))
        }
        "resolve_bom" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            let prompt = args
                .get("prompt")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let pages = args
                .get("pages_ingested")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as usize;
            let plan: crate::services::design_pipeline::BuildPlan =
                serde_json::from_value(args.get("plan").cloned().unwrap_or(json!({})))
                    .map_err(|e| AppError::BadRequest(format!("invalid plan: {e}")))?;
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            let resolved = crate::services::design_pipeline::resolve_bom_parts(
                state,
                &state.pool,
                auth.user_id,
                did,
                &prompt,
                &plan,
                pages,
            )
            .await?;
            serde_json::to_value(resolved).map_err(|e| AppError::Any(e.into()))
        }
        "suggest_schematic" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            let prompt = args
                .get("prompt")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            let (schematic, erc) = crate::services::design_pipeline::suggest_schematic_stage(
                state,
                &state.pool,
                auth.user_id,
                did,
                &prompt,
            )
            .await?;
            Ok(json!({ "schematic": schematic, "erc_warnings": erc }))
        }
        "reconcile_bom" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            crate::services::design_pipeline::reconcile_bom(&state.pool, did).await
        }
        "run_research" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            let query = args
                .get("query")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            let ids = crate::services::research_pipeline::search_web_into_design(
                state,
                &state.pool,
                auth.user_id,
                did,
                &query,
                Some(4),
            )
            .await?;
            Ok(json!({ "design_id": did, "pages_ingested": ids.len() }))
        }
        "get_design_bom" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            let rows = bom::list_for_design(&state.pool, did).await?;
            serde_json::to_value(rows).map_err(|e| AppError::Any(e.into()))
        }
        "append_bom_lines" => {
            let did = args
                .get("design_id")
                .and_then(|x| x.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(default_design)
                .ok_or_else(|| AppError::BadRequest("design_id required".into()))?;
            designs::assert_visible(&state.pool, did, auth.user_id).await?;
            let lines_arr = args
                .get("lines")
                .and_then(|x| x.as_array())
                .ok_or_else(|| AppError::BadRequest("lines array required".into()))?;
            let mut inputs = Vec::new();
            for line in lines_arr {
                let part_id = line
                    .get("part_id")
                    .and_then(|x| x.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| AppError::BadRequest("each line needs part_id UUID".into()))?;
                let quantity = line
                    .get("quantity")
                    .and_then(|x| x.as_f64())
                    .ok_or_else(|| AppError::BadRequest("each line needs quantity".into()))?;
                if quantity <= 0.0 {
                    return Err(AppError::BadRequest("quantity must be > 0".into()));
                }
                let notes = line
                    .get("notes")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                inputs.push(crate::models::BomLineInput {
                    part_id,
                    quantity,
                    sort_order: 0,
                    notes,
                });
            }
            let appended = bom::append_lines(&state.pool, did, &inputs).await?;
            Ok(json!({ "design_id": did, "appended": appended }))
        }
        _ => Ok(json!({"error":"unknown tool", "name": name})),
    }
}

pub async fn run(state: &AppState, auth: AuthUser, input: AgentRunInput) -> AppResult<Value> {
    let model = input
        .model
        .clone()
        .unwrap_or_else(|| state.agent.default_model.clone());
    let mut messages = input.messages;
    if messages.is_empty() {
        messages.push(json!({
                "role": "system",
                "content": "You are Tokito's hardware design copilot. Use tools to fetch facts from the catalog and the web. Prefer concise outputs."
            }));
    }
    if let Some(did) = input.design_id {
        designs::assert_visible(&state.pool, did, auth.user_id).await?;
        messages.push(json!({
            "role": "system",
            "content": format!("Active design_id context (UUID): {did}")
        }));
    }

    let mut log: Vec<Value> = Vec::new();
    let mut total_prompt: i64 = 0;
    let mut total_completion: i64 = 0;
    let mut scrapes_used: i32 = 0;
    let mut llm_rounds: i32 = 0;
    let mut last_assistant_text: Option<String> = None;

    for iteration in 0..state.agent.max_iterations {
        if total_prompt + total_completion >= state.agent.max_llm_tokens_per_run {
            tracing::warn!("agent stopping: token cap hit");
            break;
        }
        let planned_tokens =
            (state.agent.max_llm_tokens_per_run - total_prompt - total_completion).clamp(1, 8192);

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools_schema(),
            "tool_choice": "auto",
            "stream": false,
        });

        let resp =
            llm::metered_chat_completion(state, &state.pool, auth.user_id, body, planned_tokens)
                .await?;
        llm_rounds += 1;
        let (pt, ct) = llm::usage_tokens(&resp);
        total_prompt += pt;
        total_completion += ct;

        let choice = resp
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| AppError::Upstream("AI response missing choices".into()))?;
        let msg = choice
            .pointer("/message")
            .cloned()
            .ok_or_else(|| AppError::Upstream("AI response missing message".into()))?;

        if let Some(t) = msg.get("content").and_then(|c| c.as_str()) {
            last_assistant_text = Some(t.to_string());
        }

        let mut amsg = json!({ "role": "assistant" });
        if let Some(c) = msg.get("content") {
            if !c.is_null() {
                amsg["content"] = c.clone();
            }
        }
        if let Some(tc) = msg.get("tool_calls") {
            if let Some(arr) = tc.as_array() {
                if !arr.is_empty() {
                    amsg["tool_calls"] = tc.clone();
                }
            }
        }
        messages.push(amsg);

        let Some(tcs) = msg.get("tool_calls").and_then(|x| x.as_array()) else {
            break;
        };
        if tcs.is_empty() {
            break;
        }

        log.push(json!({"iteration": iteration, "tool_calls": tcs}));

        for tc in tcs {
            let id = tc
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("tool_call")
                .to_string();
            let name = tc
                .pointer("/function/name")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let args_raw = tc
                .pointer("/function/arguments")
                .and_then(|x| x.as_str())
                .unwrap_or("{}");
            let args: Value = serde_json::from_str(args_raw).unwrap_or_else(|_| json!({}));
            let mut tool_result = execute_tool(state, &auth, input.design_id, name, &args).await?;
            if name == "scrape_url" {
                scrapes_used += 1;
                tool_result = json!({
                  "success": tool_result.get("success"),
                  "markdown_preview": tool_result.pointer("/data/markdown").and_then(|x| x.as_str()).map(|s| s.chars().take(8000).collect::<String>())
                });
            }
            let content = serde_json::to_string(&tool_result).unwrap_or_else(|_| "{}".to_string());
            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": content,
            }));
        }
    }

    let summary = last_assistant_text.map(|s| s.chars().take(500).collect::<String>());

    let run_id = account::insert_agent_run(
        &state.pool,
        auth.user_id,
        input.design_id,
        "completed",
        llm_rounds,
        total_prompt,
        total_completion,
        scrapes_used,
        serde_json::to_value(&log).unwrap_or_else(|_| json!([])),
        summary.as_deref(),
    )
    .await?;

    let final_message = summary.clone().unwrap_or_else(|| "Agent finished.".into());

    Ok(json!({
        "run_id": run_id,
        "usage": {
            "prompt_tokens": total_prompt,
            "completion_tokens": total_completion,
            "scrapes": scrapes_used
        },
        "summary": summary,
        "final_message": final_message,
        "messages": messages
    }))
}
