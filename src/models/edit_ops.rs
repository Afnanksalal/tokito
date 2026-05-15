//! Reviewable schematic edit operations (Flux-style copilot apply flow).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Position, ReplaceSchematic};

/// Provenance for a single proposed edit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditProvenance {
    ModelInference,
    BomLine { part_id: Uuid },
    ResearchArtifact { artifact_id: Uuid },
    UserRequest,
    ErcFix { code: String },
}

/// One inspectable change the user can approve or reject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SchematicEditOp {
    ReplaceSchematic {
        schematic: ReplaceSchematic,
        summary: String,
    },
    AddInstance {
        ref_des: String,
        part_id: Option<Uuid>,
        position: Position,
        rotation: f64,
        summary: String,
    },
    RemoveInstance {
        ref_des: String,
        summary: String,
    },
    ConnectPins {
        net_name: String,
        pins: Vec<(String, String)>,
        summary: String,
    },
    SetInstanceField {
        ref_des: String,
        field: String,
        value: String,
        summary: String,
    },
}

impl SchematicEditOp {
    pub fn summary(&self) -> &str {
        match self {
            Self::ReplaceSchematic { summary, .. }
            | Self::AddInstance { summary, .. }
            | Self::RemoveInstance { summary, .. }
            | Self::ConnectPins { summary, .. }
            | Self::SetInstanceField { summary, .. } => summary,
        }
    }
}

/// Batch returned by generation / suggest endpoints for native review UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchematicEditBatch {
    pub ops: Vec<SchematicEditOp>,
    /// Per-operation provenance (`ops[i]` → `op_provenance[i]` when present).
    #[serde(default)]
    pub op_provenance: Vec<EditProvenance>,
    /// Batch-level provenance (pipeline / user action).
    #[serde(default)]
    pub provenance: Vec<EditProvenance>,
}

impl SchematicEditBatch {
    pub fn from_replace(schematic: ReplaceSchematic, summary: impl Into<String>) -> Self {
        Self {
            ops: vec![SchematicEditOp::ReplaceSchematic {
                schematic,
                summary: summary.into(),
            }],
            op_provenance: vec![EditProvenance::ModelInference],
            provenance: vec![EditProvenance::ModelInference],
        }
    }

    pub fn provenance_for_op(&self, index: usize) -> EditProvenance {
        self.op_provenance
            .get(index)
            .cloned()
            .or_else(|| self.provenance.first().cloned())
            .unwrap_or(EditProvenance::ModelInference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Position, ReplaceSchematic, SchematicInstanceInput, SchematicNetInput, SchematicPinInput,
    };

    #[test]
    fn batch_from_replace_has_one_op() {
        let batch = SchematicEditBatch::from_replace(
            ReplaceSchematic {
                instances: vec![SchematicInstanceInput {
                    id: None,
                    part_id: None,
                    ref_des: "R1".into(),
                    position: Some(Position { x: 0.0, y: 0.0 }),
                    rotation: 0.0,
                    meta: None,
                }],
                nets: vec![SchematicNetInput {
                    id: None,
                    name: "GND".into(),
                }],
                pins: vec![SchematicPinInput {
                    instance_ref: "R1".into(),
                    pin_name: "1".into(),
                    net_name: "GND".into(),
                }],
            },
            "test",
        );
        assert_eq!(batch.ops.len(), 1);
        assert_eq!(batch.ops[0].summary(), "test");
    }
}
