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
pub enum ValidationError {
    MethodNotAllowed,
    PayloadTooLarge,
    LimitNotInteger,
    LimitOutsideRange,
}

impl ValidationError {
    pub fn into_response(self) -> Response<Body> {
        match self {
            Self::MethodNotAllowed => problem(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "this Lambda accepts GET requests only",
            ),
            Self::PayloadTooLarge => problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "request body exceeds the 64 KiB Lambda adapter limit",
            ),
            Self::LimitNotInteger => problem(
                StatusCode::BAD_REQUEST,
                "invalid_limit",
                "limit must be an unsigned integer",
            ),
            Self::LimitOutsideRange => problem(
                StatusCode::BAD_REQUEST,
                "invalid_limit",
                "limit is outside the accepted range",
            ),
        }
    }
}

pub fn validate_get(
    request: &Request,
    default_limit: usize,
    max_limit: usize,
) -> Result<RequestBounds, ValidationError> {
    if request.method() != Method::GET {
        return Err(ValidationError::MethodNotAllowed);
    }

    let body_len = match request.body() {
        Body::Empty => 0,
        Body::Text(value) => value.len(),
        Body::Binary(value) => value.len(),
        _ => MAX_REQUEST_BYTES + 1,
    };
    if body_len > MAX_REQUEST_BYTES {
        return Err(ValidationError::PayloadTooLarge);
    }

    let raw_limit = request
        .query_string_parameters_ref()
        .and_then(|parameters| parameters.first("limit"));

    parse_limit(raw_limit, default_limit, max_limit).map(|limit| RequestBounds { limit })
}

fn parse_limit(
    raw: Option<&str>,
    default_limit: usize,
    max_limit: usize,
) -> Result<usize, ValidationError> {
    if max_limit == 0 || default_limit == 0 || default_limit > max_limit {
        return Err(ValidationError::LimitOutsideRange);
    }

    let Some(raw) = raw else {
        return Ok(default_limit);
    };

    let parsed = raw
        .parse::<usize>()
        .map_err(|_| ValidationError::LimitNotInteger)?;

    if parsed == 0 || parsed > max_limit {
        return Err(ValidationError::LimitOutsideRange);
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
            Err(ValidationError::LimitNotInteger)
        );
        assert_eq!(
            parse_limit(Some("0"), 25, 100),
            Err(ValidationError::LimitOutsideRange)
        );
        assert_eq!(
            parse_limit(Some("101"), 25, 100),
            Err(ValidationError::LimitOutsideRange)
        );
    }

    #[test]
    fn rejects_non_get_requests() {
        let mut request = Request::new(Body::Empty);
        *request.method_mut() = Method::POST;

        assert_eq!(
            validate_get(&request, 25, 100),
            Err(ValidationError::MethodNotAllowed)
        );
    }
}
