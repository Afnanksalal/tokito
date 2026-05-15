//! Import a `.tokito_sym` tree into the user symbol library.
//!
//! `cargo run -p tokito-native --bin tokito-symbol-import -- <source_dir>`

use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let src: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("usage: tokito-symbol-import <source_dir>"))?;
    if !src.is_dir() {
        anyhow::bail!("not a directory: {}", src.display());
    }
    let n = tokito_native::symbol_library::import_folder(&src)?;
    println!(
        "Imported {n} symbol file(s) into {}",
        tokito_native::symbol_library::user_symbols_root().display()
    );
    Ok(())
}
