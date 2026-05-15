//! Undoable edit command descriptors (snapshot-based undo for now).

use crate::canvas::CanvasSnapshot;

#[derive(Clone)]
pub enum EditorCommand {
    Snapshot(CanvasSnapshot),
}

impl EditorCommand {
    pub fn snapshot(snap: CanvasSnapshot) -> Self {
        Self::Snapshot(snap)
    }
}
