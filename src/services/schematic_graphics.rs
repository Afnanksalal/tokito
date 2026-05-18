//! Shared schematic plot geometry for SVG and PDF export.

use crate::models::{
    DocumentBusSegment, DocumentNetLabel, DocumentPoint, DocumentPowerSymbol, DocumentSymbol,
    DocumentWireSegment, NetLabelKind, SchematicDocument,
};

pub struct PlotTransform {
    pub min_x: f64,
    pub min_y: f64,
    pub scale: f64,
    pub margin: f64,
    pub page_h: f64,
}

impl PlotTransform {
    pub fn from_document(doc: &SchematicDocument, page_w: f64, page_h: f64, margin: f64) -> Self {
        let (min_x, min_y, max_x, max_y) = bounds(doc);
        let span_x = (max_x - min_x).max(1.0);
        let span_y = (max_y - min_y).max(1.0);
        let scale = ((page_w - 2.0 * margin) / span_x).min((page_h - 2.0 * margin) / span_y);
        Self {
            min_x,
            min_y,
            scale,
            margin,
            page_h,
        }
    }

    pub fn tx(&self, p: DocumentPoint) -> (f64, f64) {
        let x = self.margin + (p.x - self.min_x) * self.scale;
        let y = self.page_h - self.margin - (p.y - self.min_y) * self.scale;
        (x, y)
    }
}

pub fn bounds(doc: &SchematicDocument) -> (f64, f64, f64, f64) {
    let mut min_x = 0.0;
    let mut min_y = 0.0;
    let mut max_x = 400.0;
    let mut max_y = 300.0;
    let mut any = false;
    let mut expand = |x: f64, y: f64| {
        if !any {
            min_x = x;
            min_y = y;
            max_x = x;
            max_y = y;
            any = true;
        } else {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    };
    for seg in &doc.wire_segments {
        expand(seg.start.x, seg.start.y);
        expand(seg.end.x, seg.end.y);
    }
    for sym in &doc.symbols {
        expand(sym.position.x, sym.position.y);
    }
    for label in &doc.net_labels {
        expand(label.position.x, label.position.y);
    }
    for pwr in &doc.power_symbols {
        expand(pwr.position.x, pwr.position.y);
    }
    for bus in &doc.buses {
        expand(bus.start.x, bus.start.y);
        expand(bus.end.x, bus.end.y);
    }
    if !any {
        return (-100.0, -100.0, 500.0, 400.0);
    }
    (min_x - 40.0, min_y - 40.0, max_x + 40.0, max_y + 40.0)
}

pub fn wire_svg(seg: &DocumentWireSegment, t: &PlotTransform) -> String {
    let a = t.tx(seg.start);
    let b = t.tx(seg.end);
    format!(
        "<line class=\"wire\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
        a.0, a.1, b.0, b.1
    )
}

pub fn wire_pdf(seg: &DocumentWireSegment, t: &PlotTransform) -> String {
    let a = t.tx(seg.start);
    let b = t.tx(seg.end);
    format!("{:.2} {:.2} m {:.2} {:.2} l S\n", a.0, a.1, b.0, b.1)
}

pub fn symbol_svg(sym: &DocumentSymbol, t: &PlotTransform) -> String {
    let c = t.tx(sym.position);
    let mut s = format!(
        "<rect class=\"sym\" x=\"{:.1}\" y=\"{:.1}\" width=\"72\" height=\"32\" rx=\"2\"/>\n\
<text class=\"ref\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
        c.0 - 36.0,
        c.1 - 16.0,
        c.0,
        c.1 - 20.0,
        xml_escape(&sym.ref_des),
    );
    for pin in &sym.pins {
        let p = sym.absolute_pin_position(pin);
        let ps = t.tx(p);
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.5\" fill=\"#3a6ea5\"/>\n",
            ps.0, ps.1
        ));
    }
    s
}

