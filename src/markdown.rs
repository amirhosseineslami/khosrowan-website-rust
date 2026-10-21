//! Markdown rendering and HTML sanitisation.
//!
//! Content is stored as Markdown and rendered to sanitised HTML. The sanitiser
//! is the last line of defence: even if an author is trusted, a pasted
//! `<script>` should never reach a reader's browser.

use comrak::options::Plugins;
use comrak::Options;
use once_cell::sync::Lazy;
use regex::Regex;

/// Build the renderer options used for all post content.
fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.superscript = true;
    options.extension.description_lists = true;
    options.extension.front_matter_delimiter = Some("---".to_string());
    options.extension.shortcodes = true;
    // Raw HTML is allowed through the parser but stripped by the sanitiser.
    options.render.r#unsafe = true;
    options.render.hardbreaks = false;
    options.render.github_pre_lang = true;
    options.render.full_info_string = true;
    options.render.tasklist_classes = true;
    options
}

/// Render Markdown to sanitised HTML.
pub fn to_html(markdown: &str) -> String {
    let plugins = Plugins::default();
    let rendered = comrak::markdown_to_html_with_plugins(markdown, &options(), &plugins);
    sanitize(&rendered)
}

/// Render Markdown to HTML *without* sanitising. Only used for trusted
/// admin-authored content that is already under our control.
pub fn to_html_unsafe(markdown: &str) -> String {
    let plugins = Plugins::default();
    comrak::markdown_to_html_with_plugins(markdown, &options(), &plugins)
}

/// Sanitise a fragment of HTML using an allow-list.
pub fn sanitize(html: &str) -> String {
    static CLEANER: Lazy<ammonia::Builder<'static>> = Lazy::new(|| {
        let mut builder = ammonia::Builder::new();
        builder.generic_attributes(
            [
                "class",
                "id",
                "title",
                "dir",
                "lang",
                "role",
                "aria-label",
                "aria-hidden",
                "target",
                "colspan",
                "rowspan",
                "start",
                "checked",
                "disabled",
                "align",
            ]
            .into_iter()
            .collect(),
        );

        builder.add_tag_attributes("a", ["href", "name", "target"]);
        builder.add_tag_attributes("img", ["src", "alt", "width", "height", "loading"]);
        builder.add_tag_attributes("input", ["type", "checked", "disabled"]);

        builder.link_rel(Some("noopener noreferrer"));
        builder.url_relative(ammonia::UrlRelative::PassThrough);
        builder.url_schemes(
            ["http", "https", "mailto", "tel", "ftp", "irc", "magnet"]
                .into_iter()
                .collect(),
        );
        builder
    });

    CLEANER.clean(html).to_string()
}

/// Strip every tag and collapse whitespace — used for excerpts and meta
/// descriptions.
pub fn to_plain_text(markdown: &str) -> String {
    let html = comrak::markdown_to_html(markdown, &options());
    let stripped = strip_tags(&html);
    decode_entities(&stripped)
}

/// Remove all HTML tags from a string.
pub fn strip_tags(html: &str) -> String {
    static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").expect("valid regex"));
    TAG.replace_all(html, " ").into_owned()
}

/// Decode the most common HTML entities.
pub fn decode_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// Estimate the reading time in minutes (200 words per minute, minimum 1).
pub fn reading_time(markdown: &str) -> i64 {
    let text = to_plain_text(markdown);
    let words = text.split_whitespace().count();
    let minutes = (words as f64 / 200.0).ceil() as i64;
    minutes.max(1)
}

/// Count the words in a document.
pub fn word_count(markdown: &str) -> usize {
    to_plain_text(markdown).split_whitespace().count()
}

/// Build a table of contents from the headings of a Markdown document.
pub fn table_of_contents(markdown: &str) -> Vec<TocEntry> {
    static HEADING: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)^(#{2,4})\s+(.+?)\s*#*$").expect("valid regex"));

    HEADING
        .captures_iter(markdown)
        .map(|caps| {
            let level = caps.get(1).map(|m| m.as_str().len()).unwrap_or(2) as u32;
            let text = caps
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let anchor = anchor_for(&text);
            TocEntry {
                level,
                text,
                anchor,
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: u32,
    pub text: String,
    pub anchor: String,
}

/// GitHub-compatible heading anchor.
pub fn anchor_for(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

/// Extract the first image URL from a Markdown document.
pub fn first_image(markdown: &str) -> Option<String> {
    static IMG: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"!\[[^\]]*\]\(([^)\s]+)").expect("valid regex"));
    IMG.captures(markdown)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Escape a string for safe inclusion in HTML text or attributes.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_tags() {
        let html = to_html("Hello <script>alert('x')</script> world");
        assert!(!html.contains("<script"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn keeps_safe_markup() {
        let html = to_html("**bold** and [link](https://example.com)");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("href=\"https://example.com\""));
    }

    #[test]
    fn drops_javascript_urls() {
        let html = to_html("[click](javascript:alert(1))");
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn builds_table_of_contents() {
        let toc = table_of_contents("## First heading\n\n### Nested one\n");
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].anchor, "first-heading");
        assert_eq!(toc[1].level, 3);
    }

    #[test]
    fn reading_time_is_at_least_one_minute() {
        assert_eq!(reading_time("short"), 1);
    }
}
