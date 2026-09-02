use hhm_contracts::{valid_kind, PRODUCT};
use hhm_mash_lambdas::{json_response, validate_get};
use lambda_http::{http::StatusCode, run, service_fn, tracing, Body, Error, Request, Response};
use serde::Serialize;

const EVENT_KIND: &str = "roster.export";

#[derive(Debug, Serialize)]
struct ExportEnvelope {
    schema: &'static str,
    product: &'static str,
    event_kind: &'static str,
    requested_rows: usize,
    records: Vec<serde_json::Value>,
}

async fn function_handler(request: Request) -> Result<Response<Body>, Error> {
    let bounds = match validate_get(&request, 100, 1_000) {
        Ok(bounds) => bounds,
        Err(response) => return Ok(response),
    };

    if !valid_kind(EVENT_KIND) {
        return json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &serde_json::json!({"error": {"code": "invalid_shared_contract"}}),
        );
    }

    json_response(
        StatusCode::OK,
        &ExportEnvelope {
            schema: "hhm.roster-export.v1",
            product: PRODUCT,
            event_kind: EVENT_KIND,
            requested_rows: bounds.limit,
            records: Vec::new(),
        },
    )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    run(service_fn(function_handler)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::http::Method;

    #[tokio::test]
    async fn returns_an_empty_non_fabricated_export() {
        let request = Request::builder()
            .method(Method::GET)
            .body(Body::Empty)
            .expect("valid request");

        let response = function_handler(request).await.expect("handler succeeds");
        assert_eq!(response.status(), StatusCode::OK);

        let Body::Text(body) = response.body() else {
            panic!("expected text response");
        };
        assert!(body.contains("\"records\":[]"));
        assert!(body.contains(PRODUCT));
    }
}
