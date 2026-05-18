//! Product regression tests (config, exports, BOM, document schema).

use tokito::config_provider::{default_provider, ConfigProvider, FileSettingsProvider};
use tokito::services::bom_sync;
use tokito::services::pdf_export;
use tokito::services::svg_export;
use tokito::models::{DocumentPoint, DocumentWireSegment, SchematicDocument};

#[test]
fn config_provider_roundtrip() {
    let s = default_provider().load_settings();
    assert!(!s.general.theme.is_empty());
    let _ = FileSettingsProvider.load_config();
}

#[test]
fn pdf_pack_and_review_plot_headers() {
    let mut doc = SchematicDocument::empty();
    doc.wire_segments.push(DocumentWireSegment {
        id: uuid::Uuid::new_v4(),
        sheet_id: "root".into(),
        start: DocumentPoint { x: 0.0, y: 0.0 },
        end: DocumentPoint { x: 80.0, y: 0.0 },
        net_name: Some("N".into()),
        net_id: None,
        start_pin: None,
        end_pin: None,
    });
    let pdf = pdf_export::document_to_pdf_titled(&doc, "T");
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.windows(2).any(|w| w == b" m" || w == b" l"));
    let pack = pdf_export::document_to_pdf_pack(&doc, "T", "mpn,qty\n", &[]);
    assert!(pack.len() > pdf.len());
    let svg = svg_export::document_to_svg(&doc);
    assert!(svg.contains("<svg"));
}

#[test]
fn document_upgrade_v1_to_v2() {
    let mut doc = SchematicDocument::empty();
    doc.schema_version = 1;
    doc.net_labels.push(tokito::models::DocumentNetLabel {
        id: uuid::Uuid::new_v4(),
        sheet_id: tokito::models::DEFAULT_SHEET_ID.into(),
        name: "CLK".into(),
        kind: tokito::models::NetLabelKind::Local,
        position: tokito::models::DocumentPoint { x: 10.0, y: 10.0 },
        orientation: tokito::models::PinOrientation::Right,
    });
    let up = SchematicDocument::upgrade_to_current(doc);
    assert_eq!(up.schema_version, 2);
    assert!(up.symbols.iter().any(|s| {
        s.symbol_id
            .as_deref()
            .is_some_and(|id| id.starts_with("aux:Label_"))
    }));
}

#[test]
fn bom_diff_summary_detects_drift() {
    let lines = vec![];
    let body = tokito::models::ReplaceSchematic {
        instances: vec![tokito::models::SchematicInstanceInput {
            id: None,
            ref_des: "U1".into(),
            part_id: Some(uuid::Uuid::new_v4()),
            position: None,
            rotation: 0.0,
            meta: None,
        }],
        nets: vec![],
        pins: vec![],
    };
    let s = bom_sync::diff_summary(&lines, &body);
    assert!(!s.in_sync);
}

#[test]
fn bom_sync_quantity_from_three_instances() {
    let part_id = uuid::Uuid::new_v4();
    let body = tokito::models::ReplaceSchematic {
        instances: (1..=3)
            .map(|i| tokito::models::SchematicInstanceInput {
                id: None,
                ref_des: format!("R{i}"),
                part_id: Some(part_id),
                position: None,
                rotation: 0.0,
                meta: None,
            })
            .collect(),
        nets: vec![],
        pins: vec![],
    };
    let proposed = tokito::services::bom_sync::propose_from_schematic(&body);
    assert_eq!(proposed.len(), 1);
    assert!((proposed[0].quantity - 3.0).abs() < f64::EPSILON);
}
