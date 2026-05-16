//! Minimal vector PDF export from `SchematicDocument` (plot for review).

use crate::models::SchematicDocument;

/// A4 landscape-ish page in points; schematic coords scaled to fit.
pub fn document_to_pdf(doc: &SchematicDocument) -> Vec<u8> {
    let page_w = 842.0;
    let page_h = 595.0;
    let margin = 36.0;

    let (min_x, min_y, max_x, max_y) = bounds(doc);
    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    let scale = ((page_w - 2.0 * margin) / span_x).min((page_h - 2.0 * margin) / span_y);

    fn tx(x: f64, min_x: f64, scale: f64, margin: f64) -> f64 {
        margin + (x - min_x) * scale
    }
    fn ty(y: f64, min_y: f64, scale: f64, margin: f64, page_h: f64) -> f64 {
        page_h - margin - (y - min_y) * scale
    }

    let mut stream = String::new();
    stream.push_str("0.2 0.25 0.3 RG\n");
    stream.push_str("1 w\n");
    for seg in &doc.wire_segments {
        let x1 = tx(seg.start.x, min_x, scale, margin);
        let y1 = ty(seg.start.y, min_y, scale, margin, page_h);
        let x2 = tx(seg.end.x, min_x, scale, margin);
        let y2 = ty(seg.end.y, min_y, scale, margin, page_h);
        stream.push_str(&format!("{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S\n"));
    }
    stream.push_str("0 0 0 rg\n");
    for sym in &doc.symbols {
        let x = tx(sym.position.x, min_x, scale, margin);
        let y = ty(sym.position.y, min_y, scale, margin, page_h);
        stream.push_str(&format!(
            "BT /F1 9 Tf {x:.2} {y:.2} Td ({}) Tj ET\n",
            escape_pdf(&sym.ref_des)
        ));
    }

    let stream_bytes = stream.as_bytes();
    let mut objects: Vec<String> = Vec::new();
    objects.push("1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n".into());
    objects.push("2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n".into());
    objects.push(format!(
        "3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.0} {page_h:.0}] /Contents 4 0 R /Resources<< /Font<< /F1 5 0 R >> >> >>endobj\n"
    ));
    objects.push(format!(
        "4 0 obj<< /Length {} >>stream\n{stream}endstream\nendobj\n",
        stream_bytes.len(),
        stream = stream
    ));
    objects.push("5 0 obj<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>endobj\n".into());

    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = vec![0usize];
    for obj in &objects {
        offsets.push(pdf.len());
        pdf.push_str(obj);
    }
    let xref_start = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n", objects.len() + 1));
    pdf.push_str("0000000000 65535 f \n");
    for off in &offsets[1..] {
        pdf.push_str(&format!("{:010} 00000 n \n", off));
    }
    pdf.push_str(&format!(
        "trailer<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
        objects.len() + 1
    ));
    pdf.into_bytes()
}

fn bounds(doc: &SchematicDocument) -> (f64, f64, f64, f64) {
    let mut min_x = 0.0;
    let mut min_y = 0.0;
    let mut max_x = 400.0;
    let mut max_y = 300.0;
    let mut any = false;
    for seg in &doc.wire_segments {
        for p in [seg.start, seg.end] {
            if !any {
                min_x = p.x;
                min_y = p.y;
                max_x = p.x;
                max_y = p.y;
                any = true;
            } else {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }
    }
    for sym in &doc.symbols {
        let p = sym.position;
        if !any {
            min_x = p.x;
            min_y = p.y;
            max_x = p.x;
            max_y = p.y;
            any = true;
        } else {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    if !any {
        return (-100.0, -100.0, 500.0, 400.0);
    }
    (min_x - 40.0, min_y - 40.0, max_x + 40.0, max_y + 40.0)
}

fn escape_pdf(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DocumentPoint, DocumentWireSegment, SchematicDocument};

    #[test]
    fn pdf_starts_with_header() {
        let mut doc = SchematicDocument::empty();
        doc.wire_segments.push(DocumentWireSegment {
            id: uuid::Uuid::new_v4(),
            sheet_id: "root".into(),
            start: DocumentPoint { x: 0.0, y: 0.0 },
            end: DocumentPoint { x: 100.0, y: 0.0 },
            net_name: Some("NET".into()),
            net_id: None,
            start_pin: None,
            end_pin: None,
        });
        let pdf = document_to_pdf(&doc);
        assert!(pdf.starts_with(b"%PDF"));
    }
}
