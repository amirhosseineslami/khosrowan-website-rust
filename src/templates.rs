//! Template rendering helpers.

use askama::Template;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// Wraps an Askama template so it can be returned directly from a handler.
pub struct Html<T>(pub T);

impl<T> Html<T> {
    pub fn new(template: T) -> Self {
        Self(template)
    }
}

impl<T: Template> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(body) => {
                let mut response = body.into_response();
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static("text/html; charset=utf-8"),
                );
                response
            }
            Err(err) => {
                tracing::error!(error = %err, template = %std::any::type_name::<T>(), "template rendering failed");
                let body = error_page(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "The page could not be rendered. The error has been logged.",
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    body,
                )
                    .into_response()
            }
        }
    }
}

/// Render a template into a string (used for feeds and email bodies).
pub fn render_to_string<T: Template>(template: &T) -> Result<String, askama::Error> {
    template.render()
}

/// A self-contained HTML error page.
pub fn error_page(status: StatusCode, message: &str) -> String {
    let code = status.as_u16();
    let title = match status {
        StatusCode::NOT_FOUND => "Page not found",
        StatusCode::FORBIDDEN => "Access denied",
        StatusCode::UNAUTHORIZED => "Authentication required",
        StatusCode::BAD_REQUEST => "Bad request",
        StatusCode::TOO_MANY_REQUESTS => "Too many requests",
        _ => "Something went wrong",
    };

    format!(
        r#"<!doctype html>
<html lang="en" data-theme="dark">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{code} · {title}</title>
<style>
  :root {{ color-scheme: dark; }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(1000px 600px at 50% -10%, #1b2130, #0b0d12 60%);
    color: #e6e9ef; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    padding: 24px;
  }}
  .card {{
    max-width: 560px; text-align: center; background: rgba(18,21,29,.8);
    border: 1px solid #1f2530; border-radius: 20px; padding: 48px 40px;
    backdrop-filter: blur(8px);
  }}
  .code {{ font-size: 64px; font-weight: 800; letter-spacing: -2px;
    background: linear-gradient(135deg, #6366f1, #22d3ee); -webkit-background-clip: text;
    background-clip: text; color: transparent; }}
  h1 {{ margin: 8px 0 12px; font-size: 24px; }}
  p {{ margin: 0 0 28px; color: #9aa4b2; line-height: 1.7; }}
  a.button {{ display: inline-block; padding: 12px 22px; border-radius: 12px; text-decoration: none;
    background: linear-gradient(135deg, #6366f1, #22d3ee); color: #0b0d12; font-weight: 650; }}
</style>
</head>
<body>
  <main class="card">
    <div class="code">{code}</div>
    <h1>{title}</h1>
    <p>{message}</p>
    <a class="button" href="/">Back to home</a>
  </main>
</body>
</html>"#,
        code = code,
        title = crate::markdown::escape(title),
        message = crate::markdown::escape(message),
    )
}
