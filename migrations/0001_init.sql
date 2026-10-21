-- Personal site + CMS schema (SQLite).

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    username        TEXT    NOT NULL UNIQUE,
    email           TEXT    NOT NULL UNIQUE,
    password_hash   TEXT    NOT NULL,
    role            TEXT    NOT NULL DEFAULT 'editor',
    display_name    TEXT    NOT NULL DEFAULT '',
    avatar          TEXT,
    bio             TEXT,
    totp_secret    TEXT,
    totp_enabled    INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    failed_logins   INTEGER NOT NULL DEFAULT 0,
    locked_until    TEXT,
    last_login_at   TEXT,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS sessions (    id           TEXT    PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    csrf_token   TEXT    NOT NULL,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    expires_at   TEXT    NOT NULL,
    last_seen_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    ip           TEXT,
    user_agent   TEXT
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

CREATE TABLE IF NOT EXISTS password_resets (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      TEXT    NOT NULL UNIQUE,
    expires_at TEXT    NOT NULL,
    used_at    TEXT,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_password_resets_token ON password_resets(token);

CREATE TABLE IF NOT EXISTS categories (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    slug        TEXT NOT NULL UNIQUE,
    description TEXT,
    post_count  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS tags (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    slug        TEXT NOT NULL UNIQUE,
    description TEXT,
    color       TEXT,
    post_count  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS posts (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    slug             TEXT    NOT NULL UNIQUE,
    title            TEXT    NOT NULL,
    excerpt          TEXT    NOT NULL DEFAULT '',
    content_md       TEXT    NOT NULL DEFAULT '',
    content_html     TEXT    NOT NULL DEFAULT '',
    cover_image      TEXT,
    status           TEXT    NOT NULL DEFAULT 'draft',
    category_id      INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    author_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_featured      INTEGER NOT NULL DEFAULT 0,
    allow_comments   INTEGER NOT NULL DEFAULT 1,
    meta_title       TEXT,
    meta_description TEXT,
    canonical_url    TEXT,
    og_image         TEXT,
    view_count       INTEGER NOT NULL DEFAULT 0,
    reading_time     INTEGER NOT NULL DEFAULT 1,
    published_at     TEXT,
    scheduled_for    TEXT,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_posts_status ON posts(status);
CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(published_at);
CREATE INDEX IF NOT EXISTS idx_posts_category ON posts(category_id);

CREATE TABLE IF NOT EXISTS post_tags (
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

CREATE TABLE IF NOT EXISTS projects (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    slug             TEXT    NOT NULL UNIQUE,
    title            TEXT    NOT NULL,
    summary          TEXT    NOT NULL DEFAULT '',
    description_md   TEXT    NOT NULL DEFAULT '',
    description_html TEXT    NOT NULL DEFAULT '',
    cover_image      TEXT,
    gallery          TEXT,
    repo_url         TEXT,
    live_url         TEXT,
    status           TEXT    NOT NULL DEFAULT 'active',
    client           TEXT,
    is_featured      INTEGER NOT NULL DEFAULT 0,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    tech_stack       TEXT,
    meta_title       TEXT,
    meta_description TEXT,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

CREATE TABLE IF NOT EXISTS project_tags (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, tag_id)
);

CREATE TABLE IF NOT EXISTS pages (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    slug             TEXT    NOT NULL UNIQUE,
    title            TEXT    NOT NULL,
    content_md       TEXT    NOT NULL DEFAULT '',
    content_html     TEXT    NOT NULL DEFAULT '',
    status           TEXT    NOT NULL DEFAULT 'published',
    is_full_width    INTEGER NOT NULL DEFAULT 0,
    show_in_nav      INTEGER NOT NULL DEFAULT 0,
    nav_order        INTEGER NOT NULL DEFAULT 0,
    meta_title       TEXT,
    meta_description TEXT,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS comments (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id        INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    parent_id      INTEGER REFERENCES comments(id) ON DELETE CASCADE,
    author_name    TEXT    NOT NULL,
    author_email   TEXT,
    author_website TEXT,
    body           TEXT    NOT NULL,
    ip             TEXT,
    user_agent     TEXT,
    status         TEXT    NOT NULL DEFAULT 'pending',
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_comments_post ON comments(post_id);
CREATE INDEX IF NOT EXISTS idx_comments_status ON comments(status);

CREATE TABLE IF NOT EXISTS messages (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    email      TEXT    NOT NULL,
    subject    TEXT,
    body       TEXT    NOT NULL,
    ip         TEXT,
    user_agent TEXT,
    status     TEXT    NOT NULL DEFAULT 'new',
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS subscribers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    email        TEXT    NOT NULL UNIQUE,
    name         TEXT,
    status       TEXT    NOT NULL DEFAULT 'active',
    token        TEXT    NOT NULL UNIQUE,
    source       TEXT,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    confirmed_at TEXT
);

CREATE TABLE IF NOT EXISTS media (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    filename    TEXT    NOT NULL,
    stored_name TEXT    NOT NULL UNIQUE,
    path        TEXT    NOT NULL,
    mime        TEXT    NOT NULL,
    size        INTEGER NOT NULL DEFAULT 0,
    width       INTEGER,
    height      INTEGER,
    alt_text    TEXT,
    uploaded_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS skills (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    category   TEXT    NOT NULL DEFAULT 'other',
    level      INTEGER NOT NULL DEFAULT 50,
    years      REAL,
    icon       TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS experiences (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT    NOT NULL,
    company    TEXT    NOT NULL DEFAULT '',
    location   TEXT,
    start_date TEXT,
    end_date   TEXT,
    is_current INTEGER NOT NULL DEFAULT 0,
    description TEXT   NOT NULL DEFAULT '',
    kind       TEXT    NOT NULL DEFAULT 'work',
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS services (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    icon        TEXT,
    price_from  TEXT,
    features    TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS testimonials (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    role         TEXT,
    company      TEXT,
    body         TEXT    NOT NULL,
    avatar       TEXT,
    rating       INTEGER NOT NULL DEFAULT 5,
    is_featured  INTEGER NOT NULL DEFAULT 0,
    sort_order   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS awards (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL,
    issuer      TEXT,
    awarded_on  TEXT,
    description TEXT,
    link        TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS talks (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT    NOT NULL,
    event      TEXT,
    date       TEXT,
    location   TEXT,
    video_url  TEXT,
    slides_url TEXT,
    summary    TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS publications (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT    NOT NULL,
    publisher  TEXT,
    date       TEXT,
    url        TEXT,
    summary    TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS gallery (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT,
    image      TEXT    NOT NULL,
    caption    TEXT,
    category   TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS social_links (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    platform   TEXT    NOT NULL,
    url        TEXT    NOT NULL,
    label      TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT,
    value_type TEXT    NOT NULL DEFAULT 'text',
    group_name TEXT    NOT NULL DEFAULT 'general',
    label      TEXT,
    help_text  TEXT,
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS redirects (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    from_path   TEXT    NOT NULL UNIQUE,
    to_path     TEXT    NOT NULL,
    status_code INTEGER NOT NULL DEFAULT 301,
    hits        INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER REFERENCES users(id) ON DELETE SET NULL,
    username    TEXT,
    action      TEXT    NOT NULL,
    entity_type TEXT,
    entity_id   TEXT,
    details     TEXT,
    ip          TEXT,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);

CREATE TABLE IF NOT EXISTS page_views (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    path       TEXT    NOT NULL,
    referrer   TEXT,
    user_agent TEXT,
    country    TEXT,
    day        TEXT,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_pageviews_day ON page_views(day);
CREATE INDEX IF NOT EXISTS idx_pageviews_path ON page_views(path);

CREATE TABLE IF NOT EXISTS campaigns (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    subject         TEXT    NOT NULL,
    body_html       TEXT    NOT NULL,
    status          TEXT    NOT NULL DEFAULT 'draft',
    recipient_count INTEGER NOT NULL DEFAULT 0,
    sent_at         TEXT,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Full-text search index over published posts.
CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5 (
    title,
    excerpt,
    content_md,
    slug UNINDEXED,
    content='posts',
    content_rowid='id',
    tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS posts_fts_insert AFTER INSERT ON posts BEGIN
    INSERT INTO posts_fts(rowid, title, excerpt, content_md, slug)
    VALUES (new.id, new.title, new.excerpt, new.content_md, new.slug);
END;

CREATE TRIGGER IF NOT EXISTS posts_fts_delete AFTER DELETE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, excerpt, content_md, slug)
    VALUES ('delete', old.id, old.title, old.excerpt, old.content_md, old.slug);
END;

CREATE TRIGGER IF NOT EXISTS posts_fts_update AFTER UPDATE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, excerpt, content_md, slug)
    VALUES ('delete', old.id, old.title, old.excerpt, old.content_md, old.slug);
    INSERT INTO posts_fts(rowid, title, excerpt, content_md, slug)
    VALUES (new.id, new.title, new.excerpt, new.content_md, new.slug);
END;
