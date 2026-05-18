use crate::error::AppResult;
use crate::models::{BomLine, ReplaceBom};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

pub async fn list_for_design(pool: &PgPool, design_id: Uuid) -> AppResult<Vec<BomLine>> {
    let rows = sqlx::query_as::<_, BomLine>(
        r#"
        SELECT id, design_id, part_id, quantity, sort_order, notes, updated_at
        FROM bom_lines
        WHERE design_id = $1
        ORDER BY sort_order ASC, updated_at ASC
        "#,
    )
    .bind(design_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Replaces all BOM lines for a design atomically and validates part IDs.
pub async fn replace_validated(
    pool: &PgPool,
    design_id: Uuid,
    body: ReplaceBom,
) -> AppResult<Vec<BomLine>> {
    let uniq: Vec<Uuid> = body
        .lines
        .iter()
        .map(|l| l.part_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let mut tx = pool.begin().await?;
    if !uniq.is_empty() {
        let n: (i64,) = sqlx::query_as(r#"SELECT COUNT(*)::bigint FROM parts WHERE id = ANY($1)"#)
            .bind(&uniq)
            .fetch_one(&mut *tx)
            .await?;
        if n.0 != uniq.len() as i64 {
            return Err(crate::error::AppError::BadRequest(
                "one or more part_id values do not exist".into(),
            ));
        }
    }
    sqlx::query(r#"DELETE FROM bom_lines WHERE design_id = $1"#)
        .bind(design_id)
        .execute(&mut *tx)
        .await?;
    let mut out = Vec::with_capacity(body.lines.len());
    for (i, line) in body.lines.iter().enumerate() {
        let sort = if line.sort_order != 0 {
            line.sort_order
        } else {
            i as i32
        };
        let row = sqlx::query_as::<_, BomLine>(
            r#"
            INSERT INTO bom_lines (id, design_id, part_id, quantity, sort_order, notes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, design_id, part_id, quantity, sort_order, notes, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(design_id)
        .bind(line.part_id)
        .bind(line.quantity)
        .bind(sort)
        .bind(&line.notes)
        .fetch_one(&mut *tx)
        .await?;
        out.push(row);
    }
    tx.commit().await?;
    Ok(out)
}

/// Append BOM lines without deleting existing rows (for agents / incremental edits).
pub async fn append_lines(
    pool: &PgPool,
    design_id: Uuid,
    lines: &[crate::models::BomLineInput],
) -> AppResult<Vec<BomLine>> {
    let uniq: Vec<Uuid> = lines
        .iter()
        .map(|l| l.part_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let mut tx = pool.begin().await?;
    let max_sort: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
        r#"SELECT MAX(sort_order) FROM bom_lines WHERE design_id = $1"#,
    )
    .bind(design_id)
    .fetch_one(&mut *tx)
    .await?;
    let base = max_sort.unwrap_or(-1) + 1;
    if !uniq.is_empty() {
        let n: (i64,) = sqlx::query_as(r#"SELECT COUNT(*)::bigint FROM parts WHERE id = ANY($1)"#)
            .bind(&uniq)
            .fetch_one(&mut *tx)
            .await?;
        if n.0 != uniq.len() as i64 {
            return Err(crate::error::AppError::BadRequest(
                "one or more part_id values do not exist".into(),
            ));
        }
    }
    let mut out = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let sort = if line.sort_order != 0 {
            line.sort_order
        } else {
            base + i as i32
        };
        let row = sqlx::query_as::<_, BomLine>(
            r#"
            INSERT INTO bom_lines (id, design_id, part_id, quantity, sort_order, notes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, design_id, part_id, quantity, sort_order, notes, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(design_id)
        .bind(line.part_id)
        .bind(line.quantity)
        .bind(sort)
        .bind(&line.notes)
        .fetch_one(&mut *tx)
        .await?;
        out.push(row);
    }
    tx.commit().await?;
    Ok(out)
}

fn csv_escape_cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub async fn csv_export(pool: &PgPool, design_id: Uuid) -> AppResult<String> {
    let rows: Vec<(String, f64, String)> = sqlx::query_as(
        r#"
        SELECT p.mpn, b.quantity, COALESCE(b.notes, '')
        FROM bom_lines b
        JOIN parts p ON p.id = b.part_id
        WHERE b.design_id = $1
        ORDER BY b.sort_order ASC, b.updated_at ASC
        "#,
    )
    .bind(design_id)
    .fetch_all(pool)
    .await?;
    let mut buf = String::from("mpn,quantity,notes\n");
    for (mpn, qty, notes) in rows {
        buf.push_str(&csv_escape_cell(&mpn));
        buf.push(',');
        buf.push_str(&qty.to_string());
        buf.push(',');
        buf.push_str(&csv_escape_cell(&notes));
        buf.push('\n');
    }
    Ok(buf)
}

pub async fn delete_line(pool: &PgPool, line_id: Uuid) -> AppResult<()> {
    let r = sqlx::query("DELETE FROM bom_lines WHERE id = $1")
        .bind(line_id)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(
            "bom line not found".into(),
        ));
    }
    Ok(())
}

pub async fn patch_line(
    pool: &PgPool,
    line_id: Uuid,
    quantity: Option<f64>,
    notes: Option<&str>,
) -> AppResult<BomLine> {
    let current: BomLine = sqlx::query_as(
        r#"
        SELECT id, design_id, part_id, quantity, sort_order, notes, updated_at
        FROM bom_lines WHERE id = $1
        "#,
    )
    .bind(line_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("bom line not found".into()))?;
    let qty = quantity.unwrap_or(current.quantity);
    if qty <= 0.0 {
        return Err(crate::error::AppError::BadRequest(
            "quantity must be > 0".into(),
        ));
    }
    let notes_val = notes.map(|s| s.to_string()).or(current.notes);
    sqlx::query_as::<_, BomLine>(
        r#"
        UPDATE bom_lines SET quantity = $2, notes = $3, updated_at = now()
        WHERE id = $1
        RETURNING id, design_id, part_id, quantity, sort_order, notes, updated_at
        "#,
    )
    .bind(line_id)
    .bind(qty)
    .bind(&notes_val)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}
