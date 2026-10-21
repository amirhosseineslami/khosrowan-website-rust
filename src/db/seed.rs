//! Seeds the database with a complete, believable site on first boot.
//!
//! Seeding is idempotent: it only runs when the database is still empty.

use crate::config::anyhow_lite::Result;
use sqlx::SqlitePool;

use crate::auth::password::hash_password;
use crate::config::Config;

/// Default site settings, grouped for the admin settings screen.
pub const DEFAULT_SETTINGS: &[(&str, &str, &str, &str)] = &[
    // key, value, type, group
    ("site.title", "Amir Rahimi", "text", "general"),
    ("site.subtitle", "Software Engineer & Systems Architect", "text", "general"),
    ("site.description", "I design and build resilient backends, distributed systems and developer tooling. Writing about Rust, performance and the craft of shipping software that lasts.", "textarea", "general"),
    ("site.tagline", "Building reliable systems, one commit at a time.", "text", "general"),
    ("site.email", "hello@example.com", "text", "general"),
    ("site.phone", "+49 151 23456789", "text", "general"),
    ("site.location", "Berlin, Germany · Remote worldwide", "text", "general"),
    ("site.available", "true", "bool", "general"),
    ("site.availability_note", "Currently available for select consulting & advisory engagements.", "text", "general"),
    ("site.logo_text", "AR", "text", "general"),
    ("site.favicon", "/assets/img/favicon.svg", "text", "general"),
    ("site.default_og_image", "/assets/img/og-default.svg", "text", "general"),
    ("site.cv_file", "/assets/files/cv.pdf", "text", "general"),
    ("site.theme_accent", "#6366f1", "color", "appearance"),
    ("site.theme_accent_2", "#22d3ee", "color", "appearance"),
    ("site.default_theme", "dark", "text", "appearance"),
    ("site.enable_dark_mode", "true", "bool", "appearance"),
    ("site.posts_per_page", "9", "number", "content"),
    ("site.enable_comments", "true", "bool", "content"),
    ("site.enable_newsletter", "true", "bool", "content"),
    ("site.allow_registration", "false", "bool", "content"),
    ("seo.meta_description", "Personal website and blog of Amir Rahimi — software engineer specialising in Rust, distributed systems and developer experience.", "textarea", "seo"),
    ("seo.og_type", "website", "text", "seo"),
    ("seo.twitter_site", "@amirrahimi", "text", "seo"),
    ("seo.google_site_verification", "", "text", "seo"),
    ("seo.bing_verification", "", "text", "seo"),
    ("analytics.provider", "none", "text", "analytics"),
    ("analytics.plausible_domain", "", "text", "analytics"),
    ("analytics.ga_measurement_id", "", "text", "analytics"),
    ("analytics.custom_head", "", "textarea", "analytics"),
    ("analytics.custom_footer", "", "textarea", "analytics"),
    ("contact.form_enabled", "true", "bool", "contact"),
    ("contact.success_message", "Thanks! Your message has been delivered — I'll get back to you within two business days.", "textarea", "contact"),
    ("contact.notify_email", "hello@example.com", "text", "contact"),
    ("newsletter.heading", "Subscribe for deep dives on Rust and system design.", "text", "newsletter"),
    ("newsletter.subheading", "One thoughtful email every couple of weeks. No spam, ever.", "text", "newsletter"),
];

pub async fn run(pool: &SqlitePool, config: &Config) -> Result<()> {
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count > 0 {
        return Ok(());
    }

    tracing::info!("empty database detected — seeding demo content");

    let password_hash = hash_password(&config.admin_password)?;
    let now = chrono::Utc::now();

    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (username, email, password_hash, role, display_name, bio, created_at, updated_at)
         VALUES (?, ?, ?, 'owner', 'Amir Rahimi', ?, ?, ?) RETURNING id",
    )
    .bind(&config.admin_username)
    .bind(&config.admin_email)
    .bind(&password_hash)
    .bind("Backend engineer with 10+ years of experience building high-throughput platforms. Currently focused on Rust, real-time systems and developer tooling.")
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    seed_settings(pool).await?;
    seed_taxonomy(pool).await?;
    seed_posts(pool, user_id).await?;
    seed_projects(pool).await?;
    seed_profile_sections(pool).await?;

    Ok(())
}

