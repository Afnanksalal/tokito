//! Keyword/MPN retrieval over design research artifacts (no embeddings).

use crate::models::DesignResearchArtifact;
use serde_json::{json, Value};

fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Score artifacts for relevance to prompt + optional MPN hints.
pub fn rank_artifacts<'a>(
    artifacts: &'a [DesignResearchArtifact],
    prompt: &str,
    mpns: &[&str],
) -> Vec<&'a DesignResearchArtifact> {
    let prompt_n = norm(prompt);
    let mpn_n: Vec<String> = mpns.iter().map(|m| norm(m)).collect();
    let mut scored: Vec<(&DesignResearchArtifact, i32)> = artifacts
        .iter()
        .map(|a| {
            let mut score = 0i32;
            if a
                .metadata_json
                .get("pinned")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                score += 100;
            }
            let body_n = norm(&a.content_text);
            let title_n = a.title.as_deref().map(norm).unwrap_or_default();
            for token in prompt_n.split_whitespace().filter(|t| t.len() >= 3) {
                if body_n.contains(token) || title_n.contains(token) {
                    score += 2;
                }
            }
            for m in &mpn_n {
                if !m.is_empty() && (body_n.contains(m.as_str()) || title_n.contains(m.as_str())) {
                    score += 8;
                }
            }
            if a.kind == crate::store::research::KIND_MANUAL_NOTE {
                score += 4;
            }
            (a, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.created_at.cmp(&a.0.created_at)));
    scored.into_iter().map(|(a, _)| a).collect()
}

pub fn excerpts_for_prompt(
    artifacts: &[DesignResearchArtifact],
    prompt: &str,
    mpns: &[&str],
    max_chars: usize,
) -> Vec<Value> {
    let ranked = rank_artifacts(artifacts, prompt, mpns);
    let mut out = Vec::new();
    let mut used = 0usize;
    for a in ranked {
        let excerpt: String = a.content_text.chars().take(8000).collect();
        let need = excerpt.chars().count();
        if used + need > max_chars && !out.is_empty() {
            break;
        }
        used += need;
        out.push(json!({
            "kind": &a.kind,
            "source_url": &a.source_url,
            "title": &a.title,
            "excerpt": excerpt,
            "pinned": a.metadata_json.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false),
        }));
        if used >= max_chars {
            break;
        }
    }
    out
}
