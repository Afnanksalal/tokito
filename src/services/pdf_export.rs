//! Vector PDF export from `SchematicDocument` (review plot + multi-page pack).

use crate::models::{ErcViolation, SchematicDocument};
use crate::services::schematic_graphics::{self, PlotTransform};

/// A4 landscape in points.
const PAGE_W: f64 = 842.0;
const PAGE_H: f64 = 595.0;
const MARGIN: f64 = 36.0;

pub fn document_to_pdf(doc: &SchematicDocument) -> Vec<u8> {
    document_to_pdf_titled(doc, "Schematic")
}

pub fn document_to_pdf_titled(doc: &SchematicDocument, design_name: &str) -> Vec<u8> {
    let stream = schematic_page_stream(doc, design_name, PAGE_W, PAGE_H, MARGIN);
    build_pdf_multipage(&[(stream, PAGE_W, PAGE_H)])
}

/// Multi-page pack: schematic (review) + BOM table + ERC summary.
pub fn document_to_pdf_pack(
    doc: &SchematicDocument,
    design_name: &str,
    bom_csv: &str,
    erc: &[ErcViolation],
) -> Vec<u8> {
    let page1 = schematic_page_stream(doc, design_name, PAGE_W, PAGE_H, MARGIN);
    let page2 = bom_table_stream(bom_csv, design_name);
    let page3 = erc_summary_stream(erc, design_name);
    build_pdf_multipage(&[
        (page1, PAGE_W, PAGE_H),
        (page2, PAGE_W, PAGE_H),
        (page3, PAGE_W, PAGE_H),
    ])
}

fn schematic_page_stream(
    doc: &SchematicDocument,
    design_name: &str,
    page_w: f64,
    page_h: f64,
    margin: f64,
) -> String {
    let t = PlotTransform::from_document(doc, page_w, page_h, margin);
    let mut stream = String::new();
    stream.push_str("0.2 0.25 0.3 RG\n1 w\n");
    for seg in &doc.wire_segments {
        stream.push_str(&schematic_graphics::wire_pdf(seg, &t));
    }
    for bus in &doc.buses {
        stream.push_str(&schematic_graphics::bus_pdf(bus, &t));
    }
    stream.push_str("0.15 0.2 0.25 RG\n0.8 w\n");
    for sym in &doc.symbols {
        stream.push_str(&schematic_graphics::symbol_pdf(sym, &t));
    }
    stream.push_str("0 0.35 0.55 RG\n1 w\n");
    for label in &doc.net_labels {
        stream.push_str(&schematic_graphics::label_pdf(label, &t));
    }
    for pwr in &doc.power_symbols {
        stream.push_str(&schematic_graphics::power_pdf(pwr, &t));
    }
    stream.push_str(&schematic_graphics::title_block_pdf(
        design_name,
        &t,
        page_w,
        page_h,
    ));
    stream
}

fn bom_table_stream(bom_csv: &str, design_name: &str) -> String {
    let mut stream = String::new();
    let mut y = PAGE_H - MARGIN;
    stream.push_str(&format!(
        "BT /F1 14 Tf {:.2} {:.2} Td (BOM — {}) Tj ET\n",
        MARGIN,
        y,
        escape_pdf(design_name)
    ));
    y -= 28.0;
    stream.push_str(&format!(
        "BT /F1 9 Tf {MARGIN:.2} {y:.2} Td (MPN    Qty    Notes) Tj ET\n"
    ));
    y -= 16.0;
    for line in bom_csv.lines().skip(1).take(48) {
        if y < MARGIN + 20.0 {
            break;
        }
        let row = line.chars().take(90).collect::<String>();
        stream.push_str(&format!(
            "BT /F1 8 Tf {:.2} {:.2} Td ({}) Tj ET\n",
            MARGIN,
            y,
            escape_pdf(&row)
        ));
        y -= 12.0;
    }
    stream
}

fn erc_summary_stream(erc: &[ErcViolation], design_name: &str) -> String {
    let mut stream = String::new();
    let mut y = PAGE_H - MARGIN;
    stream.push_str(&format!(
        "BT /F1 14 Tf {:.2} {:.2} Td (ERC — {}) Tj ET\n",
        MARGIN,
        y,
        escape_pdf(design_name)
    ));
    y -= 28.0;
    if erc.is_empty() {
        stream.push_str(&format!(
            "BT /F1 10 Tf {MARGIN:.2} {y:.2} Td (No ERC violations.) Tj ET\n"
        ));
        return stream;
    }
    for v in erc.iter().take(40) {
        if y < MARGIN + 20.0 {
            break;
        }
        let row = format!("[{}] {}", v.code, v.message);
        let row = row.chars().take(100).collect::<String>();
        stream.push_str(&format!(
            "BT /F1 8 Tf {:.2} {:.2} Td ({}) Tj ET\n",
            MARGIN,
            y,
            escape_pdf(&row)
        ));
        y -= 12.0;
    }
    stream
}

fn build_pdf_multipage(pages: &[(String, f64, f64)]) -> Vec<u8> {
    let n = pages.len();
    let font_id = 3 + n * 2;
    let mut objects: Vec<String> = Vec::new();
    objects.push("1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n".into());

    let mut page_obj_ids: Vec<usize> = Vec::new();
    let mut content_obj_ids: Vec<usize> = Vec::new();
    for i in 0..n {
        page_obj_ids.push(3 + i * 2);
        content_obj_ids.push(4 + i * 2);
    }

    let mut kids: String = String::from("[");
    for id in &page_obj_ids {
        kids.push_str(&format!("{id} 0 R "));
    }
    kids.push(']');
    objects.push(format!(
        "2 0 obj<< /Type /Pages /Kids {kids} /Count {n} >>endobj\n"
    ));

    for (i, (stream, pw, ph)) in pages.iter().enumerate() {
        let page_id = page_obj_ids[i];
        let content_id = content_obj_ids[i];
        objects.push(format!(
            "{page_id} 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.0} {ph:.0}] /Contents {content_id} 0 R /Resources<< /Font<< /F1 {font_id} 0 R >> >> >>endobj\n"
        ));
        objects.push(format!(
            "{content_id} 0 obj<< /Length {} >>stream\n{stream}endstream\nendobj\n",
            stream.len(),
            stream = stream
        ));
    }

    objects.push(format!(
        "{font_id} 0 obj<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>endobj\n"
    ));

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
        pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
        objects.len() + 1
    ));
    pdf.into_bytes()
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

    #[test]
    fn pdf_pack_has_multiple_pages() {
        let doc = SchematicDocument::empty();
        let pdf = document_to_pdf_pack(&doc, "Test", "mpn,qty,notes\nR1,1,\n", &[]);
        assert!(pdf.windows(5).filter(|w| *w == b"/Type").count() >= 3);
    }
}
