# personal-site

A complete personal website, blog and content-management system written **entirely in Rust**.
One binary, no JavaScript framework, no external services required.

```
┌──────────────────────────────────────────────────────────────────────┐
│  axum  ·  tokio  ·  askama  ·  sqlx (SQLite)  ·  comrak  ·  ammonia │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Table of contents

- [Highlights](#highlights)
- [Public site](#public-site)
- [Admin panel](#admin-panel)
- [Architecture](#architecture)
- [Getting started](#getting-started)
- [Configuration](#configuration)
- [Content model](#content-model)
- [Security](#security)
- [Deployment](#deployment)
- [Testing](#testing)
- [Project layout](#project-layout)
- [Licence](#licence)

---

## Highlights

| | |
|---|---|
| **Single binary** | Templates, CSS, JavaScript and migrations are compiled into the executable. Copy it, run it, done. |
| **Zero frontend framework** | The site is server-rendered HTML with a small progressive-enhancement JavaScript file. It works with JavaScript disabled. |
| **Full CMS** | Posts, pages, projects, tags, categories, comments, media, subscribers, users, settings, redirects and an activity log. |
| **Real security** | Argon2id password hashing, database-backed sessions, CSRF protection on every mutation, a strict CSP, rate limiting and HTML sanitisation on every byte of user content. |
| **Batteries included** | RSS feed, sitemap, `robots.txt`, web manifest, `security.txt`, Open Graph cards, JSON-LD, dark/light themes, search, pagination, e-mail notifications and newsletter campaigns. |
| **Fast** | Compiled templates, a single SQLite file in WAL mode, no runtime template parsing, no N+1 queries on the hot paths. |

---

## Public site

| Page | Route | What it does |
|---|---|---|
| **Home** | `/` | Hero with an animated word rotator, live statistics, featured projects, recent writing, skills, services, timeline and testimonials. |
| **About** | `/about` | Long-form biography (CMS-editable), personal facts, skill breakdown, career timeline, testimonials. |
| **Projects** | `/projects` | Filterable portfolio grid with technology tags, plus a case-study page per project. |
| **Blog** | `/blog` | Paginated article list with category/tag filters, full-text search, sidebar with taxonomy. |
| **Article** | `/blog/{slug}` | Rendered Markdown, auto-generated table of contents, reading time, word count, view counter, share buttons, related posts, threaded comments. |
| **Experience** | `/experience` | Work and education timelines plus a full technology breakdown. |
| **Skills** | `/skills` | Grouped proficiency bars with years of experience. |
| **Services** | `/services` | What you can hire me for, the engagement process, social proof. |
| **Testimonials** | `/testimonials` | Client quotes. |
| **Awards** | `/awards` | Recognition, talks and publications in one place. |
| **Speaking** | `/speaking` | Conference talks with video and slides. |
| **Publications** | `/publications` | Papers and articles. |
| **Gallery** | `/gallery` | Filterable photo grid. |
| **Résumé** | `/resume` | Print-optimised CV page with a PDF download. |
| **Contact** | `/contact` | Validated contact form with honeypot and timing anti-spam, plus direct contact details. |
| **Search** | `/search` | Cross-site search across articles, projects and pages. |
| **CMS pages** | `/{slug}` | Any page created in the admin panel is reachable at its slug. |
| **Feeds** | `/feed.xml`, `/sitemap.xml`, `/robots.txt` | RSS 2.0, XML sitemap and crawler policy. |

---

## Admin panel

Everything under `/admin`, behind an Argon2id-verified login with optional TOTP two-factor.

| Section | Capabilities |
|---|---|
| **Dashboard** | Post/comment/message/subscriber counters, 14-day traffic chart, most-visited pages, top articles, client breakdown, pending items. |
| **Posts** | Full CRUD, draft/published/scheduled/archived states, scheduling with automatic publishing, categories, tags, cover images, SEO fields, Markdown editor with **live preview**, quick status changes, search and filters. |
| **Pages** | CRUD for standalone pages, menu visibility and ordering, full-width or narrow reading column. |
| **Projects** | CRUD with technology stack, repository/live links, client, ordering and feature flags. |
| **Tags & categories** | Create, rename, recolour and delete; post counts are maintained automatically. |
| **Comments** | Approve, unapprove, mark as spam or delete. Threaded replies are rendered on the article. |
| **Messages** | Contact-form inbox with read/archived/spam states and one-click e-mail reply. |
| **Subscribers** | Newsletter audience, CSV export, removal. |
| **Media** | Drag-and-drop uploads (images, PDFs, archives), image dimensions, alt-text editing, deletion. |
| **Users** | Owner/editor roles, activation, password resets, two-factor enrolment, session revocation. |
| **Settings** | Grouped settings editor — identity, appearance (including theme colours), content, SEO, analytics, contact and newsletter. Changes apply immediately. |
| **Redirects** | 301/302/307/308 rules with hit counters. |
| **Newsletter** | Write campaigns in Markdown, personalise with `{{ name }}`, save as draft or send to the whole list. |
| **Activity log** | Every administrative action, with the actor, entity and timestamp. |
| **Backup** | Export every content table as JSON, import an export, or (carefully) reset the content tables. |
| **Profile** | Change your own name, e-mail, bio, avatar and password. |

---

## Architecture

```
src/
├── main.rs              binary entry point: config → database → router → serve
├── lib.rs               library root
├── config.rs            environment-driven configuration
├── settings.rs          typed, cached access to the settings table
├── state.rs             shared application state
├── error.rs             error type → HTML or JSON responses
├── audit.rs             administrative activity log
├── extract.rs           auth and CSRF extractors
├── markdown.rs          Markdown → sanitised HTML, TOC, reading time
├── mail.rs              SMTP delivery with a dry-run mode
├── util.rs              dates, slugs, pagination, hashing helpers
├── views.rs             view models consumed by the templates
├── templates.rs         Askama → HTTP response bridge
├── auth/
│   ├── password.rs      Argon2id hashing and strength scoring
│   ├── session.rs       database-backed sessions and CSRF tokens
│   └── totp.rs          RFC 6238 two-factor authentication
├── db/
│   ├── mod.rs           connection setup and migrations
│   └── seed.rs          first-run demo content
├── middleware/
│   ├── mod.rs           session loading and analytics
│   ├── ratelimit.rs     fixed-window rate limiter
│   └── security.rs      security headers and CSP
├── models/db.rs         row structs
└── handlers/
    ├── mod.rs           router assembly
    ├── context.rs       shared page context
    ├── data.rs          reusable data loaders
    ├── public.rs        public pages
    ├── blog.rs          blog listing, articles and comments
    ├── auth.rs          login, logout, password reset
    ├── feeds.rs         RSS, sitemap, robots, manifest
    └── admin/           the admin panel
