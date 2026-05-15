//! Bridge between native editor geometry and shared `SchematicDocument`.

use uuid::Uuid;

use crate::editor::{sheets, SchematicEditor};
use tokito::models::SchematicDocument;

/// Load a persisted document into editor state (active sheet).
pub fn load_document(editor: &mut SchematicEditor, doc: SchematicDocument) {
    let sheet = editor.active_sheet_id.clone();
    sheets::hydrate_active_sheet(editor, &doc, &sheet);
}

/// Export full document: caller must flush via `sheets::flush_active_sheet` into cached doc.
pub fn export_document(
    editor: &SchematicEditor,
    part_cache: &std::collections::HashMap<Uuid, String>,
    doc: &mut SchematicDocument,
) {
    sheets::flush_active_sheet(editor, doc, part_cache);
}
