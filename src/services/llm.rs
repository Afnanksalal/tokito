//! Normalized chat completions across supported AI providers.

use crate::config::{AiProvider, LlmConfig};
use crate::error::{AppError, AppResult};
use crate::router::AppState;
use crate::store::account;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

const DEFAULT_MAX_COMPLETION_TOKENS: i64 = 8192;

fn join_base_path(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

pub fn usage_tokens(resp: &Value) -> (i64, i64) {
    let usage = resp.get("usage");
    let prompt = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let completion = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    (prompt, completion)
}

fn cap_token_field(obj: &mut serde_json::Map<String, Value>, key: &str, max_tokens: i64) {
    let current = obj.get(key).and_then(|v| v.as_i64());
    if current.map(|n| n > max_tokens).unwrap_or(true) {
        obj.insert(key.to_string(), Value::from(max_tokens));
    }
}

fn max_tokens_from_body(body: &Value) -> i64 {
    body.get("max_tokens")
        .or_else(|| body.get("max_completion_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_MAX_COMPLETION_TOKENS)
}

pub fn enforce_completion_token_cap(mut body: Value, max_tokens: i64) -> Value {
    if let Some(obj) = body.as_object_mut() {
        cap_token_field(obj, "max_tokens", max_tokens);
        if obj.contains_key("max_completion_tokens") {
            cap_token_field(obj, "max_completion_tokens", max_tokens);
        }
    }
    body
}

pub async fn metered_chat_completion(
    state: &AppState,
    pool: &PgPool,
    user_id: Uuid,
    body: Value,
    planned_tokens: i64,
) -> AppResult<Value> {
    let reserved = account::reserve_llm_tokens(pool, user_id, planned_tokens).await?;
    let body = enforce_completion_token_cap(body, DEFAULT_MAX_COMPLETION_TOKENS);
    match chat_completion(state, body).await {
        Ok(resp) => {
            let (prompt_tokens, completion_tokens) = usage_tokens(&resp);
            account::reconcile_llm_reservation(
                pool,
                user_id,
                reserved,
                prompt_tokens,
                completion_tokens,
            )
            .await?;
            Ok(resp)
        }
        Err(err) => {
            account::refund_llm_reservation(pool, user_id, reserved).await?;
            Err(err)
        }
    }
}

pub async fn chat_completion(state: &AppState, body: Value) -> AppResult<Value> {
    let Some(llm) = state.llm.as_ref() else {
        return Err(AppError::Unavailable(
            "AI provider is not configured".into(),
        ));
    };
    match llm.provider {
        AiProvider::OpenAi | AiProvider::Xai | AiProvider::Kimi => {
            openai_compatible_chat_completion(state, llm, body).await
        }
        AiProvider::Anthropic => anthropic_messages_completion(state, llm, body).await,
        AiProvider::Gemini => gemini_generate_content(state, llm, body).await,
    }
}

async fn openai_compatible_chat_completion(
    state: &AppState,
    llm: &LlmConfig,
    body: Value,
) -> AppResult<Value> {
    let url = join_base_path(&llm.base_url, "chat/completions");
    let res = state
        .http
        .post(&url)
        .bearer_auth(&llm.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "{} {status}: {}",
            llm.provider.as_str(),
            String::from_utf8_lossy(&bytes)
        )));
    }
    serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))
}

async fn anthropic_messages_completion(
    state: &AppState,
    llm: &LlmConfig,
    body: Value,
) -> AppResult<Value> {
    let model = model_from_body(&body).unwrap_or_else(|| llm.provider.default_model().to_string());
    let max_tokens = max_tokens_from_body(&body);
    let temperature = body.get("temperature").and_then(|v| v.as_f64());
    let mut system = Vec::new();
    let mut messages = Vec::new();
    for msg in body
        .get("messages")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
        if role == "system" {
            let text = message_text(msg);
            if text.is_empty() {
                continue;
            }
            system.push(text);
        } else {
            push_anthropic_message(&mut messages, msg);
        }
    }
    let mut request = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages,
    });
    if !system.is_empty() {
        request["system"] = Value::from(system.join("\n\n"));
    }
    if let Some(t) = temperature {
        request["temperature"] = Value::from(t);
    }
    let tools = anthropic_tools_from_openai(&body);
    if !tools.is_empty() {
        request["tools"] = Value::Array(tools);
    }
    let url = join_base_path(&llm.base_url, "messages");
    let res = state
        .http
        .post(&url)
        .header("x-api-key", &llm.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "anthropic {status}: {}",
            String::from_utf8_lossy(&bytes)
        )));
    }
    let raw: Value = serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))?;
    let mut content_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for part in raw
        .get("content")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        match part.get("type").and_then(|v| v.as_str()) {
            Some("text") => {
                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                    content_parts.push(text);
                }
            }
            Some("tool_use") => {
                let id = part
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool_use")
                    .to_string();
                let name = part
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = part.get("input").cloned().unwrap_or_else(|| json!({}));
                tool_calls.push(json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": args.to_string(),
                    }
                }));
            }
            _ => {}
        }
    }
    let mut message = json!({
        "role": "assistant",
        "content": content_parts.join(""),
    });
    if !tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls);
    }
    Ok(json!({
        "id": raw.get("id").cloned().unwrap_or(Value::Null),
        "model": raw.get("model").cloned().unwrap_or(Value::String(model)),
        "choices": [{
            "index": 0,
            "finish_reason": if message.get("tool_calls").is_some() { Value::from("tool_calls") } else { raw.get("stop_reason").cloned().unwrap_or(Value::Null) },
            "message": message,
        }],
        "usage": {
            "prompt_tokens": raw.pointer("/usage/input_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            "completion_tokens": raw.pointer("/usage/output_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
        }
    }))
}