```

**Request flow**

```
request
   └─ compression
      └─ rate limiter
         └─ security headers
            └─ session loader ──► AuthContext in request extensions
               └─ page-view recorder
                  └─ router
                     ├─ public handler ─► Askama template ─► HTML
                     └─ admin handler  ─► guard ─► CRUD ─► flash redirect
```

---

## Getting started

### Requirements

* Rust 1.80 or newer (stable)
* A C compiler (for SQLite's bundled amalgamation)

### Run it

```bash
git clone <your-repo> personal-site
cd personal-site

cp .env.example .env
$EDITOR .env          # set APP_SECRET and ADMIN_PASSWORD

cargo run --release
```

Open <http://localhost:3000>. The database is created, migrated and seeded with
realistic demo content on first boot.

The admin panel lives at <http://localhost:3000/admin> — sign in with the
`ADMIN_USERNAME` / `ADMIN_PASSWORD` from your `.env`.

### Development loop

```bash
cargo watch -x run          # auto-restart on change
cargo test                  # unit + integration tests
cargo fmt && cargo clippy   # style and lints
```

---

## Configuration

Everything is read from the environment (a `.env` file is loaded automatically).
See [`.env.example`](.env.example) for the full list; the important ones:

| Variable | Default | Notes |
|---|---|---|
| `APP_ENV` | `development` | `production` enables strict validation and secure cookies. |
| `APP_SECRET` | dev placeholder | **Required in production** — at least 32 random characters. Also salts the analytics IP hashes. |
| `BASE_URL` | `http://localhost:3000` | Used for canonical links, the sitemap and the feed. |
| `DATABASE_URL` | `sqlite://data/personal-site.db?mode=rwc` | Any SQLite path. |
| `ADMIN_USERNAME` / `ADMIN_EMAIL` / `ADMIN_PASSWORD` | `admin` / `admin@example.com` / `admin12345` | Used only when the database is empty. **Change the password.** |
| `SESSION_TTL_SECONDS` | `14400` | Idle session lifetime. |
| `REMEMBER_TTL_SECONDS` | `2592000` | "Keep me signed in" lifetime. |
| `SMTP_HOST`, `SMTP_PORT`, … | empty | Leave unset to log e-mails instead of sending them (`SMTP_DRY_RUN`). |
| `MAX_UPLOAD_SIZE` | `10485760` | 10 MiB upload limit. |
| `RUST_LOG` | `info,tower_http=warn,sqlx=warn` | Tracing filter. |

