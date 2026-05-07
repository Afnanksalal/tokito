//! Electrical rule check (ERC) reports — topology errors vs advisory warnings.

use serde::{Deserialize, Serialize};

use super::ReplaceSchematic;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErcSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErcViolation {
    pub code: String,
    pub severity: ErcSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Returned by `POST …/schematic/validate` (non-persisting check).
#[derive(Debug, Serialize)]
pub struct SchematicValidationReport {
    pub topology_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_error: Option<String>,
    pub erc_warnings: Vec<ErcViolation>,
}

/// AI draft + ERC advisory list (save still allowed with warnings).
#[derive(Debug, Serialize)]
pub struct SchematicSuggestResponse {
    pub schematic: ReplaceSchematic,
    pub erc_warnings: Vec<ErcViolation>,
}
