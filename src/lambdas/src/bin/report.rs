use hhm_contracts::{valid_kind, PRODUCT};
use hhm_mash_lambdas::{html_response, validate_get};
use lambda_http::{http::StatusCode, run, service_fn, tracing, Body, Error, Request, Response};
use maud::{html, DOCTYPE};

const EVENT_KIND: &str = "occupancy.report";

async fn function_handler(request: Request) -> Result<Response<Body>, Error> {
    let bounds = match validate_get(&request, 24, 250) {
        Ok(bounds) => bounds,
        Err(response) => return Ok(response),
    };

    if !valid_kind(EVENT_KIND) {
        return html_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "<!doctype html><title>contract error</title>".to_owned(),
        );
    }

    let document = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Hacker House occupancy report" }
            }
            body {
                main {
                    h1 { "Hacker House occupancy report" }
                    p { "Product contract: " code { (PRODUCT) } }
                    p { "Event kind: " code { (EVENT_KIND) } }
                    p { "Bounded report rows requested: " strong { (bounds.limit) } }
                    p {
                        "This isolated Lambda entrypoint is ready for the shared-core "
                        "query adapter; it does not initialize an ORM connection during cold start."
                    }
                }
            }
        }
    };

    html_response(StatusCode::OK, document.into_string())
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
    async fn renders_the_shared_product_contract() {
        let request = Request::builder()
            .method(Method::GET)
            .body(Body::Empty)
            .expect("valid request");

        let response = function_handler(request).await.expect("handler succeeds");
        assert_eq!(response.status(), StatusCode::OK);

        let Body::Text(body) = response.body() else {
            panic!("expected text response");
        };
        assert!(body.contains(PRODUCT));
        assert!(body.contains(EVENT_KIND));
    }
}