async fn seed_settings(pool: &SqlitePool) -> Result<()> {
    for (key, value, value_type, group) in DEFAULT_SETTINGS {
        sqlx::query(
            "INSERT OR REPLACE INTO settings (key, value, value_type, group_name) VALUES (?, ?, ?, ?)",
        )
        .bind(key)
        .bind(value)
        .bind(value_type)
        .bind(group)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_taxonomy(pool: &SqlitePool) -> Result<()> {
    let categories = [
        (
            "Engineering",
            "engineering",
            "Deep dives into software engineering practice.",
        ),
        ("Rust", "rust", "All about the Rust programming language."),
        (
            "Architecture",
            "architecture",
            "Designing systems that survive reality.",
        ),
        ("Notes", "notes", "Short, opinionated notes."),
        ("Tutorial", "tutorial", "Step-by-step walkthroughs."),
    ];
    for (name, slug, desc) in categories {
        sqlx::query("INSERT OR IGNORE INTO categories (name, slug, description) VALUES (?, ?, ?)")
            .bind(name)
            .bind(slug)
            .bind(desc)
            .execute(pool)
            .await?;
    }

    let tags = [
        ("Rust", "rust", "rust", "#f74c00"),
        ("Axum", "axum", "axum", "#8b5cf6"),
        ("PostgreSQL", "postgresql", "postgresql", "#336791"),
        (
            "Distributed Systems",
            "distributed-systems",
            "distributed-systems",
            "#10b981",
        ),
        ("Performance", "performance", "performance", "#f59e0b"),
        ("DevOps", "devops", "devops", "#0ea5e9"),
        ("Career", "career", "career", "#ec4899"),
        ("Open Source", "open-source", "open-source", "#22d3ee"),
        ("Architecture", "architecture", "architecture", "#6366f1"),
        ("Databases", "databases", "databases", "#14b8a6"),
    ];
    for (name, slug, _kind, hex) in tags {
        sqlx::query("INSERT OR IGNORE INTO tags (name, slug, color) VALUES (?, ?, ?)")
            .bind(name)
            .bind(slug)
            .bind(hex)
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn seed_posts(pool: &SqlitePool, author_id: i64) -> Result<()> {
    use crate::markdown;

    let posts: Vec<(&str, &str, &str, &str, &str, &str, i64)> = vec![
        (
            "building-a-production-web-stack-in-rust",
            "Building a Production Web Stack in Rust",
            "A tour of the architecture behind a high-traffic Rust service: routing, middleware, graceful shutdown and the boring parts that keep a system alive at 3am.",
            "engineering",
            "rust",
            r#"## Why Rust for the web?

Rust gives you two things that matter enormously when a service runs in production: **predictable performance** and **fearless concurrency**. There is no garbage collector pause, and the compiler rejects entire classes of bugs at compile time.

In this post I want to walk through the exact stack I use for services that handle millions of requests per day.

### The stack

| Layer | Choice | Why |
| --- | --- | --- |
| HTTP | `axum` | Composable extractors, tower middleware |
| Async | `tokio` | The de-facto standard runtime |
| Storage | `sqlx` | Compile-time checked SQL |
| Templating | `askama` | Type-safe templates, zero runtime cost |

### Middleware is the real architecture

Most people reach for a framework and then bolt middleware on top. With `tower` you get a *stack*, and that stack is your architecture:

```rust
let app = Router::new()
    .route("/posts", get(list_posts))
    .layer(middleware::from_fn(with_user))
    .layer(middleware::from_fn(rate_limit))
    .layer(CompressionLayer::new())
    .layer(TraceLayer::new_for_http());
```

Layers apply bottom-up, so `CompressionLayer` sits closest to the socket and `TraceLayer` sees the raw request. Getting this order right is the difference between a service that is easy to reason about and one that is not.

### Graceful shutdown

A service that drops in-flight requests on deploy is a service that loses money. Signal handling is straightforward:

```rust
async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok(); };
    let terminate = async {
        tokio::signal::unix::signal(SignalKind::terminate())
            .unwrap()
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
```

> The best incident is the one where the deploy went smoothly and nobody noticed.

### What I'd do differently

* Start with boring technology. The cleverest system is the one that still works when you're paged at 3am.
* Measure before you optimise. `tracing` plus a flamegraph beats intuition every single time.
* Keep the domain layer free of framework types. If your business logic mentions `axum::extract::Query`, it is not business logic.

The payoff is a service that is fast, predictable and — most importantly — *boring* to operate."#,
            30,
        ),
        (
            "the-type-system-is-a-teaching-tool",
            "The Type System Is a Teaching Tool",
            "Encoding invariants in your domain so the compiler teaches your team what the code means — and refuses to let them get it wrong.",
            "rust",
            "architecture",
            r#"## Types as documentation

A well-typed API answers most questions a caller could ask. If `new_user` takes a `ValidatedEmail` instead of a `String`, the fact that validation already happened is self-documenting.

```rust
pub struct ValidatedEmail(String);

impl TryFrom<&str> for ValidatedEmail {
    type Error = InvalidEmail;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parsed = EmailAddress::parse(value)
            .map_err(|_| InvalidEmail)?;
        Ok(Self(parsed.to_string()))
    }
}
```

### Make illegal states unrepresentable

This is the phrase that gets repeated so often it has lost its meaning, so let me make it concrete.

Bad:

```rust
enum Subscription {
    Trial { ends_at: DateTime<Utc> },
    Active { renews_at: DateTime<Utc> },
    Cancelled { cancelled_at: DateTime<Utc>, refund: bool },
}
```

Good:

```rust
enum Subscription {
    Trial(TrialPeriod),
    Active(ActivePeriod),
    Cancelled(CancelledPeriod),
}
```

Now each variant carries exactly the data that is valid for it, and the compiler will not let you read `trial.ends_at` from a cancelled subscription.

### The teaching effect

The real benefit is not safety, though safety is wonderful. The benefit is **onboarding**. A new engineer joining the team reads the type, and the type tells them the rules. Documentation rots; types do not."#,
            21,
        ),
        (
            "query-planning-for-humans",
            "Query Planning for Humans",
            "Reading EXPLAIN ANALYZE output without panicking, and the handful of statistics that actually matter in production.",
            "engineering",
            "databases",
            r#"## Start with the data you have

Before you rewrite the query, ask what the optimiser thinks it is doing:

```sql
EXPLAIN (ANALYZE, BUFFERS) SELECT ...;
```

The `Buffers` section is the most informative part, because it tells you about *disk* behaviour rather than CPU time.

### The four numbers that matter

1. **actual rows vs estimated rows** — a large gap means stale statistics.
2. **loops** — a nested loop with a high loop count is where your latency hides.
3. **rows removed by filter** — a huge number here is a missing index.
4. **shared read / hit** — tells you whether you are hitting disk or cache.

### Common fixes

* `ANALYZE` the table after a bulk load. This is shockingly often the whole fix.
* Add a covering index so the query can be answered from the index alone.
* Rewrite `NOT IN` as `NOT EXISTS` — the planner handles the latter far better.
* Be careful with functions applied to indexed columns; the index is not used.

> An index is not free. Every write to the table must also update the index, which is why `CREATE INDEX` on twenty columns is not a performance strategy.

Once you can read a plan, database performance stops being folklore and starts being engineering."#,
            14,
        ),
        (
            "notes-on-remote-first-collaboration",
            "Notes on Remote-First Collaboration",
            "Six years of distributed teams distilled into practices that actually held up.",
            "notes",
            "career",
            r#"## The things that actually mattered

### Write things down

Every decision that only lives in someone's head is a decision that will be lost, argued about again, or quietly reversed. A short ADR costs fifteen minutes and saves a sprint.

### Over-communicate the context, not the status

"Working on it" is noise. "Here is the trade-off I made between option A and option B, and why" is signal.

### Protect focus

Meetings are expensive. Not just for the attendees — for everyone who has to rebuild their context afterwards. Two status meetings a week beat six.

### Make space for the quiet ones

The fastest ideas in a distributed team often arrive asynchronously, from people who would never speak up in a meeting. Write-first cultures find those ideas.

### Be deliberately kind

Remote work removes most of the incidental social contact that resolves small misunderstandings in an office. You have to replace it deliberately, with explicit appreciation and directness."#,
            10,
        ),
        (
            "zero-downtime-schema-migrations",
            "Zero-Downtime Schema Migrations",
            "A repeatable playbook for evolving a live schema without taking the site down.",
            "tutorial",
            "postgresql",
            r#"## The expand / migrate / contract pattern

Every safe schema change is a sequence of three phases:

### 1. Expand

Add the new structure alongside the old one. No behaviour changes yet.

```sql
ALTER TABLE users ADD COLUMN display_name TEXT;
```

### 2. Migrate

Backfill in batches, then dual-write from the application.

```sql
UPDATE users SET display_name = username
WHERE id IN (
    SELECT id FROM users WHERE display_name IS NULL LIMIT 1000
);
```

### 3. Contract

Once nothing reads the old column, remove it.

```sql
ALTER TABLE users DROP COLUMN legacy_name;
```

## Rules I follow

* Never mix a schema change and a data-rewrite in the same release.
* Every migration must be backwards compatible with the *previous* application version, because during a rolling deploy both are running at once.
* Test the migration against a copy of production-sized data, not a toy dataset.
* Make every step idempotent so a failed deploy can be retried safely.

```bash
# Apply with a lock timeout so we never queue behind a long transaction.
psql -v ON_ERROR_STOP=1 -c "SET lock_timeout = '3s';" -f 0042_add_display_name.sql"
```

That `lock_timeout` line has saved me from more incidents than any other single piece of advice I have received."#,
            7,
        ),
        (
            "writing-a-cli-that-people-love",
            "Writing a CLI That People Actually Enjoy",
            "Command line ergonomics, exit codes, colours and the small details that turn a tool into a habit.",
            "engineering",
            "open-source",
            r#"## The five-second rule

If a user cannot tell what a tool does within five seconds of running `--help`, the tool is not finished.

```text
$ cargo-shim --help
A fast, opinionated workspace runner

Usage: cargo-shim <COMMAND>

Commands:
  run       Run a task in the workspace
  watch     Re-run a task when files change
  bench     Run the benchmark suite
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet       Decrease logging output
```

## Details worth the effort

* **Colour, but only when it helps.** Respect `NO_COLOR` and non-TTY output.
* **Good exit codes.** `0` success, `1` general failure, `2` usage error, `3` validation error.
* **Machine-readable output.** Add `--json` to every command that produces data.
* **Predictable output.** Sorted where order is not meaningful.

```rust
fn exit_code_for(err: &CliError) -> i32 {
    match err {
        CliError::Usage(_) => 2,
        CliError::Validation(_) => 3,
        CliError::Io(_) => 4,
        _ => 1,
    }
}
```

## The test that matters

Run the tool as a user would, in a fresh shell, on a clean machine. If it does not work there, it does not work."#,
            4,
        ),
        (
            "on-mentorship",
            "On Mentorship",
            "What actually changes when someone senior takes the time to unblock you.",
            "notes",
            "career",
            r#"## Mentorship is not review

Code review is a checkpoint. Mentorship is a trajectory. The difference shows up months later, when the person no longer needs you.

## Three things a good mentor does

1. **Asks better questions than you would have asked.** Not "why did you do this?" but "what did you expect to happen, and what did happen instead?"
2. **Explains the *why*, not just the *what*.** Rules without reasoning become cargo cult.
3. **Gives you the hard problem, then gets out of the way.**

## Three things a bad mentor does

1. Takes over the keyboard.
2. Says "just do it this way" and refuses to explain why.
3. Is unavailable exactly when you need them.

## What I got wrong for years

I thought being helpful meant having the answer. It usually means having the question that helps someone find the answer themselves. That is slower in the moment and much faster overall."#,
            2,
        ),
    ];

    for (idx, (slug, title, excerpt, cat, tag, body, days_ago)) in posts.iter().enumerate() {
        let published_at = chrono::Utc::now() - chrono::Duration::days(*days_ago);
        let html = markdown::to_html(body);
        let reading_time = markdown::reading_time(body);

        let post_id: i64 = sqlx::query_scalar(
            "INSERT INTO posts (slug, title, excerpt, content_md, content_html, status, category_id, author_id,
                                is_featured, allow_comments, published_at, reading_time, view_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 'published',
                     (SELECT id FROM categories WHERE slug = ?),
                     ?, ?, 1, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(*slug)
        .bind(*title)
        .bind(*excerpt)
        .bind(*body)
        .bind(&html)
        .bind(*cat)
        .bind(author_id)
        .bind(idx == 0)
        .bind(published_at)
        .bind(reading_time)
        .bind(120 * (idx as i64 + 3))
        .bind(published_at)
        .bind(published_at)
        .fetch_one(pool)
        .await?;

        let tag_id: Option<i64> = sqlx::query_scalar("SELECT id FROM tags WHERE slug = ?")
            .bind(*tag)
            .fetch_optional(pool)
            .await?;
        if let Some(tag_id) = tag_id {
            sqlx::query("INSERT OR IGNORE INTO post_tags (post_id, tag_id) VALUES (?, ?)")
                .bind(post_id)
                .bind(tag_id)
                .execute(pool)
                .await?;
        }
    }

    // A draft so the admin panel has something to show.
    let draft_body = r#"## Draft

This post is still being written.

- [x] Outline
- [ ] First draft
- [ ] Diagrams
- [ ] Review"#;
    sqlx::query(
        "INSERT INTO posts (slug, title, excerpt, content_md, content_html, status, author_id, created_at, updated_at)
         VALUES ('post-quantum-cryptography-primer', 'A Post-Quantum Cryptography Primer',
                 'What changes when the machines that break your encryption exist?',
                 ?, ?, 'draft', ?, ?, ?)",
    )
    .bind(draft_body)
    .bind(markdown::to_html(draft_body))
    .bind(author_id)
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .execute(pool)
    .await?;

    // A scheduled post.
    sqlx::query(
        "INSERT INTO posts (slug, title, excerpt, content_md, content_html, status, author_id, published_at, scheduled_for, created_at, updated_at)
         VALUES ('engineering-in-the-age-of-agents', 'Engineering in the Age of Agents',
                 'How the software we write changes when it can write more software.',
                 ?, ?, 'scheduled', ?, ?, ?, ?, ?)",
    )
    .bind(r#"## Coming soon

A short piece on how agentic tooling changes the shape of a team."#)
    .bind(markdown::to_html(r#"## Coming soon

A short piece on how agentic tooling changes the shape of a team."#))
    .bind(author_id)
    .bind(chrono::Utc::now() - chrono::Duration::days(1))
    .bind(chrono::Utc::now() + chrono::Duration::days(3))
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_projects(pool: &SqlitePool) -> Result<()> {
    let projects = [
        (
            "helios",
            "Helios",
            "A distributed job scheduler with exactly-once semantics, built for teams that cannot afford a duplicate invoice.",
            r#"## The problem

Every team eventually builds a scheduler, and every one of them discovers the hard part: **exactly-once execution** in a system where the network lies.

## What Helios does

Helios is a Rust job scheduler with a durable, append-only event log. Every state transition is an event; every event is idempotent. A job runs exactly once even if workers crash, network partitions or a node is yanked out of the rack mid-execution.

### Architecture

- **Control plane** — API, scheduler, and event log writer.
- **Data plane** — stateless workers that lease jobs with a heartbeat.
- **Guarantees** — at-least-once delivery with idempotent handlers, or exactly-once with the transactional outbox pattern.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum Transition {
    Scheduled { at: DateTime<Utc> },
    Leased { worker: WorkerId, until: DateTime<Utc> },
    Completed { output: JobOutput },
    Failed { reason: String, retry_in: Duration },
}
```

## Results

* p99 scheduling latency under 40 ms across three regions
* 1.2 million jobs per day at peak
* Zero duplicate executions in 18 months of production"#,
            "https://github.com/example/helios",
            "https://helios.example.com",
            "#ef4444",
        ),
        (
            "quill",
            "Quill",
            "A fast, opinionated Markdown publishing toolkit with a WASM core so it runs anywhere.",
            r#"## Why another Markdown tool

Because every existing tool either does too little or makes you fight it.

## Highlights

* **Portable core.** The parser and renderer are compiled to WebAssembly, so the same binary powers the CLI, the web playground and the VS Code extension.
* **Fast.** Roughly 40× faster than the reference implementation on large documents.
* **Opinionated.** Sensible defaults for tables, footnotes, task lists and front matter.

```bash
quill build ./posts --output ./dist --theme noir
```

## Adoption

Used by three of my own sites and about 1,400 GitHub stars."#,
            "https://github.com/example/quill",
            "https://quill.example.com",
            "#8b5cf6",
        ),
        (
            "atlas-observability",
            "Atlas",
            "A lightweight observability collector that fits in a sidecar and speaks OpenTelemetry.",
            r#"## Small, sharp, and cheap

Atlas is an OpenTelemetry collector with a narrow goal: **be the smallest thing that can be deployed as a sidecar**.

## What it does

* Accepts OTLP over gRPC and HTTP.
* Exports to Prometheus, Jaeger, Loki or any OTLP endpoint.
* Ships a single statically-linked binary of 4 MB.

```yaml
receivers:
  otlp:
    protocols:
      grpc: {}
processors:
  batch:
    timeout: 2s
exporters:
  prometheus:
    endpoint: 0.0.0.0:9464
```

## Design notes

The whole collector fits in a single tokio task per pipeline stage. Backpressure is explicit: when an exporter stalls, the batch processor drops the oldest metrics rather than growing without bound. Losing old telemetry beats running out of memory."#,
            "https://github.com/example/atlas",
            "",
            "#22d3ee",
        ),
        (
            "kiln",
            "Kiln",
            "A tiny, type-safe job queue for Rust applications with SQLite or Postgres backing.",
            r#"## Queues without the ceremony

Kiln is a job queue that fits in a single file of your codebase.

```rust
let queue = Queue::builder("sqlite://jobs.db")
    .max_attempts(5)
    .backoff(Backoff::Exponential { base: Duration::from_secs(2) })
    .build()
    .await?;

queue.enqueue(Job::SendWelcomeEmail { user_id: 42 }).await?;
```

## Features

* At-least-once delivery with exponential backoff and jitter
* Dead-letter queue with a web UI
* Poison message handling that never blocks the queue
* Zero-dependency core"#,
            "https://github.com/example/kiln",
            "",
            "#f59e0b",
        ),
        (
            "driftwood",
            "Driftwood",
            "A terminal toolkit for reading logs like a story — time-based context, diffing, and colour that means something.",
            r#"## Logs deserve better

Driftwood reads structured logs and renders them as a timeline, with colour that encodes severity consistently across every backend.

## Features

* Time-windowed context around every error
* Automatic diffing between two runs of the same test
* A TUI that stays responsive on million-line inputs
* Shipped as a single binary with no runtime dependencies"#,
            "https://github.com/example/driftwood",
            "",
            "#10b981",
        ),
        (
            "weathervane",
            "Weathervane",
            "A feature-flag service with a hard guarantee: evaluation is local, so a flag can never slow down your request path.",
            r#"## Flags without latency

Weathervane compiles flags into a compact binary blob that is refreshed every few seconds. Evaluation is a memory lookup — nanoseconds, no network.

## Guarantees

* Stale-while-revalidate: the last known good value is always served
* Every evaluation is local; the network is never on the request path
* Full audit trail of who changed what and when"#,
            "https://github.com/example/weathervane",
            "",
            "#ec4899",
        ),
    ];

    for (idx, (slug, title, summary, body, repo, live, color)) in projects.iter().enumerate() {
        let html = crate::markdown::to_html(body);
        sqlx::query(
            "INSERT INTO projects (slug, title, summary, description_md, description_html, status, repo_url, live_url,
                                   is_featured, sort_order, tech_stack, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 'active', ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(*slug)
        .bind(*title)
        .bind(*summary)
        .bind(*body)
        .bind(&html)
        .bind(*repo)
        .bind(if live.is_empty() { None } else { Some(*live) })
        .bind(idx < 3)
        .bind(idx as i64)
        .bind(r#"["Rust","Tokio","PostgreSQL"]"#)
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
        let _ = color;
    }
    Ok(())
}

async fn seed_profile_sections(pool: &SqlitePool) -> Result<()> {
    // Skills
    let skills = [
        ("Rust", "languages", 95, 8.0),
        ("Go", "languages", 85, 7.0),
        ("Python", "languages", 88, 9.0),
        ("TypeScript", "languages", 80, 6.0),
        ("SQL", "languages", 90, 10.0),
        ("Axum / Actix", "backend", 92, 5.0),
        ("Event-driven design", "backend", 88, 6.0),
        ("gRPC & REST APIs", "backend", 90, 8.0),
        ("PostgreSQL", "data", 87, 8.0),
        ("Redis", "data", 80, 7.0),
        ("ClickHouse", "data", 70, 3.0),
        ("Kafka", "data", 72, 4.0),
        ("Docker & Kubernetes", "ops", 84, 7.0),
        ("Terraform", "ops", 76, 5.0),
        ("CI/CD pipelines", "ops", 88, 8.0),
        ("Observability", "ops", 79, 6.0),
    ];
    for (i, (name, cat, level, years)) in skills.iter().enumerate() {
        sqlx::query(
            "INSERT INTO skills (name, category, level, years, sort_order) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(*name)
        .bind(*cat)
        .bind(*level)
        .bind(*years)
        .bind(i as i64)
        .execute(pool)
        .await?;
    }

    // Experience
    let experiences = [
        (
            "Principal Engineer",
            "Northwind Systems",
            "Berlin, Germany · Remote",
            "2022",
            None,
            true,
            "Leading the platform group: 12 engineers across ingestion, storage and the developer experience team. Cut p99 API latency by 61% while doubling throughput.",
            "work",
        ),
        (
            "Senior Backend Engineer",
            "Helix Analytics",
            "Amsterdam, Netherlands",
            "2019",
            Some("2022"),
            false,
            "Designed the event ingestion pipeline that now processes 40 billion events per month. Mentored six engineers, two of whom are now staff engineers.",
            "work",
        ),
        (
            "Backend Engineer",
            "Vireo Labs",
            "Remote",
            "2016",
            Some("2019"),
            false,
            "Built the multi-tenant billing service and its reconciliation engine. Introduced contract testing that reduced production incidents by 70%.",
            "work",
        ),
        (
            "Software Engineer",
            "Corvid Robotics",
            "Tehran, Iran",
            "2014",
            Some("2016"),
            false,
            "Firmware and control software for industrial robotics. Wrote the real-time scheduler that coordinated 200+ actuators.",
            "work",
        ),
        (
            "M.Sc. Computer Science",
            "Sharif University of Technology",
            "Tehran, Iran",
            "2012",
            Some("2014"),
            false,
            "Thesis on fault-tolerant consensus protocols for geo-distributed databases. Graduated with distinction.",
            "education",
        ),
        (
            "B.Sc. Software Engineering",
            "University of Tehran",
            "Tehran, Iran",
            "2008",
            Some("2012"),
            false,
            "Focus on distributed systems and compilers. Founded the campus Rust user group.",
            "education",
        ),
    ];
    for (i, e) in experiences.iter().enumerate() {
        sqlx::query(
            "INSERT INTO experiences (title, company, location, start_date, end_date, is_current, description, kind, sort_order)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(e.0)
        .bind(e.1)
        .bind(e.2)
        .bind(e.3)
        .bind(e.4)
        .bind(e.5)
        .bind(e.6)
        .bind(e.7)
        .bind(i as i64)
        .execute(pool)
        .await?;
    }

    // Services
    let services = [
        (
            "Backend Architecture Review",
            "A rigorous review of your service architecture — from data model to deployment topology — with a written report and concrete, prioritised recommendations.",
            "architecture",
            "from €2,500",
            r#"["Architecture decision record","Scalability & capacity plan","Risk register","90-minute follow-up call"]"#,
        ),
        (
            "Rust Adoption & Training",
            "Hands-on training that takes a team from zero to shipping production Rust, including a project they own from day one.",
            "rust",
            "from €1,800",
            r#"["Two-day workshop","Cohort project","Code review playbook","Three-month mentoring retainer"]"#,
        ),
        (
            "Performance & Reliability Audit",
            "Find the bottleneck before your users do. Profiling, query tuning, capacity modelling and a prioritised remediation plan.",
            "performance",
            "from €2,000",
            r#"["Load-test harness","Profiling report","Capacity model","Prioritised backlog"]"#,
        ),
        (
            "Technical Due Diligence",
            "An independent, evidence-based assessment of an engineering organisation for investors and acquirers.",
            "advisory",
            "from €4,000",
            r#"["Team & process review","Architecture assessment","Risk register","Confidential findings"]"#,
        ),
    ];
    for (i, s) in services.iter().enumerate() {
        sqlx::query(
            "INSERT INTO services (title, description, icon, price_from, features, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(s.0)
        .bind(s.1)
        .bind(s.2)
        .bind(s.3)
        .bind(s.4)
        .bind(i as i64)
        .execute(pool)
        .await?;
    }

    // Testimonials
    let testimonials = [
        (
            "Sarah Mitchell",
            "VP Engineering",
            "Northwind Systems",
            "Amir joined us for a six-month engagement and left behind a platform team that is faster, calmer and far better documented than when he arrived. He is the rare engineer who improves the code *and* the people around it.",
            5,
            true,
        ),
        (
            "Daniel Okafor",
            "CTO",
            "Helix Analytics",
            "We brought Amir in to untangle an ingestion pipeline that had defeated three previous contractors. He found the real problem in a week, wrote it down clearly, and left a team that could maintain it without him.",
            5,
            true,
        ),
        (
            "Elena Novak",
            "Head of Product",
            "Vireo Labs",
            "Technically extraordinary, but the thing I remember is the communication. Weekly updates that were genuinely useful, bad news raised early, and a plan that never slipped.",
            5,
            true,
        ),
        (
            "Ravi Menon",
            "Engineering Manager",
            "Kestrel",
            "The architecture review alone paid for the engagement. We implemented four of the six recommendations in the first quarter and our on-call rota went from nightly to weekly.",
            4,
            false,
        ),
        (
            "Marta Kowalski",
            "Founder",
            "Lumen Analytics",
            "Amir is the rare consultant who will tell you when you do not need him. He scoped a two-week engagement, delivered it, and told us exactly what we could do ourselves afterwards.",
            5,
            false,
        ),
    ];
    for (i, t) in testimonials.iter().enumerate() {
        sqlx::query(
            "INSERT INTO testimonials (name, role, company, body, rating, is_featured, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(t.0)
        .bind(t.1)
        .bind(t.2)
        .bind(t.3)
        .bind(t.4)
        .bind(t.5)
        .bind(i as i64)
        .execute(pool)
        .await?;
    }

    // Awards
    let awards = [
        (
            "Best Paper — Systems Track",
            "ACM ICSE",
            "2024",
            "For the paper on predictable backpressure in tiered storage.",
        ),
        (
            "Open Source Maintainer of the Year",
            "Rust Community",
            "2023",
            "Recognised for stewardship of several widely used Rust crates.",
        ),
        (
            "Distinguished Speaker",
            "FOSDEM",
            "2023",
            "Invited talk on ownership patterns in large Rust codebases.",
        ),
        (
            "Engineering Excellence Award",
            "Northwind Systems",
            "2022",
            "For the platform reliability programme.",
        ),
    ];
    for (i, a) in awards.iter().enumerate() {
        sqlx::query("INSERT INTO awards (title, issuer, awarded_on, description, sort_order) VALUES (?, ?, ?, ?, ?)")
            .bind(a.0)
            .bind(a.1)
            .bind(a.2)
            .bind(a.3)
            .bind(i as i64)
            .execute(pool)
            .await?;
    }

    // Talks
    let talks = [
        (
            "Ownership Without the Fear",
            "FOSDEM",
            "2023",
            "Brussels, Belgium",
            Some("https://video.example.com/fosdem2023"),
            None,
        ),
        (
            "Backpressure: The Hardest Part",
            "SREcon",
            "2022",
            "Amsterdam, Netherlands",
            None,
            Some("https://slides.example.com/srecon2022"),
        ),
        (
            "Building a Job Scheduler in Rust",
            "Rust Nation",
            "2021",
            "London, United Kingdom",
            None,
            None,
        ),
        (
            "Zero-Downtime Migrations",
            "DataCouncil",
            "2020",
            "Berlin, Germany",
            None,
            None,
        ),
    ];
    for (i, t) in talks.iter().enumerate() {
        sqlx::query("INSERT INTO talks (title, event, date, location, video_url, slides_url, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(t.0)
            .bind(t.1)
            .bind(t.2)
            .bind(t.3)
            .bind(t.4)
            .bind(t.5)
            .bind(i as i64)
            .execute(pool)
            .await?;
    }

    // Publications
    let publications = [
        (
            "Predictable Backpressure in Tiered Storage Systems",
            "ACM Transactions on Storage",
            "2024",
            "https://doi.org/example/1",
            "Co-authored with three researchers at TU Delft.",
        ),
        (
            "Ownership Patterns for Long-Lived Rust Services",
            "IEEE Software",
            "2023",
            "https://doi.org/example/2",
            "A practical taxonomy of ownership in production Rust.",
        ),
        (
            "A Field Study of Schema Migrations at Scale",
            "VLDB",
            "2022",
            "https://doi.org/example/3",
            "Analysis of 14,000 production migrations across six companies.",
        ),
    ];
    for (i, p) in publications.iter().enumerate() {
        sqlx::query("INSERT INTO publications (title, publisher, date, url, summary, sort_order) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(p.0)
            .bind(p.1)
            .bind(p.2)
            .bind(p.3)
            .bind(p.4)
            .bind(i as i64)
            .execute(pool)
            .await?;
    }

    // Gallery
    let gallery = [
        (
            "Mountain ridge at dawn",
            "landscape",
            "Shot on a cold morning above 2,400 m.",
        ),
        (
            "The office whiteboard",
            "workspace",
            "Architecture diagrams, ad infinitum.",
        ),
        ("RustConf stage", "talks", "2023, FOSDEM."),
        (
            "Old harbour, blue hour",
            "landscape",
            "Somewhere between meetings.",
        ),
        (
            "Ceramic glazes",
            "hobbies",
            "A recent and deeply unserious pursuit.",
        ),
        (
            "Server room, 3am",
            "workspace",
            "The most honest photograph in this gallery.",
        ),
    ];
    for (i, g) in gallery.iter().enumerate() {
        sqlx::query("INSERT INTO gallery (title, image, caption, category, sort_order) VALUES (?, ?, ?, ?, ?)")
            .bind(g.0)
            .bind(format!("/assets/img/gallery/{}.svg", i + 1))
            .bind(g.2)
            .bind(g.1)
            .bind(i as i64)
            .execute(pool)
            .await?;
    }

    // Social links
    let socials = [
        ("github", "https://github.com/example", "GitHub"),
        ("linkedin", "https://linkedin.com/in/example", "LinkedIn"),
        ("x", "https://x.com/example", "X"),
        ("mastodon", "https://mastodon.social/@example", "Mastodon"),
        ("rss", "/feed.xml", "RSS Feed"),
        ("email", "mailto:hello@example.com", "Email"),
    ];
    for (i, (platform, url, label)) in socials.iter().enumerate() {
        sqlx::query(
            "INSERT INTO social_links (platform, url, label, sort_order) VALUES (?, ?, ?, ?)",
        )
        .bind(*platform)
        .bind(*url)
        .bind(*label)
        .bind(i as i64)
        .execute(pool)
        .await?;
    }

    // A couple of pages for the CMS.
    let pages = [
        (
            "about",
            "About",
            r#"## The short version

I am a backend engineer who spends most of my time on the parts of software that
only show up months later: the shape of a module, the failure mode of a deployment,
the sentence in a runbook that saves somebody an hour at 3am.

I have spent the last decade building high-throughput ingestion pipelines, event
driven backends and the developer tooling that makes them bearable. Most of that
work has been in Rust and Go, against PostgreSQL and the various queues and caches
people insist on putting in front of it.

## What I care about

* **Boring technology.** The cleverest system is the one that still works when you
  are paged at 3am.
* **Written decisions.** Anything that lives only in someone's head gets lost,
  argued about again, or quietly reversed.
* **Measured performance.** A flamegraph beats intuition, every single time.
* **Clear boundaries.** If your business logic mentions a framework type, it is not
  business logic.

## How I work

Small teams, high trust, and a strong preference for handing over working software
rather than status updates. I write things down — architecture decision records,
runbooks, and the occasional long article about why a particular approach did not
work.

## Outside the editor

I climb, I cook far too much for two people, and I maintain a small number of open
source Rust crates that I use in my own work. I am always happy to talk about any of
it."#,
            false,
            8,
        ),
        (
            "uses",
            "Uses",
            r#"## Hardware

I keep the desk simple and the machine fast.

* **MacBook Pro 16"** — M3 Max, 64 GB RAM. The compiler is the bottleneck, so the CPU is the tool.
* **A 27" 5K display** calibrated to sRGB. Colour accuracy matters more than resolution.
* **A real keyboard** — mechanical, quiet, with slightly heavy switches. Nothing else survives a long session.
* **A standing desk** — because back pain is a career risk.

## Editor

* **Neovim** with a small, deliberate configuration. I keep it fast and I keep it mine.
* **Zellij** for panes, but I mostly use `:split` because muscle memory beats novelty.
* **ripgrep**, `fd`, `fzf`, `jq`, `bat`, `delta`, `lazygit`. The boring UNIX tools done well.

## Terminal

* **Ghostty** — fast, native, no configuration archaeology.
* **starship** prompt, **fish** shell (zsh was fine; fish is faster to type in).
* **atuin** for shell history with fuzzy search.

## Software I actually pay for

* **1Password** — the only password manager that has never let me down.
* **Linear** — issue tracking that does not create busywork.
* **Raycast** — a launcher that replaced four other applications.

## Desk setup

The goal is that nothing on the desk competes for attention. The monitor is on an arm, the keyboard is directly in front of me, and the notebook is for thinking rather than for lists."#,
            true,
            3,
        ),
        (
            "colophon",
            "Colophon",
            r#"## How this site is built

This website is a single Rust binary. No JavaScript framework, no CMS, no build step for the front-end.

* **Framework** — [Axum](https://github.com/tokio-rs/axum) on top of Tokio.
* **Templating** — [Askama](https://github.com/askama-rs/askama), compiled at build time, so a broken template is a compile error rather than a 500 in production.
* **Database** — SQLite in WAL mode through [SQLx](https://github.com/launchbadge/sqlx).
* **Content** — Markdown rendered with [comrak](https://github.com/kivikakk/comrak) and sanitised with [ammonia](https://github.com/raphaelm/ammonia) before it is ever stored.
* **Security** — Argon2id password hashing, database-backed sessions, CSRF protection, and strict content-security headers.

## Design

The visual language is deliberately restrained: a deep neutral canvas, one accent colour, generous whitespace and typography that does the work. Light and dark themes are both first-class, and the choice is remembered.

## Philosophy

The site is a static-ish document with a small dynamic core. Every request that could be a file is a file. Everything else is served from a database that I can reason about, back up with one command, and migrate with a script.

> If it needs a JavaScript framework to render text, it is using the wrong tool."#,
            false,
            9,
        ),
    ];
    for (slug, title, body, in_nav, order) in pages {
        sqlx::query(
            "INSERT INTO pages (slug, title, content_md, content_html, status, show_in_nav, nav_order, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'published', ?, ?, ?, ?)",
        )
        .bind(slug)
        .bind(title)
        .bind(body)
        .bind(crate::markdown::to_html(body))
        .bind(in_nav)
        .bind(order)
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
    }

    // Keep the cached tag and category counters accurate.
    sqlx::query(
        "UPDATE tags SET post_count = (
            SELECT COUNT(*) FROM post_tags pt JOIN posts p ON p.id = pt.post_id
            WHERE pt.tag_id = tags.id AND p.status = 'published')",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE categories SET post_count = (
            SELECT COUNT(*) FROM posts p WHERE p.category_id = categories.id AND p.status = 'published')",
    )
    .execute(pool)
    .await?;

    // Redirects
    sqlx::query("INSERT OR IGNORE INTO redirects (from_path, to_path, status_code) VALUES ('/home', '/', 301)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO redirects (from_path, to_path, status_code) VALUES ('/articles', '/blog', 301)")
        .execute(pool)
        .await?;

    Ok(())
}