pub fn symbol_pdf(sym: &DocumentSymbol, t: &PlotTransform) -> String {
    let c = t.tx(sym.position);
    let w = 36.0;
    let h = 16.0;
    let mut s = format!(
        "{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l S\n",
        c.0 - w,
        c.1 - h,
        c.0 + w,
        c.1 - h,
        c.0 + w,
        c.1 + h,
        c.0 - w,
        c.1 + h,
        c.0 - w,
        c.1 - h,
    );
    s.push_str(&format!(
        "BT /F1 9 Tf {:.2} {:.2} Td ({}) Tj ET\n",
        c.0 - w + 4.0,
        c.1 + 4.0,
        pdf_escape(&sym.ref_des)
    ));
    for pin in &sym.pins {
        let p = sym.absolute_pin_position(pin);
        let ps = t.tx(p);
        s.push_str(&format!(
            "{:.2} {:.2} m {:.2} {:.2} l S\n",
            ps.0 - 2.0,
            ps.1,
            ps.0 + 2.0,
            ps.1
        ));
    }
    s
}

pub fn label_svg(label: &DocumentNetLabel, t: &PlotTransform) -> String {
    let p = t.tx(label.position);
    let flag = match label.kind {
        NetLabelKind::Global => format!(
            "<polygon class=\"label\" points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"none\" stroke=\"#2a4a6a\"/>\n",
            p.0, p.1, p.0 - 14.0, p.1 - 6.0, p.0 - 14.0, p.1 + 6.0
        ),
        NetLabelKind::Hierarchical => format!(
            "<rect class=\"label\" x=\"{:.1}\" y=\"{:.1}\" width=\"28\" height=\"16\" fill=\"none\" stroke=\"#2a4a6a\"/>\n",
            p.0, p.1 - 8.0
        ),
        _ => format!(
            "<polygon class=\"label\" points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"none\" stroke=\"#2a4a6a\"/>\n",
            p.0, p.1, p.0 - 18.0, p.1 - 8.0, p.0 - 18.0, p.1
        ),
    };
    format!(
        "{flag}<text class=\"label\" x=\"{:.1}\" y=\"{:.1}\">{name}</text>\n",
        p.0 + 6.0,
        p.1 + 4.0,
        name = xml_escape(&label.name)
    )
}

pub fn power_svg(pwr: &DocumentPowerSymbol, t: &PlotTransform) -> String {
    let p = t.tx(pwr.position);
    let lower = pwr.name.to_ascii_lowercase();
    if lower.contains("gnd") {
        format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#22885a\"/>\n\
<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#22885a\"/>\n\
<text class=\"label\" x=\"{:.1}\" y=\"{:.1}\">{name}</text>\n",
            p.0,
            p.1,
            p.0,
            p.1 + 10.0,
            p.0 - 10.0,
            p.1 + 10.0,
            p.0 + 10.0,
            p.1 + 10.0,
            p.0 + 12.0,
            p.1 + 8.0,
            name = xml_escape(&pwr.name)
        )
    } else {
        format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#22885a\"/>\n\
<text class=\"label\" x=\"{:.1}\" y=\"{:.1}\">{name}</text>\n",
            p.0,
            p.1,
            p.0,
            p.1 + 14.0,
            p.0,
            p.1 - 4.0,
            name = xml_escape(&pwr.name)
        )
    }
}

pub fn label_pdf(label: &DocumentNetLabel, t: &PlotTransform) -> String {
    let p = t.tx(label.position);
    let mut s = String::new();
    match label.kind {
        NetLabelKind::Global => {
            s.push_str(&format!(
                "{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l S\n",
                p.0,
                p.1,
                p.0 - 14.0,
                p.1 - 6.0,
                p.0 - 14.0,
                p.1 + 6.0,
                p.0,
                p.1
            ));
        }
        NetLabelKind::Hierarchical => {
            s.push_str(&format!(
                "{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l S\n",
                p.0,
                p.1 - 8.0,
                p.0 + 28.0,
                p.1 - 8.0,
                p.0 + 28.0,
                p.1 + 8.0,
                p.0,
                p.1 + 8.0,
                p.0,
                p.1 - 8.0
            ));
        }
        _ => {
            s.push_str(&format!(
                "{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l S\n",
                p.0,
                p.1,
                p.0 - 18.0,
                p.1 - 8.0,
                p.0 - 18.0,
                p.1,
                p.0,
                p.1
            ));
        }
    }
    s.push_str(&format!(
        "BT /F1 8 Tf {:.2} {:.2} Td ({}) Tj ET\n",
        p.0 + 4.0,
        p.1 + 4.0,
        pdf_escape(&label.name)
    ));
    s
}

