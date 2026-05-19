//! Single integration-test binary. Hosts every previously-separate `tests/*.rs`
//! file as a submodule so the embedded Postgres cluster (managed in
//! `tokito::test_support`) is started **once** per `cargo test` run instead of
//! once per binary.

mod ai_pipeline_fixtures;
mod api_designs;
mod api_parts;
mod api_schematic;
mod db_stability;
mod golden_document;
mod golden_netlist_move;
mod notes_research;
mod project_workspace;
mod services_exports;
mod spec_compliance;
