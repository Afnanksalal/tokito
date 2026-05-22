//! Studio UI panels (dock tabs, projects launcher, CAD shell).

mod bom;
mod build;
mod canvas;
mod chrome;
mod command_palette;
mod console;
mod design_manager;
mod inspector;
mod layout;
mod messages;
mod place_panel;
pub use place_panel::PlaceScope;
mod agent;
mod projects;
mod research;
mod settings;
pub use settings::SettingsSection;
mod shortcuts;
mod viewer3d;
