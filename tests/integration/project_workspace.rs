//! Project workspace paths and export naming (§7).

use tokito::paths;

#[test]
fn project_exports_dir_under_workspace() {
    let ws = paths::project_dir("my_board");
    let exports = paths::project_exports_dir(&ws);
    assert!(exports.starts_with(&ws));
    assert!(exports.ends_with("exports"));
}

#[test]
fn dated_export_filename_includes_timestamp() {
    let name = tokito::services::export_service::dated_filename("My Design", "pdf");
    assert!(name.starts_with("My_Design_"));
    assert!(name.ends_with(".pdf"));
    assert!(name.len() > "My_Design_.pdf".len());
}

#[test]
fn slugify_produces_stable_folder_name() {
    assert_eq!(paths::slugify_name("My Board!"), "my_board");
}
