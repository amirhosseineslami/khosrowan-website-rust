//! Small helpers shared across the application.

use axum::http::HeaderMap;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::net::IpAddr;
use std::str::FromStr;

/// Convert a title into a URL-safe slug.
pub fn slugify(input: &str) -> String {
    let out = slug::slugify(input);
    if out.is_empty() {
        "untitled".to_string()
    } else {
        out.to_string()
    }
}

/// Anything that can be treated as an optional timestamp.
pub trait DateLike {
    fn as_datetime(&self) -> Option<DateTime<Utc>>;
}

impl DateLike for DateTime<Utc> {
    fn as_datetime(&self) -> Option<DateTime<Utc>> {
        Some(*self)
    }
}

impl DateLike for Option<DateTime<Utc>> {
    fn as_datetime(&self) -> Option<DateTime<Utc>> {
        *self
    }
}

/// Format a date as `12 March 2024`.
pub fn format_date(date: impl DateLike) -> String {
    match date.as_datetime() {
        Some(d) => d.format("%d %B %Y").to_string(),
        None => "—".to_string(),
    }
}

/// Format a date as `2024-03-12`.
pub fn format_date_iso(date: impl DateLike) -> String {
    match date.as_datetime() {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => String::new(),
    }
}

/// Format a date as an ISO-8601 timestamp for `<time datetime="...">`.
pub fn format_datetime(date: impl DateLike) -> String {
    match date.as_datetime() {
        Some(d) => d.to_rfc3339(),
        None => String::new(),
    }
}

/// Format a date as a short, human friendly string such as `3 days ago`.
pub fn time_ago(date: impl DateLike) -> String {
    let Some(date) = date.as_datetime() else {
        return "never".to_string();
    };
    let now = Utc::now();
    let delta = now.signed_duration_since(date);

    if delta.num_seconds() < 0 {
        return "in the future".to_string();
    }
    if delta.num_seconds() < 60 {
        return "just now".to_string();
    }
    if delta.num_minutes() < 60 {
        let n = delta.num_minutes();
        return format!("{n} minute{} ago", if n == 1 { "" } else { "s" });
    }
    if delta.num_hours() < 24 {
        let n = delta.num_hours();
        return format!("{n} hour{} ago", if n == 1 { "" } else { "s" });
    }
    if delta.num_days() < 30 {
        let n = delta.num_days();
        return format!("{n} day{} ago", if n == 1 { "" } else { "s" });
    }
    if delta.num_days() < 365 {
        let n = delta.num_days() / 30;
        return format!("{n} month{} ago", if n == 1 { "" } else { "s" });
    }
    let n = delta.num_days() / 365;
    format!("{n} year{} ago", if n == 1 { "" } else { "s" })
}

/// Extract the client IP address from proxy headers.
pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    static FORWARDED: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)for=([^;,\s]+)").expect("valid regex"));
    static IPV4: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("valid regex"));

    for header in ["cf-connecting-ip", "x-real-ip", "x-forwarded-for"] {
        if let Some(value) = headers.get(header).and_then(|v| v.to_str().ok()) {
            let candidate = value.split(',').next().unwrap_or(value).trim();
            if IpAddr::from_str(candidate).is_ok() {
                return Some(candidate.to_string());
            }
            if let Some(found) = IPV4.find(candidate) {
                return Some(found.as_str().to_string());
            }
        }
    }

    if let Some(value) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        if let Some(caps) = FORWARDED.captures(value) {
            let candidate = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if IpAddr::from_str(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// Extract the user agent string.
pub fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(512).collect())
}

/// Hash an IP address for privacy-preserving analytics.
pub fn hash_ip(ip: &str, salt: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(ip.as_bytes());
    hex::encode(hasher.finalize())
}

/// Pagination parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

