//! SVG plot export from editor-grade schematic documents.

use crate::models::{DocumentPoint, SchematicDocument, DEFAULT_SHEET_ID};

/// Render a readable SVG review plot for design review.
pub fn document_to_svg(doc: &SchematicDocument) -> String {
    let (min_x, min_y, max_x, max_y) = bounds(doc);
    let pad = 40.0;
    let w = (max_x - min_x + pad * 2.0).max(200.0);
    let h = (max_y - min_y + pad * 2.0).max(200.0);
    let ox = -min_x + pad;
    let oy = -min_y + pad;

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
            "<rect x=\"{ox:.1}\" y=\"{oy:.1}\" width=\"{fw:.1}\" height=\"{fh:.1}\" fill=\"none\" stroke=\"#ccc\" stroke-width=\"1\"/>\n"
        ));
    }

    for seg in &doc.wire_segments {
        let a = tx(seg.start, ox, oy);
        let b = tx(seg.end, ox, oy);
        s.push_str(&format!(
            "<line class=\"wire\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
            a.0, a.1, b.0, b.1
        ));
    }

    for sym in &doc.symbols {
        let c = tx(sym.position, ox, oy);
        s.push_str(&format!(
            "<rect class=\"sym\" x=\"{:.1}\" y=\"{:.1}\" width=\"80\" height=\"36\"/>\n\
<text class=\"ref\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            c.0 - 40.0,
            c.1 - 18.0,
            c.0,
            c.1 - 22.0,
            xml_escape(&sym.ref_des),
        ));
        for pin in &sym.pins {
            let p = sym.absolute_pin_position(pin);
            let ps = tx(p, ox, oy);
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2\" fill=\"#3a6ea5\"/>\n",
                ps.0, ps.1
            ));
        }
    }

    for label in &doc.net_labels {
        let p = tx(label.position, ox, oy);
        s.push_str(&format!(
            "<text class=\"label\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            p.0,
            p.1,
            xml_escape(&label.name),
        ));
    }

    s.push_str("</svg>\n");
    s
}

fn tx(p: DocumentPoint, ox: f64, oy: f64) -> (f64, f64) {
    (p.x + ox, p.y + oy)
}

fn bounds(doc: &SchematicDocument) -> (f64, f64, f64, f64) {
    let mut min_x = 0.0;
    let mut min_y = 0.0;
    let mut max_x = 400.0;
    let mut max_y = 300.0;
    let mut any = false;
    for sym in &doc.symbols {
        expand(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            sym.position.x,
            sym.position.y,
        );
        any = true;
    }
    for seg in &doc.wire_segments {
        expand(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            seg.start.x,
            seg.start.y,
        );
        expand(
            &mut min_x, &mut min_y, &mut max_x, &mut max_y, seg.end.x, seg.end.y,
        );
        any = true;
    }
    if !any {
        return (0.0, 0.0, 400.0, 300.0);
    }
    (min_x, min_y, max_x, max_y)
}

fn expand(min_x: &mut f64, min_y: &mut f64, max_x: &mut f64, max_y: &mut f64, x: f64, y: f64) {
    *min_x = min_x.min(x);
    *min_y = min_y.min(y);
    *max_x = max_x.max(x);
    *max_y = max_y.max(y);
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SchematicDocument;

    #[test]
    fn svg_is_well_formed_prefix() {
        let doc = SchematicDocument::empty();
        let svg = document_to_svg(&doc);
        assert!(svg.starts_with("<?xml"));
        assert!(svg.contains("</svg>"));
    }
}
