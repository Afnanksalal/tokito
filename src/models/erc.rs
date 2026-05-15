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
    /// Navigate to symbol in native editor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_name: Option<String>,
}

/// Returned by `POST …/schematic/validate` (non-persisting check).
#[derive(Debug, Serialize)]
pub struct SchematicValidationReport {
    pub topology_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_error: Option<String>,
    pub erc_warnings: Vec<ErcViolation>,
}

/// Copilot draft plus ERC advisory list.
#[derive(Debug, Serialize)]
pub struct SchematicSuggestResponse {
    pub schematic: ReplaceSchematic,
    pub erc_warnings: Vec<ErcViolation>,
    /// Reviewable operations (approve/reject in native studio).
    pub edit_batch: crate::models::SchematicEditBatch,
}
