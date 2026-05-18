//! SVG plot export from editor-grade schematic documents.

use crate::models::{SchematicDocument, DEFAULT_SHEET_ID};
use crate::services::schematic_graphics::{self, PlotTransform};

/// Render a readable SVG review plot for design review.
pub fn document_to_svg(doc: &SchematicDocument) -> String {
    document_to_svg_titled(doc, "Schematic")
}

pub fn document_to_svg_titled(doc: &SchematicDocument, design_name: &str) -> String {
    let (min_x, min_y, max_x, max_y) = schematic_graphics::bounds(doc);
    let pad = 40.0;
    let w = (max_x - min_x + pad * 2.0).max(200.0);
    let h = (max_y - min_y + pad * 2.0).max(200.0);
    let page_w = w;
    let page_h = h;
    let t = PlotTransform::from_document(doc, page_w, page_h, pad);

    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\">\n\
<style>\n\
  .wire {{ stroke: #3a5a7a; stroke-width: 1.5; fill: none; }}\n\
  .sym {{ stroke: #1a1a1a; stroke-width: 1.2; fill: none; }}\n\
  .label {{ font: 11px monospace; fill: #2a4a6a; }}\n\
  .ref {{ font: 12px monospace; fill: #111; }}\n\
</style>\n\
<rect width=\"100%\" height=\"100%\" fill=\"#f8f6f0\"/>\n"
    );

    if let Some(sheet) = doc.sheets.iter().find(|s| s.id == DEFAULT_SHEET_ID) {
        let fw = sheet.page_size.width;
        let fh = sheet.page_size.height;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{fw:.1}\" height=\"{fh:.1}\" fill=\"none\" stroke=\"#ccc\" stroke-width=\"1\"/>\n",
            pad - min_x + pad,
            pad - min_y + pad
        ));
    }

    s.push_str(&schematic_graphics::title_block_svg(
        design_name,
        &t,
        page_w,
    ));

    for seg in &doc.wire_segments {
        s.push_str(&schematic_graphics::wire_svg(seg, &t));
    }
    for bus in &doc.buses {
        s.push_str(&schematic_graphics::bus_svg(bus, &t));
    }
    for sym in &doc.symbols {
        s.push_str(&schematic_graphics::symbol_svg(sym, &t));
    }
    for label in &doc.net_labels {
        s.push_str(&schematic_graphics::label_svg(label, &t));
    }
    for pwr in &doc.power_symbols {
        s.push_str(&schematic_graphics::power_svg(pwr, &t));
    }

    s.push_str("</svg>\n");
    s
}
