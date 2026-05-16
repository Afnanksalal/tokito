//! User symbol library paths and import pipeline (`.tokito_sym` / `.kicad_sym` folders).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use walkdir::WalkDir;

use crate::symbol_format::SymbolLibFile;

pub fn user_symbols_root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tokito")
        .join("symbols")
}

/// Copy symbol library files from `src` into the user library, preserving subfolders.
/// `.kicad_sym` files are validated and stored as `.tokito_sym` (equivalent s-expression).
pub fn import_folder(src: &Path) -> anyhow::Result<usize> {
    let dst_root = user_symbols_root();
    fs::create_dir_all(&dst_root)?;
    let mut count = 0usize;
    for entry in WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if !SymbolLibFile::is_library_path(path) {
            continue;
        }
        let text = fs::read_to_string(path).context(format!("read {}", path.display()))?;
        SymbolLibFile::parse(&text).context(format!("parse {}", path.display()))?;
        let rel = path.strip_prefix(src).unwrap_or(path).to_path_buf();
        let mut out = dst_root.join(rel);
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == SymbolLibFile::IMPORT_SYM_EXT)
        {
            out.set_extension(SymbolLibFile::EXTENSION);
            let normalized = SymbolLibFile::normalize_to_tokito_format(&text);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&out, normalized)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &out)?;
        }
        count += 1;
    }
    Ok(count)
}

pub fn bundled_symbols_dir() -> PathBuf {
    crate::paths::bundled_symbols_dir()
}
