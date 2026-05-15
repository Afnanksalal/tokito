use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;

use super::{
    Position, ReplaceSchematic, SchematicInstanceInput, SchematicNetInput, SchematicPinInput,
    SchematicView,
};

pub const SCHEMATIC_DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_SHEET_ID: &str = "root";
const GRID: f64 = 40.0;
const PIN_SPACING: f64 = 20.0;
const SYMBOL_HALF_WIDTH: f64 = 70.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicDocument {
    pub schema_version: u32,
    pub sheets: Vec<SchematicSheet>,
    pub symbols: Vec<DocumentSymbol>,
    #[serde(default)]
    pub wire_segments: Vec<DocumentWireSegment>,
    #[serde(default)]
    pub junctions: Vec<DocumentJunction>,
    #[serde(default)]
    pub net_labels: Vec<DocumentNetLabel>,
    #[serde(default)]
    pub power_symbols: Vec<DocumentPowerSymbol>,
    #[serde(default)]
    pub no_connects: Vec<DocumentNoConnect>,
    #[serde(default)]
    pub text_items: Vec<DocumentTextItem>,
    #[serde(default)]
    pub buses: Vec<DocumentBusSegment>,
    #[serde(default)]
    pub erc_markers: Vec<DocumentErcMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicSheet {
    pub id: String,
    pub name: String,
    pub path: String,
    pub page_size: PageSize,
    pub grid: f64,
    #[serde(default)]
    pub title_block: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    pub id: Uuid,
    pub sheet_id: String,
    pub part_id: Option<Uuid>,
    pub symbol_id: Option<String>,
    pub ref_des: String,
    pub value: Option<String>,
    pub position: DocumentPoint,
    pub rotation: f64,
    pub mirror: MirrorMode,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
    pub footprint_ref: Option<String>,
    #[serde(default)]
    pub pins: Vec<DocumentPin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPin {
    pub number: Option<String>,
    pub name: String,
    pub electrical_type: ElectricalPinType,
    /// Pin location relative to the symbol origin before rotation/mirror.
    pub offset: DocumentPoint,
    pub orientation: PinOrientation,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWireSegment {
    pub id: Uuid,
    pub sheet_id: String,
    pub start: DocumentPoint,
    pub end: DocumentPoint,
    /// Optional explicit net seed, useful when importing older graph data.
    pub net_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentJunction {
    pub id: Uuid,
    pub sheet_id: String,
    pub position: DocumentPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentNetLabel {
    pub id: Uuid,
    pub sheet_id: String,
    pub name: String,
    pub kind: NetLabelKind,
    pub position: DocumentPoint,
    pub orientation: PinOrientation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPowerSymbol {
    pub id: Uuid,
    pub sheet_id: String,
    pub name: String,
    pub position: DocumentPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentNoConnect {
    pub id: Uuid,
    pub sheet_id: String,
    pub position: DocumentPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTextItem {
    pub id: Uuid,
    pub sheet_id: String,
    pub text: String,
    pub position: DocumentPoint,
    pub rotation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentBusSegment {
    pub id: Uuid,
    pub sheet_id: String,
    pub start: DocumentPoint,
    pub end: DocumentPoint,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentErcMarker {
    pub id: Uuid,
    pub sheet_id: String,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub position: DocumentPoint,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DocumentPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MirrorMode {
    #[default]
    None,
    X,
    Y,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ElectricalPinType {
    Input,
    Output,
    Bidirectional,
    TriState,
    Passive,
    PowerIn,
    PowerOut,
    OpenCollector,
    OpenEmitter,
    NoConnect,
    #[default]
    Unspecified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PinOrientation {
    Up,
    Down,
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetLabelKind {
    #[default]
    Local,
    Global,
    Hierarchical,
    NetClassDirective,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentDiagnostic {
    pub code: &'static str,
    pub message: String,
}

impl SchematicDocument {
    pub fn empty() -> Self {
        Self {
            schema_version: SCHEMATIC_DOCUMENT_SCHEMA_VERSION,
            sheets: vec![SchematicSheet::default_root()],
            symbols: vec![],
            wire_segments: vec![],
            junctions: vec![],
            net_labels: vec![],
            power_symbols: vec![],
            no_connects: vec![],
            text_items: vec![],
            buses: vec![],
            erc_markers: vec![],
        }
    }

    pub fn from_replace_schematic(s: &ReplaceSchematic) -> Self {
        let mut doc = Self::empty();
        let mut pins_by_ref: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut nets: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

        for pin in &s.pins {
            pins_by_ref
                .entry(pin.instance_ref.clone())
                .or_default()
                .insert(pin.pin_name.clone());
            nets.entry(pin.net_name.clone())
                .or_default()
                .push((pin.instance_ref.clone(), pin.pin_name.clone()));
        }

        doc.symbols = s
            .instances
            .iter()
            .enumerate()
            .map(|(idx, inst)| {
                let position = inst
                    .position
                    .as_ref()
                    .map(DocumentPoint::from)
                    .unwrap_or_else(|| DocumentPoint {
                        x: GRID * (idx as f64 + 2.0),
                        y: GRID * 3.0,
                    });
                let pin_names = pins_by_ref.remove(&inst.ref_des).unwrap_or_default();
                DocumentSymbol {
                    id: inst.id.unwrap_or_else(Uuid::new_v4),
                    sheet_id: DEFAULT_SHEET_ID.to_string(),
                    part_id: inst.part_id,
                    symbol_id: None,
                    ref_des: inst.ref_des.clone(),
                    value: None,
                    position,
                    rotation: inst.rotation,
                    mirror: MirrorMode::None,
                    fields: BTreeMap::new(),
                    footprint_ref: None,
                    pins: generic_pins(pin_names.into_iter().collect()),
                }
            })
            .collect();

        let mut pin_positions = HashMap::new();
        for symbol in &doc.symbols {
            for pin in &symbol.pins {
                pin_positions.insert(
                    (symbol.ref_des.clone(), pin.name.clone()),
                    symbol.absolute_pin_position(pin),
                );
            }
        }

        for (net_name, pins) in nets {
            let mut points: Vec<DocumentPoint> = pins
                .iter()
                .filter_map(|key| pin_positions.get(key).copied())
                .collect();
            points.dedup_by(|a, b| point_key(*a) == point_key(*b));
            for pair in points.windows(2) {
                let a = pair[0];
                let b = pair[1];
                let mid = DocumentPoint { x: b.x, y: a.y };
                doc.wire_segments.push(DocumentWireSegment {
                    id: Uuid::new_v4(),
                    sheet_id: DEFAULT_SHEET_ID.to_string(),
                    start: a,
                    end: mid,
                    net_name: Some(net_name.clone()),
                });
                doc.wire_segments.push(DocumentWireSegment {
                    id: Uuid::new_v4(),
                    sheet_id: DEFAULT_SHEET_ID.to_string(),
                    start: mid,
                    end: b,
                    net_name: Some(net_name.clone()),
                });
            }
            if points.len() == 1 {
                doc.net_labels.push(DocumentNetLabel {
                    id: Uuid::new_v4(),
                    sheet_id: DEFAULT_SHEET_ID.to_string(),
                    name: net_name,
                    kind: NetLabelKind::Local,
                    position: points[0],
                    orientation: PinOrientation::Right,
                });
            }
        }

        doc
    }

    pub fn from_view(view: &SchematicView) -> Self {
        let inst_id_to_ref: HashMap<Uuid, String> = view
            .instances
            .iter()
            .map(|i| (i.id, i.ref_des.clone()))
            .collect();
        let net_id_to_name: HashMap<Uuid, String> =
            view.nets.iter().map(|n| (n.id, n.name.clone())).collect();

        let replace = ReplaceSchematic {
            instances: view
                .instances
                .iter()
                .map(|i| SchematicInstanceInput {
                    id: Some(i.id),
                    part_id: i.part_id,
                    ref_des: i.ref_des.clone(),
                    position: match (i.pos_x, i.pos_y) {
                        (Some(x), Some(y)) => Some(Position { x, y }),
                        _ => None,
                    },
                    rotation: i.rotation,
                    meta: Some(i.meta.clone()),
                })
                .collect(),
            nets: view
                .nets
                .iter()
                .map(|n| SchematicNetInput {
                    id: Some(n.id),
                    name: n.name.clone(),
                })
                .collect(),
            pins: view
                .pins
                .iter()
                .filter_map(|p| {
                    Some(SchematicPinInput {
                        instance_ref: inst_id_to_ref.get(&p.instance_id)?.clone(),
                        pin_name: p.pin_name.clone(),
                        net_name: net_id_to_name.get(&p.net_id)?.clone(),
                    })
                })
                .collect(),
        };
        Self::from_replace_schematic(&replace)
    }

    pub fn to_replace_schematic(&self) -> (ReplaceSchematic, Vec<DocumentDiagnostic>) {
        let mut diagnostics = Vec::new();
        let mut dsu = DisjointSet::default();

        let no_connects: BTreeSet<PointKey> = self
            .no_connects
            .iter()
            .map(|nc| point_key(nc.position))
            .collect();

        let mut component_pins: BTreeMap<PointKey, Vec<(String, String)>> = BTreeMap::new();
        for symbol in &self.symbols {
            for pin in &symbol.pins {
                let p = point_key(symbol.absolute_pin_position(pin));
                dsu.make(p);
                component_pins
                    .entry(p)
                    .or_default()
                    .push((symbol.ref_des.clone(), pin.name.clone()));
            }
        }

        let mut semantic_points = BTreeSet::new();
        for point in component_pins.keys() {
            semantic_points.insert(*point);
        }
        for label in &self.net_labels {
            semantic_points.insert(point_key(label.position));
        }
        for power in &self.power_symbols {
            semantic_points.insert(point_key(power.position));
        }
        for junction in &self.junctions {
            semantic_points.insert(point_key(junction.position));
        }
        for no_connect in &self.no_connects {
            semantic_points.insert(point_key(no_connect.position));
        }

        for segment in &self.wire_segments {
            let mut points = vec![point_key(segment.start), point_key(segment.end)];
            for point in &semantic_points {
                if point_lies_on_segment(*point, segment.start, segment.end) {
                    points.push(*point);
                }
            }
            points.sort_by(|a, b| segment_point_order(*a, *b, segment));
            points.dedup();
            for pair in points.windows(2) {
                dsu.union(pair[0], pair[1]);
            }
        }

        let mut net_names: BTreeMap<PointKey, BTreeSet<String>> = BTreeMap::new();
        for segment in &self.wire_segments {
            if let Some(name) = normalized_net_name(segment.net_name.as_deref()) {
                let root = dsu.find(point_key(segment.start));
                net_names.entry(root).or_default().insert(name);
            }
        }
        for label in &self.net_labels {
            let root = dsu.find(point_key(label.position));
            net_names
                .entry(root)
                .or_default()
                .insert(label.name.trim().to_string());
        }
        for power in &self.power_symbols {
            let root = dsu.find(point_key(power.position));
            net_names
                .entry(root)
                .or_default()
                .insert(power.name.trim().to_string());
        }

        let mut pins_by_root: BTreeMap<PointKey, Vec<(String, String)>> = BTreeMap::new();
        for (point, pins) in component_pins {
            if no_connects.contains(&point) {
                continue;
            }
            let root = dsu.find(point);
            pins_by_root.entry(root).or_default().extend(pins);
        }

        let instances = self
            .symbols
            .iter()
            .map(|s| SchematicInstanceInput {
                id: Some(s.id),
                part_id: s.part_id,
                ref_des: s.ref_des.clone(),
                position: Some(Position {
                    x: s.position.x,
                    y: s.position.y,
                }),
                rotation: s.rotation,
                meta: Some(serde_json::json!({
                    "symbol_id": s.symbol_id,
                    "value": s.value,
                    "fields": s.fields,
                    "footprint_ref": s.footprint_ref,
                    "mirror": s.mirror,
                })),
            })
            .collect::<Vec<_>>();

        let mut pins = Vec::new();
        let mut net_set = BTreeSet::new();
        let mut unnamed_idx = 1usize;
        for (root, root_pins) in pins_by_root {
            if root_pins.is_empty() {
                continue;
            }
            let names = net_names.remove(&root).unwrap_or_default();
            let net_name = match names.len() {
                0 => {
                    let generated = format!("N${unnamed_idx}");
                    unnamed_idx += 1;
                    generated
                }
                1 => names.iter().next().unwrap().clone(),
                _ => {
                    let chosen = names.iter().next().unwrap().clone();
                    diagnostics.push(DocumentDiagnostic {
                        code: "DOC_CONFLICTING_NET_LABELS",
                        message: format!(
                            "Connected node has multiple net labels ({}); using '{}'",
                            names.into_iter().collect::<Vec<_>>().join(", "),
                            chosen
                        ),
                    });
                    chosen
                }
            };
            net_set.insert(net_name.clone());
            for (instance_ref, pin_name) in root_pins {
                pins.push(SchematicPinInput {
                    instance_ref,
                    pin_name,
                    net_name: net_name.clone(),
                });
            }
        }

        for names in net_names.values() {
            for name in names {
                net_set.insert(name.clone());
            }
        }

        let nets = net_set
            .into_iter()
            .map(|name| SchematicNetInput { id: None, name })
            .collect();

        (
            ReplaceSchematic {
                instances,
                nets,
                pins,
            },
            diagnostics,
        )
    }
}

impl SchematicSheet {
    pub fn default_root() -> Self {
        Self {
            id: DEFAULT_SHEET_ID.to_string(),
            name: "Root".to_string(),
            path: "/".to_string(),
            page_size: PageSize {
                width: 1160.0,
                height: 820.0,
            },
            grid: GRID,
            title_block: BTreeMap::new(),
        }
    }
}

impl DocumentSymbol {
    pub fn absolute_pin_position(&self, pin: &DocumentPin) -> DocumentPoint {
        let mut x = pin.offset.x;
        let mut y = pin.offset.y;
        match self.mirror {
            MirrorMode::None => {}
            MirrorMode::X => y = -y,
            MirrorMode::Y => x = -x,
        }

        let turns = ((self.rotation / 90.0).round() as i32).rem_euclid(4);
        let (rx, ry) = match turns {
            1 => (-y, x),
            2 => (-x, -y),
            3 => (y, -x),
            _ => (x, y),
        };
        DocumentPoint {
            x: snap(self.position.x + rx),
            y: snap(self.position.y + ry),
        }
    }
}

impl From<&Position> for DocumentPoint {
    fn from(value: &Position) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

type PointKey = (i64, i64);

fn point_key(point: DocumentPoint) -> PointKey {
    (
        (point.x / 0.001).round() as i64,
        (point.y / 0.001).round() as i64,
    )
}

fn point_from_key(point: PointKey) -> DocumentPoint {
    DocumentPoint {
        x: point.0 as f64 * 0.001,
        y: point.1 as f64 * 0.001,
    }
}

fn point_lies_on_segment(point: PointKey, start: DocumentPoint, end: DocumentPoint) -> bool {
    let p = point_from_key(point);
    let cross = (p.y - start.y) * (end.x - start.x) - (p.x - start.x) * (end.y - start.y);
    if cross.abs() > 0.001 {
        return false;
    }
    let min_x = start.x.min(end.x) - 0.001;
    let max_x = start.x.max(end.x) + 0.001;
    let min_y = start.y.min(end.y) - 0.001;
    let max_y = start.y.max(end.y) + 0.001;
    p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y
}

fn segment_point_order(a: PointKey, b: PointKey, segment: &DocumentWireSegment) -> Ordering {
    let pa = point_from_key(a);
    let pb = point_from_key(b);
    if (segment.end.x - segment.start.x).abs() >= (segment.end.y - segment.start.y).abs() {
        pa.x.partial_cmp(&pb.x).unwrap_or(Ordering::Equal)
    } else {
        pa.y.partial_cmp(&pb.y).unwrap_or(Ordering::Equal)
    }
}

fn snap(v: f64) -> f64 {
    (v / 0.001).round() * 0.001
}

fn normalized_net_name(name: Option<&str>) -> Option<String> {
    let trimmed = name?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn generic_pins(pin_names: Vec<String>) -> Vec<DocumentPin> {
    let count = pin_names.len().max(2);
    let top = -((count as f64 - 1.0) * PIN_SPACING) / 2.0;
    let names = if pin_names.is_empty() {
        vec!["1".to_string(), "2".to_string()]
    } else {
        pin_names
    };

    names
        .into_iter()
        .enumerate()
        .map(|(idx, name)| {
            let left_side = idx % 2 == 0;
            let row = idx / 2;
            DocumentPin {
                number: Some((idx + 1).to_string()),
                name,
                electrical_type: ElectricalPinType::Unspecified,
                offset: DocumentPoint {
                    x: if left_side {
                        -SYMBOL_HALF_WIDTH
                    } else {
                        SYMBOL_HALF_WIDTH
                    },
                    y: top + row as f64 * PIN_SPACING,
                },
                orientation: if left_side {
                    PinOrientation::Left
                } else {
                    PinOrientation::Right
                },
                visible: true,
            }
        })
        .collect()
}

#[derive(Default)]
struct DisjointSet {
    parent: BTreeMap<PointKey, PointKey>,
}

impl DisjointSet {
    fn make(&mut self, x: PointKey) {
        self.parent.entry(x).or_insert(x);
    }

    fn find(&mut self, x: PointKey) -> PointKey {
        self.make(x);
        let p = self.parent[&x];
        if p == x {
            return x;
        }
        let root = self.find(p);
        self.parent.insert(x, root);
        root
    }

    fn union(&mut self, a: PointKey, b: PointKey) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            let (small, large) = if ra <= rb { (ra, rb) } else { (rb, ra) };
            self.parent.insert(large, small);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_pin_doc() -> SchematicDocument {
        let mut doc = SchematicDocument::empty();
        doc.symbols.push(DocumentSymbol {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            part_id: None,
            symbol_id: Some("generic:R".to_string()),
            ref_des: "R1".to_string(),
            value: Some("10k".to_string()),
            position: DocumentPoint { x: 120.0, y: 120.0 },
            rotation: 0.0,
            mirror: MirrorMode::None,
            fields: BTreeMap::new(),
            footprint_ref: None,
            pins: generic_pins(vec!["1".to_string(), "2".to_string()]),
        });
        doc.symbols.push(DocumentSymbol {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            part_id: None,
            symbol_id: Some("generic:C".to_string()),
            ref_des: "C1".to_string(),
            value: Some("100n".to_string()),
            position: DocumentPoint { x: 320.0, y: 120.0 },
            rotation: 0.0,
            mirror: MirrorMode::None,
            fields: BTreeMap::new(),
            footprint_ref: None,
            pins: generic_pins(vec!["1".to_string(), "2".to_string()]),
        });
        doc
    }

    #[test]
    fn derives_replace_schematic_from_pin_geometry_and_label() {
        let mut doc = two_pin_doc();
        let r1_pin2 = doc.symbols[0].absolute_pin_position(&doc.symbols[0].pins[1]);
        let c1_pin1 = doc.symbols[1].absolute_pin_position(&doc.symbols[1].pins[0]);
        doc.wire_segments.push(DocumentWireSegment {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            start: r1_pin2,
            end: c1_pin1,
            net_name: None,
        });
        doc.net_labels.push(DocumentNetLabel {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            name: "SENSE".to_string(),
            kind: NetLabelKind::Local,
            position: r1_pin2,
            orientation: PinOrientation::Right,
        });

        let (replace, diagnostics) = doc.to_replace_schematic();
        assert!(diagnostics.is_empty());
        assert!(replace.nets.iter().any(|n| n.name == "SENSE"));
        assert!(replace
            .pins
            .iter()
            .any(|p| { p.instance_ref == "R1" && p.pin_name == "2" && p.net_name == "SENSE" }));
        assert!(replace
            .pins
            .iter()
            .any(|p| { p.instance_ref == "C1" && p.pin_name == "1" && p.net_name == "SENSE" }));
    }

    #[test]
    fn no_connect_excludes_pin_from_normalized_graph() {
        let mut doc = two_pin_doc();
        let p = doc.symbols[0].absolute_pin_position(&doc.symbols[0].pins[0]);
        doc.no_connects.push(DocumentNoConnect {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            position: p,
        });

        let (replace, _) = doc.to_replace_schematic();
        assert!(!replace
            .pins
            .iter()
            .any(|pin| pin.instance_ref == "R1" && pin.pin_name == "1"));
    }

    #[test]
    fn net_label_on_wire_interior_names_connected_pins() {
        let mut doc = two_pin_doc();
        let r1_pin2 = doc.symbols[0].absolute_pin_position(&doc.symbols[0].pins[1]);
        let c1_pin1 = doc.symbols[1].absolute_pin_position(&doc.symbols[1].pins[0]);
        let mid = DocumentPoint {
            x: (r1_pin2.x + c1_pin1.x) * 0.5,
            y: r1_pin2.y,
        };
        doc.wire_segments.push(DocumentWireSegment {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            start: r1_pin2,
            end: c1_pin1,
            net_name: None,
        });
        doc.net_labels.push(DocumentNetLabel {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            name: "MID_NET".to_string(),
            kind: NetLabelKind::Local,
            position: mid,
            orientation: PinOrientation::Right,
        });

        let (replace, diagnostics) = doc.to_replace_schematic();
        assert!(diagnostics.is_empty());
        assert!(replace
            .pins
            .iter()
            .any(|p| { p.instance_ref == "R1" && p.pin_name == "2" && p.net_name == "MID_NET" }));
        assert!(replace
            .pins
            .iter()
            .any(|p| { p.instance_ref == "C1" && p.pin_name == "1" && p.net_name == "MID_NET" }));
    }

    #[test]
    fn no_connect_on_wire_interior_does_not_remove_connected_pins() {
        let mut doc = two_pin_doc();
        let r1_pin2 = doc.symbols[0].absolute_pin_position(&doc.symbols[0].pins[1]);
        let c1_pin1 = doc.symbols[1].absolute_pin_position(&doc.symbols[1].pins[0]);
        let mid = DocumentPoint {
            x: (r1_pin2.x + c1_pin1.x) * 0.5,
            y: r1_pin2.y,
        };
        doc.wire_segments.push(DocumentWireSegment {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            start: r1_pin2,
            end: c1_pin1,
            net_name: Some("KEEP".to_string()),
        });
        doc.no_connects.push(DocumentNoConnect {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            position: mid,
        });

        let (replace, _) = doc.to_replace_schematic();
        assert!(replace
            .pins
            .iter()
            .any(|p| { p.instance_ref == "R1" && p.pin_name == "2" && p.net_name == "KEEP" }));
        assert!(replace
            .pins
            .iter()
            .any(|p| { p.instance_ref == "C1" && p.pin_name == "1" && p.net_name == "KEEP" }));
    }

    #[test]
    fn round_trips_replace_schematic_shape() {
        let replace = ReplaceSchematic {
            instances: vec![
                SchematicInstanceInput {
                    id: None,
                    part_id: None,
                    ref_des: "U1".to_string(),
                    position: Some(Position { x: 80.0, y: 120.0 }),
                    rotation: 0.0,
                    meta: None,
                },
                SchematicInstanceInput {
                    id: None,
                    part_id: None,
                    ref_des: "J1".to_string(),
                    position: Some(Position { x: 280.0, y: 120.0 }),
                    rotation: 0.0,
                    meta: None,
                },
            ],
            nets: vec![SchematicNetInput {
                id: None,
                name: "VCC".to_string(),
            }],
            pins: vec![
                SchematicPinInput {
                    instance_ref: "U1".to_string(),
                    pin_name: "VDD".to_string(),
                    net_name: "VCC".to_string(),
                },
                SchematicPinInput {
                    instance_ref: "J1".to_string(),
                    pin_name: "1".to_string(),
                    net_name: "VCC".to_string(),
                },
            ],
        };

        let doc = SchematicDocument::from_replace_schematic(&replace);
        let (normalized, diagnostics) = doc.to_replace_schematic();
        assert!(diagnostics.is_empty());
        assert_eq!(normalized.instances.len(), 2);
        assert!(normalized.nets.iter().any(|n| n.name == "VCC"));
        assert_eq!(normalized.pins.len(), 2);
    }
}
