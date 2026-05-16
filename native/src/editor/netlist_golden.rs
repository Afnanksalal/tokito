//! Golden tests: electrical connectivity survives symbol moves.

use std::collections::BTreeSet;

use egui::Pos2;

use crate::canvas::{PinEndpoint, Sym};
use crate::editor::document::export_document;
use crate::editor::SchematicEditor;

fn pin_net_set(editor: &SchematicEditor) -> BTreeSet<String> {
    let mut doc = tokito::models::SchematicDocument::empty();
    export_document(editor, &std::collections::HashMap::new(), &mut doc);
    let (body, _) = doc.to_replace_schematic();
    body.pins
        .iter()
        .map(|p| format!("{}:{}:{}", p.instance_ref, p.pin_name, p.net_name))
        .collect()
}

#[test]
fn netlist_topology_stable_after_symbol_move() {
    let mut editor = SchematicEditor::default();
    editor.symbols.push(Sym {
        ref_des: "R1".into(),
        part_id: None,
        pos: Pos2::new(200.0, 200.0),
        rotation_deg: 0.0,
        pins: vec!["1".into(), "2".into()],
        footprint_ref: None,
        symbol_id: None,
        pin_layout: vec![("1".into(), -40.0, 0.0), ("2".into(), 40.0, 0.0)],
        value: "10k".into(),
        fields: Default::default(),
    });

    let net = "SIGNAL".to_string();
    editor.push_wire_between(
        PinEndpoint {
            ref_des: "R1".into(),
            pin_name: "1".into(),
        },
        PinEndpoint {
            ref_des: "R1".into(),
            pin_name: "2".into(),
        },
        net,
    );
    editor.refresh_wire_connectivity();
    let before = pin_net_set(&editor);

    assert!(
        before.iter().any(|p| p.contains("SIGNAL")),
        "expected wired net before move: {before:?}"
    );

    editor.symbols[0].pos += egui::vec2(120.0, 80.0);
    let mut moved = std::collections::HashSet::new();
    moved.insert("R1".into());
    editor.push_wires_for_moved_symbols(&moved, egui::vec2(120.0, 80.0));
    editor.sync_wires_after_symbol_move();
    editor.refresh_wire_connectivity();

    let after = pin_net_set(&editor);
    assert_eq!(
        before, after,
        "pin→net map must be unchanged after symbol move"
    );
}
