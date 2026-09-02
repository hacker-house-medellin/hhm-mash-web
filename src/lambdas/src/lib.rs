use lambda_http::{
    http::{Method, StatusCode},
    Body, Error, Request, RequestExt, Response,
};
use serde::Serialize;

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestBounds {
    pub limit: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LimitError {
    NotAnInteger,
    OutsideRange,
}

pub fn validate_get(
    request: &Request,
    default_limit: usize,
    max_limit: usize,
) -> Result<RequestBounds, Response<Body>> {
    if request.method() != Method::GET {
        return Err(problem(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "this Lambda accepts GET requests only",
        ));
    }

    let body_len = match request.body() {
        Body::Empty => 0,
        Body::Text(value) => value.len(),
        Body::Binary(value) => value.len(),
        _ => MAX_REQUEST_BYTES + 1,
    };
    if body_len > MAX_REQUEST_BYTES {
        return Err(problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "request body exceeds the 64 KiB Lambda adapter limit",
        ));
    }

    let raw_limit = request
        .query_string_parameters_ref()
        .and_then(|parameters| parameters.first("limit"));

    match parse_limit(raw_limit, default_limit, max_limit) {
        Ok(limit) => Ok(RequestBounds { limit }),
        Err(LimitError::NotAnInteger) => Err(problem(
            StatusCode::BAD_REQUEST,
            "invalid_limit",
            "limit must be an unsigned integer",
        )),
        Err(LimitError::OutsideRange) => Err(problem(
            StatusCode::BAD_REQUEST,
            "invalid_limit",
            "limit is outside the accepted range",
        )),
    }
}

fn parse_limit(
    raw: Option<&str>,
    default_limit: usize,
    max_limit: usize,
) -> Result<usize, LimitError> {
    if max_limit == 0 || default_limit == 0 || default_limit > max_limit {
        return Err(LimitError::OutsideRange);
    }

    let Some(raw) = raw else {
        return Ok(default_limit);
    };

    let parsed = raw.parse::<usize>().map_err(|_| LimitError::NotAnInteger)?;

    if parsed == 0 || parsed > max_limit {
        return Err(LimitError::OutsideRange);
    }

    Ok(parsed)
}

pub fn html_response(status: StatusCode, body: String) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .header("cache-control", "no-store")
        .header(
            "content-security-policy",
            "default-src 'none'; style-src 'unsafe-inline'",
        )
        .header("referrer-policy", "no-referrer")
        .header("x-content-type-options", "nosniff")
        .body(Body::Text(body))?)
}

pub fn json_response<T: Serialize>(
    status: StatusCode,
    payload: &T,
) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(payload)?;
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("cache-control", "no-store")
        .header("content-security-policy", "default-src 'none'")
        .header("referrer-policy", "no-referrer")
        .header("x-content-type-options", "nosniff")
        .body(Body::Text(body))?)
}

fn problem(status: StatusCode, code: &'static str, message: &'static str) -> Response<Body> {
    #[derive(Serialize)]
    struct Problem<'a> {
        error: ErrorBody<'a>,
    }

    #[derive(Serialize)]
    struct ErrorBody<'a> {
        code: &'a str,
        message: &'a str,
    }

    let payload = serde_json::to_string(&Problem {
        error: ErrorBody { code, message },
    })
    .unwrap_or_else(|_| {
        "{\"error\":{\"code\":\"response_encoding_failed\",\"message\":\"response encoding failed\"}}"
            .to_owned()
    });

    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("cache-control", "no-store")
        .header("content-security-policy", "default-src 'none'")
        .header("referrer-policy", "no-referrer")
        .header("x-content-type-options", "nosniff")
        .body(Body::Text(payload))
        .unwrap_or_else(|_| Response::new(Body::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_default_limit() {
        assert_eq!(parse_limit(None, 25, 100), Ok(25));
    }

    #[test]
    fn rejects_invalid_limits() {
        assert_eq!(
            parse_limit(Some("not-a-number"), 25, 100),
            Err(LimitError::NotAnInteger)
        );
        assert_eq!(
            parse_limit(Some("0"), 25, 100),
            Err(LimitError::OutsideRange)
        );
        assert_eq!(
            parse_limit(Some("101"), 25, 100),
            Err(LimitError::OutsideRange)
        );
    }

    #[test]
    fn rejects_non_get_requests() {
        let request = Request::builder()
            .method(Method::POST)
            .body(Body::Empty)
            .expect("valid request");

        let response = validate_get(&request, 25, 100).expect_err("POST must be rejected");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
