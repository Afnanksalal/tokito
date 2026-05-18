//! Shared export logic for HTTP handlers and native Studio.

use crate::error::{AppError, AppResult};
use crate::models::{SchematicDocument, SchematicView};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Netlist,
    SexpNetlist,
    Svg,
    Pdf,
    PdfPack,
    BundleZip,
    McadJson,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "csv" | "bom_csv" => Some(Self::Csv),
            "netlist" | "txt" => Some(Self::Netlist),
            "sexp_netlist" | "net" => Some(Self::SexpNetlist),
            "svg" => Some(Self::Svg),
            "pdf" => Some(Self::Pdf),
            "pdf_pack" | "pdfpack" => Some(Self::PdfPack),
            "bundle" | "zip" => Some(Self::BundleZip),
            "mcad" | "mcad_json" => Some(Self::McadJson),
            _ => None,
        }
    }

    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Netlist => "txt",
            Self::SexpNetlist => "net",
            Self::Svg => "svg",
            Self::Pdf | Self::PdfPack => "pdf",
            Self::BundleZip => "zip",
            Self::McadJson => "json",
        }
    }
}

pub struct ExportPayload {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub filename: String,
}

pub fn export_design_bytes(
    fmt: ExportFormat,
    design_id: Uuid,
    design_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
) -> AppResult<ExportPayload> {
    let safe = sanitize_filename(design_name);
    match fmt {
        ExportFormat::Svg => Ok(ExportPayload {
            bytes: crate::services::svg_export::document_to_svg_titled(document, design_name)
                .into_bytes(),
            content_type: "image/svg+xml; charset=utf-8",
            filename: format!("{safe}.svg"),
        }),
        ExportFormat::Pdf => Ok(ExportPayload {
            bytes: crate::services::pdf_export::document_to_pdf_titled(document, design_name),
            content_type: "application/pdf",
            filename: format!("{safe}.pdf"),
        }),
        ExportFormat::PdfPack => {
            let (body, _) = document.to_replace_schematic();
            let erc =
                crate::services::schematic_validate::erc_full_with_options(&body, document, false);
            Ok(ExportPayload {
                bytes: crate::services::pdf_export::document_to_pdf_pack(
                    document,
                    design_name,
                    bom_csv,
                    &erc,
                ),
                content_type: "application/pdf",
                filename: format!("{safe}_pack.pdf"),
            })
        }
        ExportFormat::Netlist => Ok(ExportPayload {
            bytes: crate::services::netlist::connectivity_text(view).into_bytes(),
            content_type: "text/plain; charset=utf-8",
            filename: format!("{safe}.txt"),
        }),
        ExportFormat::SexpNetlist => Ok(ExportPayload {
            bytes: crate::services::sexp_netlist::export(view).into_bytes(),
            content_type: "text/plain; charset=utf-8",
            filename: format!("{safe}.net"),
        }),
        ExportFormat::Csv => Ok(ExportPayload {
            bytes: bom_csv.as_bytes().to_vec(),
            content_type: "text/csv; charset=utf-8",
            filename: format!("{safe}_bom.csv"),
        }),
        ExportFormat::McadJson => {
            let json = crate::services::mcad_export::document_handoff_json(document, design_name);
            Ok(ExportPayload {
                bytes: json.into_bytes(),
                content_type: "application/json; charset=utf-8",
                filename: format!("{safe}_mcad.json"),
            })
        }
        ExportFormat::BundleZip => {
            let dir =
                std::env::temp_dir().join(format!("tokito_export_{design_id}_{}", Uuid::new_v4()));
            let mcad = crate::services::mcad_export::document_handoff_json(document, design_name);
            let result = crate::services::export_bundle::write_design_exports_zip(
                &dir,
                &safe,
                document,
                view,
                bom_csv,
                Some(&mcad),
            );
            let written = match result {
                Ok(written) => written,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(e);
                }
            };
            let zip_path = written
                .zip_path
                .ok_or_else(|| AppError::BadRequest("bundle zip missing".into()))?;
            let bytes = match std::fs::read(&zip_path).map_err(|e| AppError::Any(e.into())) {
                Ok(bytes) => bytes,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(e);
                }
            };
            let _ = std::fs::remove_dir_all(&dir);
            Ok(ExportPayload {
                bytes,
                content_type: "application/zip",
                filename: format!("{safe}_bundle.zip"),
            })
        }
        ExportFormat::Json => Err(AppError::BadRequest(
            "use design JSON export handler for full design dump".into(),
        )),
    }
}

pub fn export_to_directory(
    dir: &Path,
    base_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
    design_name: &str,
) -> AppResult<crate::services::export_bundle::ExportWritten> {
    let mcad = crate::services::mcad_export::document_handoff_json(document, design_name);
    crate::services::export_bundle::write_design_exports_zip(
        dir,
        base_name,
        document,
        view,
        bom_csv,
        Some(&mcad),
    )
}

pub fn dated_filename(base: &str, ext: &str) -> String {
    let safe = sanitize_filename(base);
    let stamp = chrono::Utc::now().format("%Y-%m-%d_%H%M");
    format!("{safe}_{stamp}.{ext}")
}

fn sanitize_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if safe.trim_matches('_').is_empty() {
        "design".to_string()
    } else {
        safe
    }
}