impl Pagination {
    pub fn page(&self) -> usize {
        self.page.unwrap_or(1).max(1)
    }
    pub fn per_page(&self) -> usize {
        self.per_page.unwrap_or(9).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        ((self.page() - 1) as i64) * self.per_page() as i64
    }
    pub fn total_pages(&self, total: i64) -> usize {
        let per_page = self.per_page() as i64;
        if per_page <= 0 {
            return 1;
        }
        ((total + per_page - 1) / per_page).max(1) as usize
    }
}

/// A page of results plus the information needed to render a pager.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: usize,
    pub per_page: usize,
    pub total: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, page: usize, per_page: usize, total: i64) -> Self {
        Self {
            items,
            page,
            per_page,
            total,
        }
    }
    pub fn total_pages(&self) -> usize {
        if self.per_page == 0 {
            return 1;
        }
        ((self.total + self.per_page as i64 - 1) / self.per_page as i64).max(1) as usize
    }
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages()
    }
    pub fn prev(&self) -> usize {
        self.page.saturating_sub(1).max(1)
    }
    pub fn next(&self) -> usize {
        (self.page + 1).min(self.total_pages())
    }
    /// The page numbers to show, with `0` marking an ellipsis.
    pub fn window(&self) -> Vec<i64> {
        let total = self.total_pages();
        let current = self.page;
        let mut pages: Vec<i64> = Vec::new();

        if total <= 7 {
            pages.extend((1..=total).map(|p| p as i64));
            return pages;
        }

        pages.push(1);
        if current > 3 {
            pages.push(0);
        }
        let start = current.saturating_sub(1).max(2);
        let end = (current + 1).min(total - 1);
        for p in start..=end {
            pages.push(p as i64);
        }
        if current < total - 2 {
            pages.push(0);
        }
        pages.push(total as i64);
        pages
    }
}

/// Parse a `NaiveDate` from a `YYYY` or `YYYY-MM` string.
pub fn parse_partial_date(input: &str) -> Option<NaiveDate> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{input}-01"), "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{input}-01-01"), "%Y-%m-%d") {
        return Some(d);
    }
    None
}

/// Truncate a string on a word boundary.
pub fn truncate(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out: String = input.chars().take(max_chars).collect();
    if let Some(pos) = out.rfind(' ') {
        out.truncate(pos);
    }
    out.push('…');
    out
}

/// Format a number with thousands separators.
pub fn format_number(value: i64) -> String {
    let negative = value < 0;
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if negative {
        format!("-{out}")
    } else {
        out
    }
}

/// Build an initials badge from a name.
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().filter(|p| !p.is_empty()).collect();
    let mut out = String::new();
    for part in parts.iter().take(2) {
        if let Some(ch) = part.chars().next() {
            out.push(ch.to_ascii_uppercase());
        }
    }
    if out.is_empty() {
        "?".to_string()
    } else {
        out
    }
}

/// Produce a deterministic colour for a string (used for avatars).
pub fn color_for(seed: &str) -> String {
    let mut hash: u32 = 2166136261;
    for byte in seed.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    let hue = hash % 360;
    format!("hsl({hue} 70% 55%)")
}

/// Validate an email address (deliberately permissive but practical).
pub fn is_valid_email(email: &str) -> bool {
    static EMAIL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}$").expect("valid regex")
    });
    EMAIL.is_match(email.trim()) && email.len() <= 254
}

/// Ensure a string only contains safe characters for a URL path segment.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('-').to_string();
    if cleaned.is_empty() {
        "file".to_string()
    } else {
        cleaned.chars().take(120).collect()
    }
}

/// Year extracted from a date, used for archive listings.
pub fn year_of(date: DateTime<Utc>) -> i32 {
    date.year()
}

/// Parse a comma separated list into a vector of trimmed strings.
pub fn split_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Render a simple markdown-ish string to plain text for meta descriptions.
pub fn meta_description(excerpt: &str, fallback: &str) -> String {
    let text = crate::markdown::to_plain_text(excerpt);
    let text = text.trim();
    if text.is_empty() {
        truncate(fallback, 160)
    } else {
        truncate(text, 160)
    }
}
