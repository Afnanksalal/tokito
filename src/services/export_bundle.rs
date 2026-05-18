//! Write export artifacts to a directory and optional ZIP archive.

use crate::error::AppResult;
use crate::models::{SchematicDocument, SchematicView};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub struct ExportWritten {
    pub paths: Vec<String>,
    pub zip_path: Option<PathBuf>,
}

pub fn write_design_exports(
    dir: &Path,
    base_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
    mcad_json: Option<&str>,
) -> AppResult<ExportWritten> {
    std::fs::create_dir_all(dir).map_err(|e| crate::error::AppError::Any(e.into()))?;
    let mut paths = Vec::new();

    let svg_path = dir.join(format!("{base_name}.svg"));
    std::fs::write(
        &svg_path,
        crate::services::svg_export::document_to_svg(document),
    )
    .map_err(|e| crate::error::AppError::Any(e.into()))?;
    paths.push(svg_path.to_string_lossy().into_owned());

    let pdf_path = dir.join(format!("{base_name}.pdf"));
    std::fs::write(
        &pdf_path,
        crate::services::pdf_export::document_to_pdf(document),
    )
    .map_err(|e| crate::error::AppError::Any(e.into()))?;
    paths.push(pdf_path.to_string_lossy().into_owned());

    let (body, _) = document.to_replace_schematic();
    let erc = crate::services::schematic_validate::erc_full_with_options(&body, document, false);
    let pack_path = dir.join(format!("{base_name}_pack.pdf"));
    std::fs::write(
        &pack_path,
        crate::services::pdf_export::document_to_pdf_pack(document, base_name, bom_csv, &erc),
    )
    .map_err(|e| crate::error::AppError::Any(e.into()))?;
    paths.push(pack_path.to_string_lossy().into_owned());

    let net_path = dir.join(format!("{base_name}.txt"));
    std::fs::write(&net_path, crate::services::netlist::connectivity_text(view))
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    paths.push(net_path.to_string_lossy().into_owned());

    let sexp_path = dir.join(format!("{base_name}.net"));
    std::fs::write(&sexp_path, crate::services::sexp_netlist::export(view))
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    paths.push(sexp_path.to_string_lossy().into_owned());

    let bom_path = dir.join(format!("{base_name}_bom.csv"));
    std::fs::write(&bom_path, bom_csv.as_bytes())
        .map_err(|e| crate::error::AppError::Any(e.into()))?;
    paths.push(bom_path.to_string_lossy().into_owned());

    if let Some(mcad) = mcad_json {
        let mcad_path = dir.join(format!("{base_name}_mcad.json"));
        std::fs::write(&mcad_path, mcad.as_bytes())
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
        paths.push(mcad_path.to_string_lossy().into_owned());
    }

    Ok(ExportWritten {
        paths,
        zip_path: None,
    })
}

/// Writes exports to `dir` and packs them into `{base_name}_bundle.zip`.
pub fn write_design_exports_zip(
    dir: &Path,
    base_name: &str,
    document: &SchematicDocument,
    view: &SchematicView,
    bom_csv: &str,
    mcad_json: Option<&str>,
) -> AppResult<ExportWritten> {
    let mut written = write_design_exports(dir, base_name, document, view, bom_csv, mcad_json)?;
    let zip_path = dir.join(format!("{base_name}_bundle.zip"));
    let file = File::create(&zip_path).map_err(|e| crate::error::AppError::Any(e.into()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for path_str in &written.paths {
        let path = Path::new(path_str);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("export.bin");
        let bytes = std::fs::read(path).map_err(|e| crate::error::AppError::Any(e.into()))?;
        zip.start_file(name, options)
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
        zip.write_all(&bytes)
            .map_err(|e| crate::error::AppError::Any(e.into()))?;
    }
    zip.finish()
        .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let zip_str = zip_path.to_string_lossy().into_owned();
    written.paths.push(zip_str.clone());
    written.zip_path = Some(zip_path);
    Ok(written)
}
