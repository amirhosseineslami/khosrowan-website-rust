-- Profile-section translations: experiences, services, skills, categories, tags.
-- Each row holds an English (or other locale) variant of a base entry.

CREATE TABLE IF NOT EXISTS experience_translations (
    experience_id   INTEGER NOT NULL REFERENCES experiences(id) ON DELETE CASCADE,
    locale          TEXT NOT NULL DEFAULT 'en',
    title           TEXT NOT NULL,
    company         TEXT NOT NULL DEFAULT '',
    location        TEXT,
    description     TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (experience_id, locale)
);

CREATE TABLE IF NOT EXISTS service_translations (
    service_id      INTEGER NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    locale          TEXT NOT NULL DEFAULT 'en',
    title           TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    features        TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (service_id, locale)
);

CREATE TABLE IF NOT EXISTS skill_translations (
    skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    locale          TEXT NOT NULL DEFAULT 'en',
    name            TEXT NOT NULL,
    category        TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (skill_id, locale)
);

CREATE TABLE IF NOT EXISTS category_translations (
    category_id     INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    locale          TEXT NOT NULL DEFAULT 'en',
    name            TEXT NOT NULL,
    description     TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (category_id, locale)
);

CREATE TABLE IF NOT EXISTS tag_translations (
    tag_id          INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    locale          TEXT NOT NULL DEFAULT 'en',
    name            TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (tag_id, locale)
);
