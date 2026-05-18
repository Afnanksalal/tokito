use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;

use super::{
    Position, ReplaceSchematic, SchematicInstanceInput, SchematicNetInput, SchematicPinInput,
    SchematicView,
};
use crate::connectivity::{point_key_xy, DisjointSet, PointKey};

pub const SCHEMATIC_DOCUMENT_SCHEMA_VERSION: u32 = 2;

const NET_LABEL_LINK_FIELD: &str = "tokito_net_label_id";
const POWER_SYMBOL_LINK_FIELD: &str = "tokito_power_id";
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

/// Pin anchor for a wire endpoint (schematic instance refdes + pin name).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentPinAnchor {
    pub ref_des: String,
    pub pin_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWireSegment {
    pub id: Uuid,
    pub sheet_id: String,
    pub start: DocumentPoint,
    pub end: DocumentPoint,
    /// Optional explicit net seed, useful when importing older graph data.
    pub net_name: Option<String>,
    /// Stable topological net id (assigned by connectivity rebuild).
    #[serde(default)]
    pub net_id: Option<Uuid>,
    /// When set, `start` is derived from this pin when the symbol moves.
    #[serde(default)]
    pub start_pin: Option<DocumentPinAnchor>,
    #[serde(default)]
    pub end_pin: Option<DocumentPinAnchor>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_name: Option<String>,
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
    /// Apply schema migrations (K.5: v1 labels/power → aux symbol instances).
    pub fn upgrade_to_current(mut doc: Self) -> Self {
        if doc.schema_version >= SCHEMATIC_DOCUMENT_SCHEMA_VERSION {
            return doc;
        }
        if doc.schema_version < 2 {
            upgrade_v1_to_v2(&mut doc);
        }
        doc.schema_version = SCHEMATIC_DOCUMENT_SCHEMA_VERSION;
        doc
    }

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
                    net_id: None,
                    start_pin: None,
                    end_pin: None,
                });
                doc.wire_segments.push(DocumentWireSegment {
                    id: Uuid::new_v4(),
                    sheet_id: DEFAULT_SHEET_ID.to_string(),
                    start: mid,
                    end: b,
                    net_name: Some(net_name.clone()),
                    net_id: None,
                    start_pin: None,
                    end_pin: None,
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

        for segment in &self.wire_segments {
            let sheet_points = semantic_points_on_sheet(self, &segment.sheet_id);
            let mut points = vec![point_key(segment.start), point_key(segment.end)];
            for point in &sheet_points {
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

        apply_cross_sheet_label_links(self, &mut dsu);

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

fn point_key(point: DocumentPoint) -> PointKey {
    point_key_xy(point.x, point.y)
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

fn semantic_points_on_sheet(doc: &SchematicDocument, sheet_id: &str) -> BTreeSet<PointKey> {
    let mut set = BTreeSet::new();
    for symbol in doc.symbols.iter().filter(|s| s.sheet_id == sheet_id) {
        for pin in &symbol.pins {
            set.insert(point_key(symbol.absolute_pin_position(pin)));
        }
    }
    for label in doc.net_labels.iter().filter(|l| l.sheet_id == sheet_id) {
        set.insert(point_key(label.position));
    }
    for power in doc.power_symbols.iter().filter(|p| p.sheet_id == sheet_id) {
        set.insert(point_key(power.position));
    }
    for junction in doc.junctions.iter().filter(|j| j.sheet_id == sheet_id) {
        set.insert(point_key(junction.position));
    }
    for nc in doc.no_connects.iter().filter(|n| n.sheet_id == sheet_id) {
        set.insert(point_key(nc.position));
    }
    set
}

/// `ChildSheet/LocalNet` on a parent sheet links to `LocalNet` on the child sheet.
fn parse_hierarchical_label(name: &str) -> Option<(String, String)> {
    let name = name.trim();
    let (sheet, net) = name.split_once('/')?;
    let sheet = sheet.trim();
    let net = net.trim();
    if sheet.is_empty() || net.is_empty() {
        return None;
    }
    Some((sheet.to_string(), net.to_string()))
}

fn apply_cross_sheet_label_links(doc: &SchematicDocument, dsu: &mut DisjointSet) {
    let mut global_by_name: BTreeMap<String, Vec<PointKey>> = BTreeMap::new();
    for label in &doc.net_labels {
        if label.kind != NetLabelKind::Global {
            continue;
        }
        let key = label.name.trim().to_string();
        if key.is_empty() {
            continue;
        }
        let pt = point_key(label.position);
        dsu.make(pt);
        global_by_name.entry(key).or_default().push(pt);
    }
    for points in global_by_name.values() {
        if let Some(&first) = points.first() {
            for &p in &points[1..] {
                dsu.union(first, p);
            }
        }
    }

    let sheet_ids: BTreeSet<String> = doc.sheets.iter().map(|s| s.id.clone()).collect();
    for label in &doc.net_labels {
        if label.kind != NetLabelKind::Hierarchical {
            continue;
        }
        let Some((child_sheet, local_net)) = parse_hierarchical_label(&label.name) else {
            continue;
        };
        if !sheet_ids.contains(&child_sheet) {
            continue;
        }
        let parent_pt = point_key(label.position);
        dsu.make(parent_pt);
        for other in &doc.net_labels {
            if other.sheet_id != child_sheet {
                continue;
            }
            let matches = other.name.trim() == local_net.as_str()
                || other.name.trim() == format!("{child_sheet}/{local_net}");
            if matches {
                let child_pt = point_key(other.position);
                dsu.make(child_pt);
                dsu.union(parent_pt, child_pt);
            }
        }
    }
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
            net_id: None,
            start_pin: None,
            end_pin: None,
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
            net_id: None,
            start_pin: None,
            end_pin: None,
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
    fn global_net_labels_connect_across_sheets() {
        let mut doc = SchematicDocument::empty();
        doc.sheets.push(SchematicSheet {
            id: "S2".into(),
            name: "Power".into(),
            path: "/S2".into(),
            page_size: PageSize {
                width: 1160.0,
                height: 820.0,
            },
            grid: GRID,
            title_block: BTreeMap::new(),
        });
        doc.symbols.push(DocumentSymbol {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            part_id: None,
            symbol_id: None,
            ref_des: "R1".into(),
            value: None,
            position: DocumentPoint { x: 80.0, y: 80.0 },
            rotation: 0.0,
            mirror: MirrorMode::None,
            fields: BTreeMap::new(),
            footprint_ref: None,
            pins: generic_pins(vec!["1".into(), "2".into()]),
        });
        doc.symbols.push(DocumentSymbol {
            id: Uuid::new_v4(),
            sheet_id: "S2".into(),
            part_id: None,
            symbol_id: None,
            ref_des: "R2".into(),
            value: None,
            position: DocumentPoint { x: 80.0, y: 80.0 },
            rotation: 0.0,
            mirror: MirrorMode::None,
            fields: BTreeMap::new(),
            footprint_ref: None,
            pins: generic_pins(vec!["1".into(), "2".into()]),
        });
        let r1_pin = doc.symbols[0].absolute_pin_position(&doc.symbols[0].pins[0]);
        let r2_pin = doc.symbols[1].absolute_pin_position(&doc.symbols[1].pins[0]);
        doc.net_labels.push(DocumentNetLabel {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            name: "GND".into(),
            kind: NetLabelKind::Global,
            position: r1_pin,
            orientation: PinOrientation::Right,
        });
        doc.net_labels.push(DocumentNetLabel {
            id: Uuid::new_v4(),
            sheet_id: "S2".into(),
            name: "GND".into(),
            kind: NetLabelKind::Global,
            position: r2_pin,
            orientation: PinOrientation::Right,
        });

        let (replace, _) = doc.to_replace_schematic();
        let gnd_pins: Vec<_> = replace
            .pins
            .iter()
            .filter(|p| p.net_name == "GND")
            .collect();
        assert!(
            gnd_pins.iter().any(|p| p.instance_ref == "R1"),
            "R1 should be on GND"
        );
        assert!(
            gnd_pins.iter().any(|p| p.instance_ref == "R2"),
            "R2 on sheet 2 should share global GND"
        );
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
            net_id: None,
            start_pin: None,
            end_pin: None,
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

    #[test]
    fn upgrade_v1_adds_aux_symbols_for_labels_and_power() {
        let mut doc = SchematicDocument::empty();
        doc.schema_version = 1;
        let label_id = Uuid::new_v4();
        doc.net_labels.push(DocumentNetLabel {
            id: label_id,
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            name: "SDA".into(),
            kind: NetLabelKind::Global,
            position: DocumentPoint { x: 120.0, y: 40.0 },
            orientation: PinOrientation::Right,
        });
        doc.power_symbols.push(DocumentPowerSymbol {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            name: "GND".into(),
            position: DocumentPoint { x: 80.0, y: 200.0 },
        });
        let upgraded = SchematicDocument::upgrade_to_current(doc);
        assert_eq!(upgraded.schema_version, 2);
        assert_eq!(upgraded.net_labels.len(), 1);
        assert!(upgraded
            .symbols
            .iter()
            .any(|s| s.symbol_id.as_deref() == Some("aux:Label_Global")));
        assert!(upgraded
            .symbols
            .iter()
            .any(|s| s.symbol_id.as_deref() == Some("aux:Power_GND")));
        assert!(upgraded.symbols.iter().any(|s| {
            s.fields
                .get(NET_LABEL_LINK_FIELD)
                .map(|v| v == &label_id.to_string())
                .unwrap_or(false)
        }));
    }

    #[test]
    fn erc_markers_persist_instance_ref_through_json_round_trip() {
        let mut doc = SchematicDocument::empty();
        doc.erc_markers.push(DocumentErcMarker {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            severity: "error".into(),
            code: "ERC001".into(),
            message: "test".into(),
            position: DocumentPoint { x: 10.0, y: 20.0 },
            instance_ref: Some("U1".into()),
            net_name: Some("VCC".into()),
        });
        let json = serde_json::to_string(&doc).unwrap();
        let doc2: SchematicDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc2.erc_markers.len(), 1);
        assert_eq!(doc2.erc_markers[0].instance_ref.as_deref(), Some("U1"));
        assert_eq!(doc2.erc_markers[0].net_name.as_deref(), Some("VCC"));
    }

    #[test]
    fn wire_segment_pin_anchor_json_round_trip() {
        let seg = DocumentWireSegment {
            id: Uuid::new_v4(),
            sheet_id: DEFAULT_SHEET_ID.to_string(),
            start: DocumentPoint { x: 0.0, y: 0.0 },
            end: DocumentPoint { x: 40.0, y: 0.0 },
            net_name: Some("NET_A".into()),
            net_id: Some(Uuid::new_v4()),
            start_pin: Some(DocumentPinAnchor {
                ref_des: "R1".into(),
                pin_name: "2".into(),
            }),
            end_pin: None,
        };
        let json = serde_json::to_string(&seg).unwrap();
        let back: DocumentWireSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.start_pin.as_ref().unwrap().ref_des, "R1");
        assert_eq!(back.start_pin.as_ref().unwrap().pin_name, "2");
        assert!(back.end_pin.is_none());
        assert_eq!(back.net_id, seg.net_id);
    }
}

fn upgrade_v1_to_v2(doc: &mut SchematicDocument) {
    let mut lbl_idx = doc
        .symbols
        .iter()
        .filter(|s| s.ref_des.starts_with("LBL"))
        .count();
    for label in &doc.net_labels {
        if doc.symbols.iter().any(|s| {
            s.fields
                .get(NET_LABEL_LINK_FIELD)
                .map(|v| v == &label.id.to_string())
                .unwrap_or(false)
        }) {
            continue;
        }
        lbl_idx += 1;
        let lib = aux_library_for_label(label.kind);
        doc.symbols.push(DocumentSymbol {
            id: Uuid::new_v4(),
            sheet_id: label.sheet_id.clone(),
            part_id: None,
            symbol_id: Some(lib.into()),
            ref_des: format!("LBL{lbl_idx:04}"),
            value: Some(label.name.clone()),
            position: label.position,
            rotation: orientation_degrees(label.orientation),
            mirror: MirrorMode::None,
            fields: BTreeMap::from([(
                NET_LABEL_LINK_FIELD.to_string(),
                label.id.to_string(),
            )]),
            footprint_ref: None,
            pins: vec![aux_attachment_pin()],
        });
    }

    let mut pwr_idx = doc
        .symbols
        .iter()
        .filter(|s| s.ref_des.starts_with("PWR"))
        .count();
    for pwr in &doc.power_symbols {
        if doc.symbols.iter().any(|s| {
            s.fields
                .get(POWER_SYMBOL_LINK_FIELD)
                .map(|v| v == &pwr.id.to_string())
                .unwrap_or(false)
        }) {
            continue;
        }
        pwr_idx += 1;
        doc.symbols.push(DocumentSymbol {
            id: Uuid::new_v4(),
            sheet_id: pwr.sheet_id.clone(),
            part_id: None,
            symbol_id: Some(aux_library_for_power(&pwr.name).into()),
            ref_des: format!("PWR{pwr_idx:04}"),
            value: Some(pwr.name.clone()),
            position: pwr.position,
            rotation: 0.0,
            mirror: MirrorMode::None,
            fields: BTreeMap::from([(
                POWER_SYMBOL_LINK_FIELD.to_string(),
                pwr.id.to_string(),
            )]),
            footprint_ref: None,
            pins: vec![aux_attachment_pin()],
        });
    }
}

fn aux_library_for_label(kind: NetLabelKind) -> &'static str {
    match kind {
        NetLabelKind::Global => "aux:Label_Global",
        NetLabelKind::Hierarchical => "aux:Label_Hierarchical",
        NetLabelKind::Local | NetLabelKind::NetClassDirective => "aux:Label_Local",
    }
}

fn aux_library_for_power(name: &str) -> &'static str {
    let u = name.to_ascii_uppercase();
    if u.contains("GND") {
        "aux:Power_GND"
    } else if u.contains("3V3") || u.contains("3.3") {
        "aux:Power_VCC_3V3"
    } else {
        "aux:Power_VCC"
    }
}

fn orientation_degrees(o: PinOrientation) -> f64 {
    match o {
        PinOrientation::Up => 270.0,
        PinOrientation::Down => 90.0,
        PinOrientation::Left => 180.0,
        PinOrientation::Right => 0.0,
    }
}

fn aux_attachment_pin() -> DocumentPin {
    DocumentPin {
        number: Some("1".into()),
        name: "1".into(),
        electrical_type: ElectricalPinType::Passive,
        offset: DocumentPoint { x: 0.0, y: 0.0 },
        orientation: PinOrientation::Right,
        visible: true,
    }
}