pub fn power_pdf(pwr: &DocumentPowerSymbol, t: &PlotTransform) -> String {
    let p = t.tx(pwr.position);
    let lower = pwr.name.to_ascii_lowercase();
    let mut s = String::new();
    if lower.contains("gnd") {
        s.push_str(&format!(
            "{:.2} {:.2} m {:.2} {:.2} l S\n{:.2} {:.2} m {:.2} {:.2} l S\n",
            p.0,
            p.1,
            p.0,
            p.1 + 10.0,
            p.0 - 10.0,
            p.1 + 10.0,
            p.0 + 10.0,
            p.1 + 10.0
        ));
    } else {
        s.push_str(&format!(
            "{:.2} {:.2} m {:.2} {:.2} l S\n",
            p.0,
            p.1,
            p.0,
            p.1 + 14.0
        ));
    }
    s.push_str(&format!(
        "BT /F1 8 Tf {:.2} {:.2} Td ({}) Tj ET\n",
        p.0,
        p.1 - 8.0,
        pdf_escape(&pwr.name)
    ));
    s
}

pub fn bus_svg(bus: &DocumentBusSegment, t: &PlotTransform) -> String {
    let a = t.tx(bus.start);
    let b = t.tx(bus.end);
    let mut s = format!(
        "<line class=\"wire\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke-width=\"4\" stroke=\"#584898\"/>\n",
        a.0, a.1, b.0, b.1
    );
    if let Some(name) = &bus.name {
        let mid_x = (a.0 + b.0) / 2.0;
        let mid_y = (a.1 + b.1) / 2.0 - 8.0;
        s.push_str(&format!(
            "<text class=\"label\" x=\"{mid_x:.1}\" y=\"{mid_y:.1}\">{name}</text>\n",
            name = xml_escape(name)
        ));
    }
    s
}

pub fn bus_pdf(bus: &DocumentBusSegment, t: &PlotTransform) -> String {
    let a = t.tx(bus.start);
    let b = t.tx(bus.end);
    let mut s = format!(
        "0.35 0.28 0.47 RG\n3 w\n{:.2} {:.2} m {:.2} {:.2} l S\n",
        a.0, a.1, b.0, b.1
    );
    if let Some(name) = &bus.name {
        s.push_str(&format!(
            "BT /F1 8 Tf {:.2} {:.2} Td ({}) Tj ET\n",
            (a.0 + b.0) / 2.0,
            (a.1 + b.1) / 2.0 - 8.0,
            pdf_escape(name)
        ));
    }
    s
}

pub fn title_block_pdf(design_name: &str, t: &PlotTransform, page_w: f64, _page_h: f64) -> String {
    let x = page_w - t.margin - 160.0;
    let y = t.margin + 8.0;
    format!(
        "0.7 w\n{:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l {:.2} {:.2} l h S\n\
BT /F1 10 Tf {:.2} {:.2} Td ({}) Tj ET\n\
BT /F1 8 Tf {:.2} {:.2} Td (Tokito review plot) Tj ET\n",
        x,
        y,
        x + 150.0,
        y,
        x + 150.0,
        y + 48.0,
        x,
        y + 48.0,
        x + 4.0,
        y + 14.0,
        pdf_escape(design_name),
        x + 4.0,
        y + 28.0,
    )
}

pub fn title_block_svg(design_name: &str, t: &PlotTransform, page_w: f64) -> String {
    let x = page_w - t.margin - 160.0;
    let y = t.margin + 8.0;
    format!(
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"150\" height=\"48\" fill=\"#fff\" stroke=\"#bbb\"/>\n\
<text class=\"ref\" x=\"{x:.1}\" y=\"{y:.1}\" font-size=\"10\">{name}</text>\n\
<text class=\"label\" x=\"{x:.1}\" y=\"{y2:.1}\" font-size=\"9\">Tokito review plot</text>\n",
        x = x,
        y = y + 14.0,
        name = xml_escape(design_name),
        y2 = y + 28.0,
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn pdf_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