fn push_anthropic_message(messages: &mut Vec<Value>, msg: &Value) {
    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
    let mapped_role = if role == "assistant" {
        "assistant"
    } else {
        "user"
    };
    let mut blocks = Vec::new();

    match role {
        "assistant" => {
            let text = message_text(msg);
            if !text.is_empty() {
                blocks.push(json!({ "type": "text", "text": text }));
            }
            for call in msg
                .get("tool_calls")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
            {
                let id = call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool_call");
                let name = call
                    .pointer("/function/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if name.is_empty() {
                    continue;
                }
                let args_raw = call
                    .pointer("/function/arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let input = serde_json::from_str::<Value>(args_raw).unwrap_or_else(|_| {
                    json!({
                        "raw_arguments": args_raw,
                    })
                });
                blocks.push(json!({
                    "type": "tool_use",
                    "id": id,
                    "name": name,
                    "input": input,
                }));
            }
        }
        "tool" => {
            let id = msg
                .get("tool_call_id")
                .and_then(|v| v.as_str())
                .unwrap_or("tool_call");
            blocks.push(json!({
                "type": "tool_result",
                "tool_use_id": id,
                "content": message_text(msg),
            }));
        }
        _ => {
            let text = message_text(msg);
            if !text.is_empty() {
                blocks.push(json!({ "type": "text", "text": text }));
            }
        }
    }

    if blocks.is_empty() {
        return;
    }
    if let Some(last) = messages.last_mut() {
        if last.get("role").and_then(|v| v.as_str()) == Some(mapped_role) {
            if let Some(content) = last.get_mut("content").and_then(|v| v.as_array_mut()) {
                content.extend(blocks);
                return;
            }
        }
    }
    messages.push(json!({
        "role": mapped_role,
        "content": blocks,
    }));
}

fn anthropic_tools_from_openai(body: &Value) -> Vec<Value> {
    openai_function_tools(body)
        .into_iter()
        .filter_map(|tool| {
            let function = tool.get("function")?;
            let name = function.get("name")?.as_str()?;
            let mut out = json!({
                "name": name,
                "input_schema": function
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            });
            if let Some(description) = function.get("description").and_then(|v| v.as_str()) {
                out["description"] = Value::from(description);
            }
            Some(out)
        })
        .collect()
}

async fn gemini_generate_content(
    state: &AppState,
    llm: &LlmConfig,
    body: Value,
) -> AppResult<Value> {
    let model = model_from_body(&body).unwrap_or_else(|| llm.provider.default_model().to_string());
    let mut system_parts = Vec::new();
    let mut contents = Vec::new();
    let mut tool_call_names = HashMap::new();
    for msg in body
        .get("messages")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
        if role == "system" {
            let text = message_text(msg);
            if text.is_empty() {
                continue;
            }
            system_parts.push(json!({ "text": text }));
        } else {
            push_gemini_content(&mut contents, &mut tool_call_names, msg);
        }
    }
    let max_tokens = max_tokens_from_body(&body);
    let mut request = json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": max_tokens,
        }
    });
    if !system_parts.is_empty() {
        request["systemInstruction"] = json!({ "parts": system_parts });
    }
    if let Some(t) = body.get("temperature").and_then(|v| v.as_f64()) {
        request["generationConfig"]["temperature"] = Value::from(t);
    }
    let tools = gemini_tools_from_openai(&body);
    if !tools.is_empty() {
        request["tools"] = json!([{ "functionDeclarations": tools }]);
    }
    let model_path = gemini_model_path(&model);
    let url = format!(
        "{}/models/{}:generateContent",
        llm.base_url.trim_end_matches('/'),
        model_path
    );
    let res = state
        .http
        .post(&url)
        .header("x-goog-api-key", &llm.api_key)
        .json(&request)
        .send()
        .await
        .map_err(|e| AppError::Any(e.into()))?;
    let status = res.status();
    let bytes = res.bytes().await.map_err(|e| AppError::Any(e.into()))?;
    if !status.is_success() {
        return Err(AppError::Upstream(format!(
            "gemini {status}: {}",
            String::from_utf8_lossy(&bytes)
        )));
    }
    let raw: Value = serde_json::from_slice(&bytes).map_err(|e| AppError::Any(e.into()))?;
    let mut content_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for (idx, part) in raw
        .pointer("/candidates/0/content/parts")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .enumerate()
    {
        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
            content_parts.push(text);
        }
        if let Some(call) = part.get("functionCall") {
            let name = call.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let args = call.get("args").cloned().unwrap_or_else(|| json!({}));
            tool_calls.push(json!({
                "id": format!("gemini_tool_{idx}"),
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": args.to_string(),
                }
            }));
        }
    }
    let mut message = json!({
        "role": "assistant",
        "content": content_parts.join(""),
    });
    if !tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls);
    }
    Ok(json!({
        "model": model,
        "choices": [{
            "index": 0,
            "finish_reason": if message.get("tool_calls").is_some() { Value::from("tool_calls") } else { raw.pointer("/candidates/0/finishReason").cloned().unwrap_or(Value::Null) },
            "message": message,
        }],
        "usage": {
            "prompt_tokens": raw.pointer("/usageMetadata/promptTokenCount").and_then(|v| v.as_i64()).unwrap_or(0),
            "completion_tokens": raw.pointer("/usageMetadata/candidatesTokenCount").and_then(|v| v.as_i64()).unwrap_or(0),
        }
    }))
}

