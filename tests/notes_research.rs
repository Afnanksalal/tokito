//! Research and BOM notes persistence (§7). Requires `TOKITO_RUN_DB_INTEGRATION=1`.

#[tokio::test]
async fn manual_research_note_crud() -> anyhow::Result<()> {
    if !tokito::test_support::database_integration_tests_enabled() {
        return Ok(());
    }
    let pool = tokito::test_support::test_pool().await?;
    let design_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO designs (id, name) VALUES ($1, $2)")
        .bind(design_id)
        .bind("notes-test")
        .execute(&pool)
        .await?;

    let created = tokito::store::research::insert(
        &pool,
        design_id,
        tokito::store::research::KIND_MANUAL_NOTE,
        Some("Bring-up"),
        None,
        "Check decoupling on VCC.",
        serde_json::json!({}),
    )
    .await?;

    let updated = tokito::store::research::update_manual(
        &pool,
        created.id,
        Some("Bring-up v2"),
        "Check decoupling on VCC and GND.",
    )
    .await?;
    assert_eq!(updated.title.as_deref(), Some("Bring-up v2"));

    let listed = tokito::store::research::list_for_design(&pool, design_id, 10).await?;
    assert!(listed.iter().any(|a| a.id == created.id));

    tokito::store::research::delete_artifact(&pool, created.id).await?;
    let after = tokito::store::research::list_for_design(&pool, design_id, 10).await?;
    assert!(!after.iter().any(|a| a.id == created.id));
    Ok(())
}

#[tokio::test]
async fn bom_line_notes_patch() -> anyhow::Result<()> {
    if !tokito::test_support::database_integration_tests_enabled() {
        return Ok(());
    }
    let pool = tokito::test_support::test_pool().await?;
    let design_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO designs (id, name) VALUES ($1, $2)")
        .bind(design_id)
        .bind("bom-notes-test")
        .execute(&pool)
        .await?;

    let mfr = tokito::store::manufacturers::create(
        &pool,
        tokito::models::CreateManufacturer {
            name: "Test Mfr".into(),
            slug: None,
        },
    )
    .await?;
    let part = tokito::store::parts::create(
        &pool,
        tokito::models::CreatePart {
            mpn: "TEST-MPN-001".into(),
            manufacturer_id: mfr.id,
            description: None,
            package_name: None,
            attributes: None,
        },
    )
    .await?;

    let lines = tokito::store::bom::append_lines(
        &pool,
        design_id,
        &[tokito::models::BomLineInput {
            part_id: part.id,
            quantity: 1.0,
            sort_order: 0,
            notes: Some("initial".into()),
        }],
    )
    .await?;

    let patched = tokito::store::bom::patch_line(
        &pool,
        lines[0].id,
        None,
        Some("rework: add 10k pull-up"),
    )
    .await?;
    assert_eq!(
        patched.notes.as_deref(),
        Some("rework: add 10k pull-up")
    );
    Ok(())
}
