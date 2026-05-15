use axum::Json;

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_json_ok() {
        let Json(v) = health().await;
        assert_eq!(v["status"], "ok");
    }
}
