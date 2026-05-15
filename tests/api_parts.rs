//! Manufacturers and parts catalog API (embedded Postgres via pg-embed).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn json_body(res: axum::response::Response) -> serde_json::Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn create_manufacturer_and_part() {
    if !tokito::test_support::database_integration_tests_enabled() {
        return;
    }
    tokito::test_support::with_timeout(async {
        let (app, bearer) = tokito::test_support::test_router().await?;

        let mfr = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/manufacturers")
                    .header("authorization", bearer.as_str())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Test Mfr"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(mfr.status(), StatusCode::OK);
        let mfr_id = json_body(mfr).await["id"].as_str().unwrap().to_string();

        let part = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/parts")
                    .header("authorization", bearer.as_str())
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"manufacturer_id":"{mfr_id}","mpn":"LM358DT","description":"dual op-amp"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(part.status(), StatusCode::OK);
        assert_eq!(json_body(part).await["mpn"], "LM358DT");
        Ok(())
    })
    .await
    .expect("create_manufacturer_and_part");
}
