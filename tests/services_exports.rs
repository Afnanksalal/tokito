//! Export services (no database).

use tokito::models::SchematicDocument;

#[test]
fn svg_export_empty_document() {
    let mut doc = SchematicDocument::empty();
    doc.symbols.push(tokito::models::DocumentSymbol {
        id: uuid::Uuid::new_v4(),
        sheet_id: "root".into(),
        part_id: None,
        symbol_id: None,
        ref_des: "R1".into(),
        value: None,
        position: tokito::models::DocumentPoint { x: 0.0, y: 0.0 },
        rotation: 0.0,
        mirror: Default::default(),
        fields: Default::default(),
        footprint_ref: None,
        pins: vec![],
    });
    let svg = tokito::services::svg_export::document_to_svg(&doc);
    assert!(svg.starts_with("<?xml"));
    assert!(svg.contains("R1"));
}

#[test]
fn sexp_netlist_empty_view() {
    let view = tokito::models::SchematicView {
        instances: vec![],
        nets: vec![],
        pins: vec![],
    };
    let text = tokito::services::sexp_netlist::export(&view);
    assert!(text.contains("(export"));
}
