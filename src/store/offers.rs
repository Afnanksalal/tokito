use crate::error::{AppError, AppResult};
use crate::models::{PartOffer, UpsertOffer};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_for_part(pool: &PgPool, part_id: Uuid) -> AppResult<Vec<PartOffer>> {
    sqlx::query_as::<_, PartOffer>(
        r#"
        SELECT id, part_id, distributor, sku, product_url, currency, unit_price_cents, stock_qty, fetched_at
        FROM part_offers
        WHERE part_id = $1
        ORDER BY distributor ASC, sku ASC
        "#,
    )
    .bind(part_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn upsert(pool: &PgPool, part_id: Uuid, o: UpsertOffer) -> AppResult<PartOffer> {
    sqlx::query_as::<_, PartOffer>(
        r#"
        INSERT INTO part_offers (part_id, distributor, sku, product_url, currency, unit_price_cents, stock_qty, fetched_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, now())
        ON CONFLICT (part_id, distributor, sku) DO UPDATE SET
          product_url = EXCLUDED.product_url,
          currency = EXCLUDED.currency,
          unit_price_cents = EXCLUDED.unit_price_cents,
          stock_qty = EXCLUDED.stock_qty,
          fetched_at = now()
        RETURNING id, part_id, distributor, sku, product_url, currency, unit_price_cents, stock_qty, fetched_at
        "#,
    )
    .bind(part_id)
    .bind(&o.distributor)
    .bind(&o.sku)
    .bind(&o.product_url)
    .bind(&o.currency)
    .bind(o.unit_price_cents)
    .bind(o.stock_qty)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23503") => {
            AppError::BadRequest("unknown part_id".into())
        }
        _ => e.into(),
    })
}
