use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};

pub(super) fn matches(headers: &HeaderMap, version: &str) -> bool {
    headers
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|tag| {
                let tag = tag.trim();
                tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == version
            })
        })
}

pub(super) fn versioned(mut response: Response, version: &str) -> Response {
    if response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED {
        response.headers_mut().insert(
            "etag",
            HeaderValue::from_str(version).expect("internal version hash"),
        );
    }
    response
}

pub(super) async fn private_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let cache = if response.headers().contains_key("etag") {
        "private, no-cache"
    } else {
        "private, no-store"
    };
    let headers = response.headers_mut();
    headers.insert("cache-control", HeaderValue::from_static(cache));
    headers.append("vary", HeaderValue::from_static("Authorization"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conditional_get_accepts_weak_tags_but_not_substrings() {
        let mut h = HeaderMap::new();
        h.insert(
            "if-none-match",
            HeaderValue::from_static("\"other\", W/\"version\""),
        );
        assert!(matches(&h, "\"version\""));
        assert!(!matches(&h, "\"vers\""));
    }
}
