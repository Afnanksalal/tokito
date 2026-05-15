//! Generate expanded bundled symbol library under `assets/base-symbols/`.
//!
//! Usage: `cargo run -p tokito-native --bin generate-base-symbols`

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/base-symbols");
    let n = tokito_native::symbol_gen::generate_library(&out)?;
    println!(
        "Generated {n} new .tokito_sym files under {}",
        out.display()
    );
    Ok(())
}
