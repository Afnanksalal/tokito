//! Tokito native desktop shell (eframe + egui).

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod base_symbols;
mod bootstrap;
mod canvas;
mod component_value;
mod editor;
mod mcad_viewer;
mod paths;
mod symbol_format;
mod symbol_library;
mod symbols_draw;
mod theme;
mod ui;
mod util;

fn main() -> anyhow::Result<()> {
    let app = app::App::try_new()?;
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Tokito")
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tokito",
        native_options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))
}
