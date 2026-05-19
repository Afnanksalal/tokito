//! Golden tests: document geometry round-trips to normalized schematic graph.

use tokito::models::{
    DocumentPin, DocumentPoint, DocumentWireSegment, ElectricalPinType, PinOrientation, Position,
    ReplaceSchematic, SchematicDocument, SchematicInstanceInput, SchematicNetInput,
    SchematicPinInput,
};
use uuid::Uuid;

#[test]
fn document_wire_segments_derive_pin_connectivity() {
    let mut doc = SchematicDocument::empty();
    doc.symbols = vec![tokito::models::DocumentSymbol {
        id: Uuid::new_v4(),
        sheet_id: "root".into(),
        part_id: None,
        symbol_id: None,
        ref_des: "R1".into(),
        value: None,
        position: DocumentPoint { x: 0.0, y: 0.0 },
        rotation: 0.0,
        mirror: Default::default(),
        fields: Default::default(),
        footprint_ref: None,
        pins: vec![
            DocumentPin {
                number: Some("1".into()),
                name: "1".into(),
                electrical_type: ElectricalPinType::Passive,
                offset: DocumentPoint { x: -70.0, y: 0.0 },
                orientation: PinOrientation::Left,
                visible: true,
            },
            DocumentPin {
                number: Some("2".into()),
                name: "2".into(),
                electrical_type: ElectricalPinType::Passive,
                offset: DocumentPoint { x: 70.0, y: 0.0 },
                orientation: PinOrientation::Right,
                visible: true,
            },
        ],
    }];
    doc.wire_segments = vec![
        DocumentWireSegment {
            id: Uuid::new_v4(),
            sheet_id: "root".into(),
            start: DocumentPoint { x: -70.0, y: 0.0 },
            end: DocumentPoint { x: 0.0, y: 0.0 },
            net_name: Some("NET1".into()),
            net_id: None,
            start_pin: None,
            end_pin: None,
        },
        DocumentWireSegment {
            id: Uuid::new_v4(),
            sheet_id: "root".into(),
            start: DocumentPoint { x: 0.0, y: 0.0 },
            end: DocumentPoint { x: 70.0, y: 0.0 },
            net_name: Some("NET1".into()),
            net_id: None,
            start_pin: None,
            end_pin: None,
        },
    ];
    let (body, diags) = doc.to_replace_schematic();
    assert!(diags.is_empty() || diags.iter().all(|d| !d.code.contains("ERROR")));
    // SchematicPinInput has no UUID fields, so no redaction needed.
    insta::assert_yaml_snapshot!("derived_pin_connectivity", body.pins);
}

#[test]
fn replace_schematic_round_trips_through_document() {
    let replace = ReplaceSchematic {
        instances: vec![SchematicInstanceInput {
            id: None,
            part_id: None,
            ref_des: "U1".into(),
            position: Some(Position { x: 80.0, y: 80.0 }),
            rotation: 0.0,
            meta: None,
        }],
        nets: vec![SchematicNetInput {
            id: None,
            name: "VCC".into(),
        }],
        pins: vec![SchematicPinInput {
            instance_ref: "U1".into(),
            pin_name: "1".into(),
            net_name: "VCC".into(),
        }],
    };
    let doc = SchematicDocument::from_replace_schematic(&replace);
    let (back, _) = doc.to_replace_schematic();
    assert_eq!(back.instances.len(), 1);
    insta::assert_yaml_snapshot!("round_tripped_replace", back, {
        ".instances[].id" => "[uuid]",
        ".instances[].part_id" => "[uuid]",
        ".nets[].id" => "[uuid]",
    });
}
