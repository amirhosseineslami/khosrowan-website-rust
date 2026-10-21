//! Security response headers and a strict Content-Security-Policy.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Apply the headers directly to a response (used where a layer is awkward).
pub fn apply(response: &mut Response) {
    let headers = response.headers_mut();

    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("SAMEORIGIN"),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "permissions-policy",
        axum::http::HeaderValue::from_static(
            "geolocation=(), microphone=(), camera=(), payment=(), usb=()",
        ),
    );
    headers.insert(
        "cross-origin-opener-policy",
        axum::http::HeaderValue::from_static("same-origin"),
    );

    // A conservative CSP: the site ships no third-party JavaScript by default.
    let csp = concat!(
        "default-src 'self'; ",
        "img-src 'self' data: blob: https:; ",
        "style-src 'self' 'unsafe-inline'; ",
        "script-src 'self' 'unsafe-inline'; ",
        "font-src 'self' data:; ",
        "connect-src 'self'; ",
        "frame-ancestors 'self'; ",
        "base-uri 'self'; ",
        "form-action 'self'; ",
        "object-src 'none'"
    );
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(csp),
    );

    headers.insert(
        axum::http::header::STRICT_TRANSPORT_SECURITY,
        axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
}

/// Middleware wrapper around [`apply`].
pub async fn headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    apply(&mut response);
    response
}
