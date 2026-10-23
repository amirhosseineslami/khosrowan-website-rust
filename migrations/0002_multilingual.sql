-- Multilingual content support.
--
-- The base tables (posts, pages, projects) keep the canonical/default content
-- (locale = 'fa'). Translations for other locales live in dedicated
-- translation tables. This is additive: it does not rebuild existing tables
-- or touch their unique slug constraints.

CREATE TABLE IF NOT EXISTS post_translations (
    post_id        INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    locale         TEXT NOT NULL DEFAULT 'en',
    title          TEXT NOT NULL,
    excerpt        TEXT NOT NULL DEFAULT '',
    content_md     TEXT NOT NULL DEFAULT '',
    content_html    TEXT NOT NULL DEFAULT '',
    meta_title      TEXT,
    meta_description TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (post_id, locale)
);

CREATE TABLE IF NOT EXISTS page_translations (
    page_id        INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    locale         TEXT NOT NULL DEFAULT 'en',
    title          TEXT NOT NULL,
    content_md     TEXT NOT NULL DEFAULT '',
    content_html    TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (page_id, locale)
);

CREATE TABLE IF NOT EXISTS project_translations (
    project_id     INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    locale         TEXT NOT NULL DEFAULT 'en',
    title          TEXT NOT NULL,
    summary        TEXT NOT NULL DEFAULT '',
    description_md TEXT NOT NULL DEFAULT '',
    description_html TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (project_id, locale)
);
