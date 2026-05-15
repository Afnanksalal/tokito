//! Install-relative paths for release builds (exe + bundled `assets/`).

use std::env;
use std::path::PathBuf;

/// Directory containing `tokito-native.exe` (or the dev `target/...` binary).
pub fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Bundled symbols: `<exe_dir>/assets/base-symbols`, then compile-time tree for `cargo run`.
pub fn bundled_symbols_dir() -> PathBuf {
    let beside_exe = exe_dir().join("assets").join("base-symbols");
    if beside_exe.is_dir() {
        return beside_exe;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/base-symbols")
}
