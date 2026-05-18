//! JSON fixtures for AI plan/resolve parsing (no live LLM).

use serde::Deserialize;
use tokito::services::schematic_gen::extract_llm_json_object;

#[derive(Debug, Deserialize)]
struct PlanFixture {
    #[serde(default)]
    firecrawl_queries: Vec<String>,
    #[serde(default)]
    candidate_parts: Vec<CandidateFixture>,
}

#[derive(Debug, Deserialize)]
struct CandidateFixture {
    mpn: String,
}

#[derive(Debug, Deserialize)]
struct ResolveFixture {
    resolved_parts: Vec<ResolvedFixture>,
}

#[derive(Debug, Deserialize)]
struct ResolvedFixture {
    mpn: String,
}

#[test]
fn extract_json_from_markdown_fence() {
    let raw = r#"Here is the plan:
```json
{"firecrawl_queries":["TPS54302 datasheet"],"candidate_parts":[{"mpn":"TPS54302"}]}
```
"#;
    let json = extract_llm_json_object(raw).expect("extract");
    let plan: PlanFixture = serde_json::from_str(&json).expect("plan parse");
    assert_eq!(plan.firecrawl_queries.len(), 1);
    assert_eq!(plan.candidate_parts[0].mpn, "TPS54302");
}

#[test]
fn resolve_phase_parses_minimal() {
    let json = r#"{"resolved_parts":[{"mpn":"LM1117-3.3"}]}"#;
    let phase: ResolveFixture = serde_json::from_str(json).expect("resolve");
    assert_eq!(phase.resolved_parts.len(), 1);
    assert_eq!(phase.resolved_parts[0].mpn, "LM1117-3.3");
}
