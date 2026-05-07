//! HTTP handlers for the REST API.

mod agent;
mod auth;
mod copilot;
mod designs;
mod health;
mod integrations;
mod manufacturers;
mod offers;
mod parts;

pub use agent::run_agent;
pub use auth::{create_api_key, delete_api_key, list_api_keys, login, register};
pub use copilot::{get_intent, list_research, put_intent, scrape_research, search_research};
pub use designs::{
    append_bom, create_design, export_design, get_bom, get_design, get_schematic, list_designs,
    patch_design, put_bom, put_schematic, suggest_schematic, validate_schematic_payload,
};
pub use health::health;
pub use integrations::{firecrawl_scrape, firecrawl_search, xai_chat_completions};
pub use manufacturers::{create_mfg, list_mfg};
pub use offers::{list_part_offers, sync_part_offers};
pub use parts::{create_part, get_part, search_parts};
