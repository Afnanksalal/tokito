//! Golden: document connectivity unchanged after symbol translation (pin-anchored wires).

use std::collections::BTreeSet;
use tokito::models::{
    DocumentPin, DocumentPinAnchor, DocumentPoint, DocumentWireSegment, ElectricalPinType,
    MirrorMode, PinOrientation, SchematicDocument,
};
use uuid::Uuid;

fn pin_net_set(doc: &SchematicDocument) -> BTreeSet<String> {
    let (body, _) = doc.to_replace_schematic();
    body.pins
        .iter()
        .map(|p| format!("{}:{}:{}", p.instance_ref, p.pin_name, p.net_name))
        .collect()
}

#[test]
fn document_pin_nets_stable_when_symbol_moves_with_anchored_wires() {
    let mut doc = SchematicDocument::empty();
    let sheet = "root".to_string();
    doc.symbols.push(tokito::models::DocumentSymbol {
        id: Uuid::new_v4(),
        sheet_id: sheet.clone(),
        part_id: None,
        symbol_id: None,
        ref_des: "R1".into(),
        value: Some("10k".into()),
        position: DocumentPoint { x: 200.0, y: 200.0 },
        rotation: 0.0,
        mirror: MirrorMode::None,
        fields: Default::default(),
        footprint_ref: None,
        pins: vec![
            DocumentPin {
                number: Some("1".into()),
                name: "1".into(),
                electrical_type: ElectricalPinType::Passive,
                offset: DocumentPoint { x: -40.0, y: 0.0 },
                orientation: PinOrientation::Left,
                visible: true,
            },
            DocumentPin {
                number: Some("2".into()),
                name: "2".into(),
                electrical_type: ElectricalPinType::Passive,
                offset: DocumentPoint { x: 40.0, y: 0.0 },
                orientation: PinOrientation::Right,
                visible: true,
            },
        ],
    });
    doc.wire_segments.push(DocumentWireSegment {
        id: Uuid::new_v4(),
        sheet_id: sheet.clone(),
        start: DocumentPoint { x: 160.0, y: 200.0 },
        end: DocumentPoint { x: 240.0, y: 200.0 },
        net_name: Some("SIGNAL".into()),
        net_id: None,
        start_pin: Some(DocumentPinAnchor {
            ref_des: "R1".into(),
            pin_name: "1".into(),
        }),
        end_pin: Some(DocumentPinAnchor {
            ref_des: "R1".into(),
            pin_name: "2".into(),
        }),
    });

    let before = pin_net_set(&doc);
    assert!(before.iter().any(|p| p.contains("SIGNAL")));

    doc.symbols[0].position.x += 100.0;
    doc.symbols[0].position.y += 50.0;
    for w in &mut doc.wire_segments {
        w.start.x += 100.0;
        w.start.y += 50.0;
        w.end.x += 100.0;
        w.end.y += 50.0;
    }

    let after = pin_net_set(&doc);
    assert_eq!(before, after);
}