---

## Content model

| Table | Purpose |
|---|---|
| `users`, `sessions`, `password_resets` | Accounts, logins and recovery |
| `posts`, `categories`, `tags`, `post_tags` | The blog |
| `projects` | The portfolio |
| `pages` | Standalone CMS pages |
| `comments` | Threaded discussion |
| `messages` | Contact-form inbox |
| `subscribers`, `campaigns` | Newsletter |
| `media` | Uploaded files |
| `skills`, `experiences`, `services`, `testimonials` | Profile sections |
| `awards`, `talks`, `publications`, `gallery` | More profile sections |
| `social_links`, `settings`, `redirects` | Site configuration |
| `audit_log`, `page_views` | Activity and analytics |

Migrations live in `migrations/` and are applied automatically on boot.

---

## Security

* **Passwords** — Argon2id with 19 MiB of memory, two iterations and a per-password salt.
* **Sessions** — random 128-bit identifiers stored server-side; revocation is immediate.
* **Two-factor** — RFC 6238 TOTP with a ±1 step clock-drift window.
* **CSRF** — a per-session token required on every state-changing request.
* **Rate limiting** — per-IP limits on login, contact, subscription and preview endpoints.
* **XSS** — every byte of user content is rendered through `ammonia`'s allow-list sanitiser.
* **Headers** — CSP, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`,
  `Permissions-Policy`, `Strict-Transport-Security`.
* **Account lockout** — eight failed attempts locks the account for fifteen minutes.
* **Anti-spam** — honeypot fields plus a submission-timing check on public forms.
* **Enumeration** — the login and password-reset flows never reveal whether an account exists.
* **Uploads** — a content-type allow-list, sanitised filenames and a size cap.

---

## Deployment

### Docker

```bash
docker build -t personal-site .
docker run -p 3000:3000 \
  -v personal-site-data:/app/data \
  -v personal-site-media:/app/media \
  -e APP_ENV=production \
  -e APP_SECRET="$(openssl rand -base64 48)" \
  -e ADMIN_PASSWORD="a-strong-password" \
  personal-site
```

### Behind a reverse proxy

Terminate TLS at the proxy and forward to the application:

```nginx
location / {
    proxy_pass http://127.0.0.1:3000;
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

The application reads `X-Forwarded-For` to determine the client IP for rate limiting
and analytics.

### systemd

```ini
[Unit]
Description=personal-site
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/srv/personal-site
EnvironmentFile=/srv/personal-site/.env
ExecStart=/srv/personal-site/target/release/personal-site
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## Testing

```bash
cargo test              # unit + integration tests
cargo test --lib        # markdown sanitiser and TOTP vectors
cargo test --test site  # full-stack routing, auth, CSRF and validation
```

The integration tests boot the real application against an in-memory SQLite
database and exercise routing, template rendering, sessions, CSRF and form
validation end to end.

---

## Project layout

```
personal-site/
├── Cargo.toml
├── .env.example
├── Dockerfile
├── migrations/          SQL migrations, applied on boot
├── assets/              CSS, JavaScript and SVG artwork (embedded at build time)
│   ├── css/main.css     the design system
│   ├── css/admin.css    the admin panel
│   └── js/main.js       progressive enhancement only
├── templates/           Askama templates
│   ├── partials/        head, header, footer, cards, pagination
│   ├── pages/           public pages
│   └── admin/           admin panel
├── src/                 the application
└── tests/               integration tests
```

---

## Licence

MIT. Use it, fork it, make it yours.
