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
fn svg_net_label_uses_flag_geometry() {
    use tokito::models::{DocumentNetLabel, DocumentPoint, NetLabelKind};
    let mut doc = SchematicDocument::empty();
    doc.net_labels.push(DocumentNetLabel {
        id: uuid::Uuid::new_v4(),
        sheet_id: "root".into(),
        name: "VCC".into(),
        kind: NetLabelKind::Local,
        position: DocumentPoint { x: 100.0, y: 80.0 },
        orientation: tokito::models::PinOrientation::Right,
    });
    let svg = tokito::services::svg_export::document_to_svg(&doc);
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("VCC"));
    assert!(
        svg.contains("<polygon") || svg.contains("<line") || svg.contains("<path"),
        "label export must include vector geometry, not text only"
    );
}

#[test]
fn pdf_review_includes_vector_strokes() {
    use tokito::models::{DocumentNetLabel, DocumentPoint, NetLabelKind};
    let mut doc = SchematicDocument::empty();
    doc.net_labels.push(DocumentNetLabel {
        id: uuid::Uuid::new_v4(),
        sheet_id: "root".into(),
        name: "SDA".into(),
        kind: NetLabelKind::Local,
        position: DocumentPoint { x: 50.0, y: 50.0 },
        orientation: tokito::models::PinOrientation::Right,
    });
    let pdf = tokito::services::pdf_export::document_to_pdf_titled(&doc, "T");
    assert!(pdf.starts_with(b"%PDF"));
    let body = String::from_utf8_lossy(&pdf);
    assert!(
        body.contains(" m") || body.contains(" l"),
        "review PDF must draw strokes"
    );
}

#[test]
fn export_bundle_zip_contains_core_files() {
    use tokito::models::SchematicView;
    let doc = SchematicDocument::empty();
    let view = SchematicView {
        instances: vec![],
        nets: vec![],
        pins: vec![],
    };
    let dir = std::env::temp_dir().join(format!("tokito_zip_{}", uuid::Uuid::new_v4()));
    let mcad = tokito::services::mcad_export::document_handoff_json(&doc, "Test");
    let written = tokito::services::export_bundle::write_design_exports_zip(
        &dir,
        "test",
        &doc,
        &view,
        "ref,mpn,qty\n",
        Some(&mcad),
    )
    .expect("zip export");
    let zip_path = written.zip_path.expect("zip path");
    assert!(zip_path.is_file());
    let file = std::fs::File::open(&zip_path).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("read zip");
    let mut names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();
    assert!(names.iter().any(|n| n.ends_with(".svg")));
    assert!(names.iter().any(|n| n.ends_with(".pdf")));
    assert!(names.iter().any(|n| n.contains("bom")));
    assert!(names.iter().any(|n| n.contains("mcad")));
    let _ = std::fs::remove_dir_all(&dir);
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
