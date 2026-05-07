//! Distributor offer refresh shared by HTTP handlers and the native client.

use crate::error::{AppError, AppResult};
use crate::router::AppState;
use crate::services::{lcsc, nexar};
use crate::store::{offers, parts};
use serde_json::json;
use uuid::Uuid;

pub async fn sync_part_offers_for_part(
    state: &AppState,
    part_id: Uuid,
    sources: Vec<String>,
) -> AppResult<serde_json::Value> {
    let part = parts::get_by_id(&state.pool, part_id).await?;
    let sources: Vec<String> = if sources.is_empty() {
        vec!["nexar".into(), "lcsc".into()]
    } else {
        sources.into_iter().map(|s| s.to_lowercase()).collect()
    };
    let mut nexar_count = 0usize;
    let mut lcsc_count = 0usize;
    for s in sources {
        match s.as_str() {
            "nexar" | "octopart" => {
                nexar_count += nexar::sync_offers_for_part(state, part_id, &part.mpn).await?;
            }
            "lcsc" => {
                let offs = lcsc::search_offers(state, &part.mpn).await;
                lcsc_count += offs.len();
                for o in offs {
                    offers::upsert(&state.pool, part_id, o).await?;
                }
            }
            other => {
                return Err(AppError::BadRequest(format!(
                    "unknown source '{other}' (use nexar, octopart, lcsc)"
                )));
            }
        }
    }
    Ok(json!({
        "part_id": part_id,
        "nexar_offers_upserted": nexar_count,
        "lcsc_rows_upserted": lcsc_count
    }))
}
