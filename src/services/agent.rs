//! Multi-step tool loop with cost caps and structured logging.

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::router::AppState;
use crate::services::{firecrawl, lcsc, nexar, xai};
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
            account::ensure_scrape_quota(&state.pool, auth.user_id).await?;
            let formats = args
                .get("formats")
                .cloned()
                .unwrap_or_else(|| json!(["markdown"]));
            let body = json!({ "url": url, "formats": formats });
            let res = firecrawl::scrape(state, body).await?;
            account::record_scrape(&state.pool, auth.user_id).await?;
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
        account::ensure_llm_quota(&state.pool, auth.user_id, 8192).await?;

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools_schema(),
            "tool_choice": "auto",
            "stream": false,
        });

        let resp = xai::chat_completion(state, body).await?;
        llm_rounds += 1;
        let (pt, ct) = xai::usage_tokens(&resp);
        total_prompt += pt;
        total_completion += ct;
        account::record_llm_usage(&state.pool, auth.user_id, pt, ct).await?;

        let choice = resp
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| AppError::Upstream("xAI response missing choices".into()))?;
        let msg = choice
            .pointer("/message")
            .cloned()
            .ok_or_else(|| AppError::Upstream("xAI response missing message".into()))?;

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

    Ok(json!({
        "run_id": run_id,
        "usage": {
            "prompt_tokens": total_prompt,
            "completion_tokens": total_completion,
            "scrapes": scrapes_used
        },
        "messages": messages
    }))
}