fn push_gemini_content(
    contents: &mut Vec<Value>,
    tool_call_names: &mut HashMap<String, String>,
    msg: &Value,
) {
    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
    let mapped_role = if role == "assistant" { "model" } else { "user" };
    let mut parts = Vec::new();

    match role {
        "assistant" => {
            let text = message_text(msg);
            if !text.is_empty() {
                parts.push(json!({ "text": text }));
            }
            for call in msg
                .get("tool_calls")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
            {
                let id = call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool_call");
                let name = call
                    .pointer("/function/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if name.is_empty() {
                    continue;
                }
                tool_call_names.insert(id.to_string(), name.to_string());
                let args_raw = call
                    .pointer("/function/arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let args = serde_json::from_str::<Value>(args_raw).unwrap_or_else(|_| {
                    json!({
                        "raw_arguments": args_raw,
                    })
                });
                parts.push(json!({
                    "functionCall": {
                        "name": name,
                        "args": args,
                    }
                }));
            }
        }
        "tool" => {
            let id = msg
                .get("tool_call_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let name = tool_call_names
                .get(id)
                .cloned()
                .unwrap_or_else(|| "tool_result".to_string());
            let content = message_text(msg);
            let response = serde_json::from_str::<Value>(&content).unwrap_or_else(|_| {
                json!({
                    "content": content,
                })
            });
            parts.push(json!({
                "functionResponse": {
                    "name": name,
                    "response": response,
                }
            }));
        }
        _ => {
            let text = message_text(msg);
            if !text.is_empty() {
                parts.push(json!({ "text": text }));
            }
        }
    }

    if parts.is_empty() {
        return;
    }
    if let Some(last) = contents.last_mut() {
        if last.get("role").and_then(|v| v.as_str()) == Some(mapped_role) {
            if let Some(existing) = last.get_mut("parts").and_then(|v| v.as_array_mut()) {
                existing.extend(parts);
                return;
            }
        }
    }
    contents.push(json!({
        "role": mapped_role,
        "parts": parts,
    }));
}

fn gemini_tools_from_openai(body: &Value) -> Vec<Value> {
    openai_function_tools(body)
        .into_iter()
        .filter_map(|tool| {
            let function = tool.get("function")?;
            let name = function.get("name")?.as_str()?;
            let mut out = json!({
                "name": name,
                "parameters": function
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            });
            if let Some(description) = function.get("description").and_then(|v| v.as_str()) {
                out["description"] = Value::from(description);
            }
            Some(out)
        })
        .collect()
}

fn gemini_model_path(model: &str) -> String {
    let trimmed = model.trim().trim_start_matches("models/");
    urlencoding::encode(trimmed).into_owned()
}

fn openai_function_tools(body: &Value) -> Vec<Value> {
    body.get("tools")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|tool| tool.get("type").and_then(|v| v.as_str()) == Some("function"))
        .filter(|tool| {
            tool.pointer("/function/name")
                .and_then(|v| v.as_str())
                .is_some()
        })
        .cloned()
        .collect()
}

fn model_from_body(body: &Value) -> Option<String> {
    body.get("model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn message_text(message: &Value) -> String {
    let Some(content) = message.get("content") else {
        return String::new();
    };
    if let Some(text) = content.as_str() {
        return text.to_string();
    }
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| {
            part.get("text")
                .and_then(|v| v.as_str())
                .or_else(|| part.get("content").and_then(|v| v.as_str()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
