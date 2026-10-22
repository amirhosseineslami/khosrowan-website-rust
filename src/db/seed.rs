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
    ("site.title", "Amirhossein Eslami", "text", "general"),
    (
        "site.subtitle",
        "Electrical & Control Engineering · AI & Robotics",
        "text",
        "general",
    ),
    (
        "site.description",
        "Electrical engineering (control) student at Iran University of Science and Technology, focused on robotics, AI/ML, control systems, computer vision and intelligent engineering solutions. Researching decentralized and safe formation control for mobile robots.",
        "textarea",
        "general",
    ),
    (
        "site.tagline",
        "The future belongs to systems that can sense, learn, adapt, and collaborate.",
        "text",
        "general",
    ),
    ("site.email", "amirhossein.eslami.ac@gmail.com", "text", "general"),
    ("site.phone", "", "text", "general"),
    (
        "site.location",
        "Tehran, Iran · Iran University of Science and Technology",
        "text",
        "general",
    ),
    ("site.available", "true", "bool", "general"),
    (
        "site.availability_note",
        "Open to research collaborations, internships and mentorship.",
        "text",
        "general",
    ),
    ("site.logo_text", "AE", "text", "general"),
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
    (
        "seo.meta_description",
        "Personal website of Amirhossein Eslami — electrical & control engineering student at IUST working on robotics, AI/ML, computer vision and intelligent control.",
        "textarea",
        "seo",
    ),
    ("seo.og_type", "website", "text", "seo"),
    (
        "seo.twitter_site",
        "@amirhosseineslami",
        "text",
        "seo",
    ),
    ("seo.google_site_verification", "", "text", "seo"),
    ("seo.bing_verification", "", "text", "seo"),
    ("analytics.provider", "none", "text", "analytics"),
    ("analytics.plausible_domain", "", "text", "analytics"),
    ("analytics.ga_measurement_id", "", "text", "analytics"),
    ("analytics.custom_head", "", "textarea", "analytics"),
    ("analytics.custom_footer", "", "textarea", "analytics"),
    ("contact.form_enabled", "true", "bool", "contact"),
    (
        "contact.success_message",
        "Thanks! Your message has been delivered — I'll get back to you soon.",
        "textarea",
        "contact",
    ),
    (
        "contact.notify_email",
        "amirhossein.eslami.ac@gmail.com",
        "text",
        "contact",
    ),
    (
        "newsletter.heading",
        "Subscribe for deep dives on AI, robotics and control engineering.",
        "text",
        "newsletter",
    ),
    (
        "newsletter.subheading",
        "One thoughtful email every couple of weeks. No spam, ever.",
        "text",
        "newsletter",
    ),
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
         VALUES (?, ?, ?, 'owner', 'Amirhossein Eslami', ?, ?, ?) RETURNING id",
    )
    .bind(&config.admin_username)
    .bind(&config.admin_email)
    .bind(&password_hash)
    .bind("An Electrical Engineering (Control) student at Iran University of Science and Technology (IUST), interested in robotics, AI/ML, control systems, computer vision, data science, and intelligent engineering solutions.")
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
            "Machine Learning",
            "machine-learning",
            "Models, algorithms and the mathematics behind learning from data.",
        ),
        (
            "Robotics",
            "robotics",
            "Robots, embedded systems and the software that makes them move.",
        ),
        (
            "Computer Vision",
            "computer-vision",
            "Teaching machines to see: detection, segmentation and beyond.",
        ),
        (
            "Control Systems",
            "control-systems",
            "Feedback, dynamics and making physical systems behave.",
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
        (
            "Reinforcement Learning",
            "reinforcement-learning",
            "#f59e0b",
        ),
        (
            "Multi-Agent Systems",
            "multi-agent-systems",
            "#8b5cf6",
        ),
        ("Deep Learning", "deep-learning", "#6366f1"),
        ("Computer Vision", "computer-vision", "#22d3ee"),
        ("Python", "python", "#10b981"),
        ("Arduino", "arduino", "#0ea5e9"),
        ("Data Science", "data-science", "#14b8a6"),
        ("Research", "research", "#ec4899"),
        ("Open Source", "open-source", "#22d3ee"),
        ("YOLO", "yolo", "#ef4444"),
    ];
    for (name, slug, hex) in tags {
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
            "building-a-worm-like-crawling-robot-with-rl",
            "Building a Worm-like Crawling Robot with Reinforcement Learning",
            "How I taught an Arduino-powered servo worm to crawl using reinforcement learning — and what the project taught me about reward shaping, sim-to-real gaps and patience.",
            "robotics",
            "reinforcement-learning",
            r#"## A robot that learns to crawl

The premise was simple: build a worm-like robot from servos and an Arduino, then use reinforcement learning to discover a crawling gait instead of hand-coding one.

### The hardware

- An Arduino board driving five servo motors arranged in a line, mimicking a segmented worm.
- Each segment flexes independently, and the whole body works together to inch forward.

### The learning setup

The state is a compact representation of the servo angles. The action is the set of target angles for the next timestep. The reward pushes the robot to maximise forward displacement while staying within safe joint limits:

```python
reward = forward_displacement - angular_penalty
```

### What actually took the time

1. **Reward shaping.** A naive "just move forward" reward produces frantic, inefficient motion. Adding a smoothness penalty changed the gait from twitchy to sinusoidal.
2. **Guardrails.** Without joint-limit penalties the agent learned to slam the servos, which strips gears. Physics punishes greed.
3. **Patience.** Early runs looked broken. It took many episodes before a coherent gait emerged.

## Takeaways

Building a physical learning system is fundamentally different from a simulated one. Every episode costs real time, real power and real wear — so you think harder about sample efficiency and safety. That constraint made the project far more valuable than any clean simulation ever would have."#,
            30,
        ),
        (
            "detecting-cars-in-aerial-images-with-yolov8",
            "Detecting Cars in Aerial Parking Imagery with YOLOv8",
            "Training a YOLOv8 model to count vehicles in aerial parking-lot images — from data labelling to a working detection pipeline.",
            "computer-vision",
            "yolo",
            r#"## The problem

Aerial images of parking lots present a deceptively hard vision problem: cars are small, dense, frequently occluded and viewed from unusual angles.

## The approach

I fine-tuned a YOLOv8 detector on aerial imagery, then used the detections to estimate how many vehicles are present in a given lot.

### Pipeline

1. **Dataset** — annotated aerial images of parking areas.
2. **Training** — a YOLOv8 model fine-tuned on the backbone, with careful anchor and image-size choices to keep small cars detectable.
3. **Inference** — run the model over new aerial frames.
4. **Counting** — aggregate detections into a vehicle count and log it as a result.

## Practical lessons

- **Small objects need resolution.** Small cars vanish at default input sizes; increasing inference resolution was the single biggest accuracy win.
- **Confidence thresholds matter.** Too low and shadows become "cars"; too high and you lose vehicles under trees.
- **Augmentation.** Horizontal flips and rotation helped the model cope with arbitrary camera orientation.

The final system detects cars and logs a count per image — a clean, useful baseline for smart-parking analytics."#,
            24,
        ),
        (
            "handwritten-digit-recognition-with-a-simple-ann",
            "Handwritten Digit Recognition with a Simple ANN",
            "A course project that distils deep learning to its essence: pixels in, probabilities out — built from scratch with a feed-forward neural network.",
            "machine-learning",
            "deep-learning",
            r#"## The assignment

Recognise handwritten digits (0–9) using an artificial neural network. No frameworks at first — just the raw mathematics of forward and backward propagation.

## What I built

- A feed-forward network with an input layer of image pixels, one hidden layer with a non-linear activation, and a softmax output layer over ten classes.
- Training via gradient descent using backpropagation, with a cross-entropy loss.
- A small evaluation harness to report accuracy and confusion on the test set.

## What the project made click

For me the value was never the model — it was peeling back the framework and seeing the math:

1. **Forward pass** — the matrix multiply, the activation, the softmax that turns logits into probabilities.
2. **Backward pass** — the chain rule, applied layer by layer, updating every weight by its contribution to the error.
3. **The loss landscape** — watching training loss fall over epochs made gradient descent concrete in a way no diagram ever did.

## After the basics

Once the from-scratch version worked, I reproduced the same result with a deep-learning library and got a production-quality pipeline in a fraction of the lines — a great way to see exactly what a framework is abstracting away."#,
            18,
        ),
        (
            "solving-equations-with-a-genetic-algorithm",
            "Solving Equation Systems with a Genetic Algorithm",
            "When brute force is too slow and calculus is awkward, evolution can find an answer — a solver for linear and non-linear equation systems built on natural selection.",
            "machine-learning",
            "research",
            r#"## Why a genetic algorithm?

Some systems of equations — especially non-linear ones — are awkward to solve analytically. A genetic algorithm searches the solution space the same way biology searches for fit organisms.

## The recipe

1. **Encoding** — each candidate solution is a chromosome holding the variable values.
2. **Initial population** — random guesses.
3. **Fitness** — how close the candidate comes to satisfying every equation (sum of squared residuals).
4. **Selection** — fitter candidates are more likely to become parents.
5. **Crossover & mutation** — combine and perturb to explore new regions.
6. **Repeat** — until the population converges on a low-error solution.

```python
fitness = 1.0 / (1.0 + total_error)
```

## Getting it to converge

- **Mutation rate** is a dial between exploration and exploitation. Too high and the population never settles; too low and it stalls in local optima.
- **Elitism** — always carry the best individual into the next generation so you never lose your best answer.
- **Equation parsing** — I used symbolic parsing so users can type non-linear equations naturally.

The result is a small, interactive solver that handles both the 2-variable linear case and higher-dimensional non-linear systems."#,
            12,
        ),
        (
            "notes-on-multi-agent-reinforcement-learning",
            "Notes on Multi-Agent Reinforcement Learning",
            "Coordination, emergence and the curse of non-stationarity — my reading notes on getting multiple agents to learn together.",
            "notes",
            "multi-agent-systems",
            r#"## Why multi-agent RL is hard

In single-agent RL the environment is static while the agent learns. With multiple agents, every agent's environment includes the *other agents*, who are also learning. This non-stationarity is the core difficulty.

## The approaches

- **Independent learning** — each agent treats the others as part of the environment. Simple, but fragile: as others change, your "environment" changes under you.
- **Centralised training, decentralised execution** — agents share information during training (e.g. a central critic) but act on local observations at run time. This is the sweet spot for many robotics tasks.
- **Communication** — letting agents exchange messages, learning *what* to communicate alongside *what* to do.

## Relevant to my research

Formation control for mobile robots is naturally a multi-agent problem. Each robot must keep formation while avoiding collisions and handling message loss. Ideas from MARL — centralised critics, reward shaping for coordination — map directly onto that setting.

## Sources I keep coming back to

The classic MARL papers on cooperative value functions and centralised training are short and worth re-reading whenever a coordination problem feels intractable."#,
            6,
        ),
        (
            "why-minimal-sensing-matters-in-formation-control",
            "Why Minimal Sensing Matters in Formation Control",
            "Robots with fewer sensors are cheaper, lighter and more reliable — but controlling them is harder. These are my notes on a research direction I care about.",
            "control-systems",
            "research",
            r#"## The pitch

Much of robot control assumes rich sensing: full state feedback, GPS, reliable communication. But real deployments — swarms, cheap fleets, robots in GNSS-denied environments — often have neither.

## Minimal sensing

"Minimal sensing" means achieving a task with the least possible information. For formation control that might mean:

- No external localisation — only local, relative measurements (e.g. distance and bearing to neighbours).
- No central coordinator — decisions are local and decentralized.
- Bounded communication — or none at all, relying purely on local perception.

## Why it matters

1. **Cost** — fewer and cheaper sensors make fleets affordable.
2. **Resilience** — fewer dependencies mean fewer single points of failure.
3. **Safety** — safe, decentralized behaviour is robust to losing one robot or one link.

## The open question

How do we guarantee *safety* (no collisions, bounded errors) when each robot only sees a slice of the world? That tension between local information and global guarantees is, to me, the most interesting problem in the field — and the one I am currently working on."#,
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
    let draft_body = r#"## Research notes: decentralized formation control

Current working notes on my formation-control research:

- [x] Problem formulation & literature review
- [x] Minimal-sensing setup
- [ ] Safety guarantees under bounded communication
- [ ] Simulations and experiments
- [ ] Write-up"#;
    sqlx::query(
        "INSERT INTO posts (slug, title, excerpt, content_md, content_html, status, author_id, created_at, updated_at)
         VALUES ('decentralized-formation-control-for-mobile-robots', 'Decentralized Formation Control for Mobile Robots',
                 'Working notes on safe, decentralized formation control with minimal sensing.',
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
         VALUES ('from-zero-to-policy-gradient-rl', 'From Zero to Policy Gradient RL',
                 'A beginner-friendly walkthrough of policy-gradient methods in fifty lines of Python.',
                 ?, ?, 'scheduled', ?, ?, ?, ?, ?)",
    )
    .bind(r#"## Coming soon

A short, hands-on introduction to policy-gradient reinforcement learning — with code you can run, not just diagrams."#)
    .bind(markdown::to_html(r#"## Coming soon

A short, hands-on introduction to policy-gradient reinforcement learning — with code you can run, not just diagrams."#))
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
            "iran-game",
            "iran-game",
            "An interactive game built with Next.js and TypeScript, with English/Persian localization and a modular game-engine structure.",
            r#"## About

An early-stage, interactive game built with Next.js (App Router) and TypeScript. The codebase is organised as a small game engine rather than a single page: a game context manages state, dedicated components render the HUD and quest panel, and i18n keeps every string in both English and Persian.

## Highlights

- **Next.js App Router** — component-based pages with server rendering.
- **Modular game engine** — a `features/game` module with context, components and services.
- **Bilingual UI** — English and Persian message catalogs with a locale switcher.
- **Quest system** — data-driven quest/mission definitions with a panel UI.
- **Map layers** — tile-independent rendering for the game map.

## Status

Active development. The structure is in place; gameplay content is being built on top of it."#,
            "https://github.com/amirhosseineslami/iran-game",
            "",
            "#22d3ee",
            r#"["Next.js","TypeScript","React"]"#,
        ),
        (
            "khosrowan-website-rust",
            "personal-site (Rust CMS)",
            "A complete personal website, blog and content-management system written entirely in Rust — one binary, no JavaScript framework, no external services.",
            r#"## The idea

A full-featured personal website platform compiled into a single Rust binary. Templates, CSS, JavaScript and database migrations are built in — copy the executable, run it, done.

## Stack

- **axum** on **tokio** for the web layer.
- **askama** for compile-time, type-safe templates.
- **sqlx** against SQLite (WAL mode).
- **comrak** for Markdown, **ammonia** for HTML sanitisation.

## What it includes

- Public site: home, about, projects, blog, experience, skills, services, testimonials, awards, speaking, publications, gallery, résumé, contact and search.
- Full admin panel for posts, pages, projects, media, users, settings, redirects and more.
- Security: Argon2id hashing, database-backed sessions, CSRF, strict CSP, rate limiting, HTML sanitisation.
- Batteries: RSS, sitemap, Open Graph, JSON-LD, dark/light themes, search, newsletter and e-mail notifications.

This very website runs on it."#,
            "https://github.com/amirhosseineslami/khosrowan-website-rust",
            "",
            "#f74c00",
            r#"["Rust","axum","SQLite"]"#,
        ),
        (
            "truthordarebot",
            "truthordarebot",
            "An anonymous chat-pairing bot for the Rubika messaging platform, written in Python.",
            r#"## What it does

Truthordarebot pairs strangers for anonymous private conversations on the Rubika messaging platform.

## Features

- **Anonymous pairing** — a waiting queue connects users at random.
- **Inline keyboards** — interactive menus for connect, disconnect and cancel-search.
- **Persian-first** — the UI is localised for Farsi.
- **MySQL persistence** — user profiles, coins, scores, levels and state survive restarts.
- **Modular** — separate classes for the API client, parser, keypad and connection handling.

## Notes

Built against the Rubika Bot API using polling with offset-based updates. Configuration is driven by environment variables, and tables are created automatically on first run."#,
            "https://github.com/amirhosseineslami/truthordarebot",
            "",
            "#10b981",
            r#"["Python","MySQL","Bot API"]"#,
        ),
        (
            "CarDetection_yolov8",
            "CarDetection_yolov8",
            "Aerial parking-lot car detection with YOLOv8 that estimates vehicle counts and logs results.",
            r#"## The goal

Detect cars in aerial images of parking areas and estimate how many vehicles are present.

## Approach

- Fine-tuned a YOLOv8 detector on aerial parking imagery.
- Run inference over new aerial frames.
- Aggregate detections into a vehicle count and log it as a result.

## Lessons

Small, dense, partially occluded cars are a classic hard case. Boosting inference resolution and careful confidence thresholds made the biggest difference.

A clean, useful baseline for smart-parking analytics."#,
            "https://github.com/amirhosseineslami/CarDetection_yolov8",
            "",
            "#ef4444",
            r#"["Python","YOLOv8","Jupyter"]"#,
        ),
        (
            "ANN-Handwritten-Digit-Recognition",
            "ANN-Handwritten-Digit-Recognition",
            "Handwritten digit recognition using a feed-forward artificial neural network, developed for an AI course.",
            r#"## About

A course project on handwritten digit recognition. The core is a feed-forward ANN — pixels in, class probabilities out — trained with backpropagation.

I first built the forward and backward passes from scratch to see the mathematics clearly, then reproduced the result with a deep-learning framework to appreciate what it abstracts away."#,
            "https://github.com/amirhosseineslami/ANN-Handwritten-Digit-Recognition",
            "",
            "#6366f1",
            r#"["Python","NumPy"]"#,
        ),
        (
            "Othello-AI-MiniMax",
            "Othello-AI-MiniMax",
            "Othello / Reversi in Python with an AI opponent powered by the MiniMax algorithm.",
            r#"## About

A full Othello (Reversi) game in Python featuring:

- A **MiniMax** AI opponent with alpha-beta pruning.
- A **pygame** graphical interface.
- A standalone Windows executable build.

A great exercise in game-tree search and evaluation-function design."#,
            "https://github.com/amirhosseineslami/Othello-AI-MiniMax",
            "",
            "#8b5cf6",
            r#"["Python","pygame"]"#,
        ),
        (
            "RL_Crawling_Robot",
            "RL_Crawling_Robot",
            "A worm-like crawling robot trained with reinforcement learning using Arduino and servo motors.",
            r#"## About

A physical, worm-like robot made from Arduino-driven servos that learns to crawl using reinforcement learning.

The project combines embedded programming with a learning-based controller — rewarding forward displacement while penalising harsh joint motion until a smooth gait emerges.

A hands-on lesson in reward shaping, physical constraints and sample efficiency on real hardware."#,
            "https://github.com/amirhosseineslami/RL_Crawling_Robot",
            "",
            "#f59e0b",
            r#"["C++","Arduino"]"#,
        ),
        (
            "genetic-algorithm-solver",
            "genetic-algorithm-solver",
            "A genetic algorithm solver for systems of linear and non-linear equations.",
            r#"## About

A Python equation solver that uses a genetic algorithm to find approximate solutions for systems of linear and non-linear equations.

- Solves 2-variable linear systems and higher-dimensional non-linear systems.
- Interactive, menu-driven interface with symbolic equation parsing.
- Uses natural selection, crossover and mutation to converge on low-error solutions.

Comes with English and Persian reports on the method."#,
            "https://github.com/amirhosseineslami/genetic-algorithm-solver",
            "",
            "#14b8a6",
            r#"["Python","sympy"]"#,
        ),
        (
            "Agricultural-Web-Scraper",
            "Agricultural-Web-Scraper",
            "An asynchronous web scraper for e-commerce and agricultural sources, developed for Lotus Futurist.",
            r#"## About

A web scraping tool developed for Lotus Futurist to extract structured data from e-commerce and agricultural websites.

- **Asynchronous** scraping with `asyncio` and Playwright.
- **Multi-site** support (Digikala, Basalam, Agriplus, Torob and more), each with its own module.
- **Persian number normalization** and data-cleaning utilities.
- Price and unit extraction, with results organised into CSV/Excel outputs and archives."#,
            "https://github.com/amirhosseineslami/Agricultural-Web-Scraper",
            "",
            "#0ea5e9",
            r#"["Python","Playwright"]"#,
        ),
        (
            "student-depression-detection",
            "student-depression-detection",
            "Detecting student depression risk using decision trees and data preprocessing in Python.",
            r#"## About

A data-science project that classifies student depression risk.

- Decision-tree based classification.
- Emphasis on cleaning and preprocessing real-world survey data before modelling.

A practical exercise in applying classic ML to a socially meaningful problem."#,
            "https://github.com/amirhosseineslami/student-depression-detection",
            "",
            "#ec4899",
            r#"["Python","scikit-learn"]"#,
        ),
        (
            "SVM-Kernel-Analysis",
            "SVM-Kernel-Analysis",
            "Comparing support-vector-machine kernels on standard classification datasets.",
            r#"## About

An analysis notebook comparing SVM kernels — linear, polynomial and RBF — on standard datasets, examining how kernel choice affects decision boundaries and accuracy."#,
            "https://github.com/amirhosseineslami/SVM-Kernel-Analysis",
            "",
            "#22d3ee",
            r#"["Python","scikit-learn"]"#,
        ),
        (
            "Currency-Convertor-Via-MCP",
            "Currency-Convertor-Via-MCP",
            "A currency converter driven through the Model Context Protocol.",
            r#"## About

A currency-conversion tool exercised through the Model Context Protocol (MCP) — an experiment in exposing a simple utility as an MCP-capable service."#,
            "https://github.com/amirhosseineslami/Currency-Convertor-Via-MCP",
            "",
            "#10b981",
            r#"["Python","MCP"]"#,
        ),
    ];

    for (idx, (slug, title, summary, body, repo, live, color, tech)) in
        projects.iter().enumerate()
    {
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
        .bind(*tech)
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
        ("C", "Languages", 85, 4.0),
        ("C++", "Languages", 88, 5.0),
        ("Python", "Languages", 92, 6.0),
        ("MATLAB", "Languages", 80, 3.0),
        ("Java", "Languages", 72, 3.0),
        ("Kotlin", "Languages", 70, 2.0),
        ("JavaScript / TypeScript", "Languages", 65, 2.0),
        ("PyTorch", "Machine Learning", 82, 3.0),
        ("TensorFlow", "Machine Learning", 78, 3.0),
        ("Scikit-learn", "Machine Learning", 84, 4.0),
        ("Pandas & NumPy", "Machine Learning", 86, 5.0),
        ("Reinforcement Learning", "Machine Learning", 80, 3.0),
        ("Deep Learning", "Machine Learning", 78, 3.0),
        ("OpenCV", "Computer Vision", 85, 5.0),
        ("YOLO & object detection", "Computer Vision", 78, 3.0),
        ("Image processing", "Computer Vision", 82, 4.0),
        ("Arduino", "Robotics & Embedded", 86, 5.0),
        ("C++ firmware", "Robotics & Embedded", 80, 4.0),
        ("ROS", "Robotics & Embedded", 68, 2.0),
        ("Motor / servo control", "Robotics & Embedded", 78, 4.0),
        ("Linux", "Tools", 88, 6.0),
        ("Git & GitHub", "Tools", 90, 6.0),
        ("Docker", "Tools", 65, 2.0),
        ("Jupyter", "Tools", 85, 5.0),
        ("VS Code", "Tools", 88, 6.0),
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
            "Research & Data Assistant",
            "Lotus Futurists",
            "Tehran, Iran · Remote",
            "2024",
            None,
            true,
            "Supporting data research: building asynchronous scraping pipelines, cleaning datasets and extracting structured data from e-commerce and agricultural sources.",
            "work",
        ),
        (
            "Computer Vision Research Intern",
            "CVLab, Iran University of Science and Technology",
            "Tehran, Iran",
            "2023",
            Some("2024"),
            false,
            "Worked on computer-vision research experiments, dataset preparation and model prototyping in an academic lab setting.",
            "work",
        ),
        (
            "Teaching Assistant",
            "Artificial Intelligence · Industrial Control Lab · Programming",
            "Iran University of Science and Technology",
            "2023",
            None,
            true,
            "Assisting courses in artificial intelligence, the industrial control laboratory and programming — running lab sessions, reviewing assignments and helping students debug.",
            "work",
        ),
        (
            "Founder & Host",
            "EnglishConvey",
            "Online",
            "2022",
            None,
            true,
            "Founded and hosted an English-speaking initiative that helps peers practice conversation in a friendly, structured setting.",
            "work",
        ),
        (
            "B.Sc. Electrical Engineering (Control)",
            "Iran University of Science and Technology",
            "Tehran, Iran",
            "2021",
            None,
            true,
            "Focus on control systems, robotics, signal processing and AI/ML. Research interests include intelligent control, multi-agent systems and autonomous robotics.",
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
            "Programming & Maths Tutoring",
            "One-on-one tutoring in Python, C/C++, MATLAB and engineering mathematics — for students who want to go deeper than the syllabus.",
            "code",
            "on request",
            r#"["Python / C++ / MATLAB","Engineering mathematics","Sessions for students at any level"]"#,
        ),
        (
            "Computer Vision & ML Help",
            "Hands-on help with computer-vision and machine-learning projects: from data preparation to training, evaluating and deploying models.",
            "vision",
            "on request",
            r#"["Data prep & labelling","Model training & evaluation","PyTorch / TensorFlow / OpenCV"]"#,
        ),
        (
            "Data Collection & Web Scraping",
            "Asynchronous scraping pipelines and dataset cleaning for research and small-business needs.",
            "data",
            "on request",
            r#"["Playwright / asyncio pipelines","Structured CSV & Excel output","Data cleaning & normalization"]"#,
        ),
        (
            "Control Systems & Arduino Projects",
            "Guidance on control theory, embedded programming and microcontroller builds with Arduino.",
            "chip",
            "on request",
            r#"["Control theory fundamentals","Arduino & servo projects","Simulation in MATLAB/Simulink"]"#,
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

    // Testimonials — intentionally left empty. Add genuine quotes via the admin panel.
    let testimonials: [(&str, &str, &str, &str, i64, bool); 0] = [];
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
            "Pull Shark",
            "GitHub",
            "2025",
            "Awarded for merging pull requests — building software in public, one PR at a time.",
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

    // Talks — intentionally left empty.
    let talks: [(&str, &str, &str, &str, Option<&str>, Option<&str>); 0] = [];
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

    // Publications — intentionally left empty.
    let publications: [(&str, &str, &str, &str, &str); 0] = [];
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

    // Gallery — intentionally left empty until the owner adds photos.
    let gallery: [(&str, &str, &str); 0] = [];
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
        ("github", "https://github.com/amirhosseineslami", "GitHub"),
        (
            "linkedin",
            "https://www.linkedin.com/in/amir-hossein-eslami",
            "LinkedIn",
        ),
        ("website", "https://about.me/amirhosseineslami", "Portfolio"),
        ("orcid", "https://orcid.org/0009-0000-5547-1764", "ORCID"),
        (
            "email",
            "mailto:amirhossein.eslami.ac@gmail.com",
            "Email",
        ),
        ("rss", "/feed.xml", "RSS Feed"),
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

I am an Electrical Engineering (Control) student at Iran University of Science and Technology (IUST). I care about systems that can sense, learn, adapt and collaborate — which is why my interests sit at the intersection of robotics, artificial intelligence, control systems and computer vision.

I am currently working on decentralized and safe formation control for differential-drive mobile robots with minimal sensing: how a fleet of cheap robots can hold formation and stay safe using only local, relative measurements and no central coordinator.

## What I work on

* **Robotics & control** — formation control, feed and embedded control, and getting physical systems to behave.
* **Machine learning & deep learning** — from classic classifiers to neural networks and reinforcement learning.
* **Computer vision** — detection and counting with YOLOv8, image processing and open-source toolchains.
* **Data engineering** — asynchronous scraping pipelines, dataset cleaning and turning messy data into answers.

## Experience worth mentioning

* Research & Data Assistant at **Lotus Futurists** — building scraping pipelines and cleaning data.
* Computer Vision Research Intern at **CVLab, IUST**.
* Teaching Assistant for Artificial Intelligence, the Industrial Control Lab and Programming.
* Founder & Host of **EnglishConvey**, an initiative to help peers practice English.

## How I work

I write things down. Research notes, experiment logs and working papers — enough that any idea can be resumed months later. I believe the best engineering is boring, measured and documented.

## Outside the lab

I read broadly, enjoy working in the open, and I am always happy to talk about robotics, RL or a tricky control problem."#,
            false,
            8,
        ),
        (
            "uses",
            "Uses",
            r#"## Languages & tools

The tools I reach for most, in rough order of how much I live in them.

* **Python** — my main language for ML, data and scripting.
* **C / C++** — embedded work (Arduino) and performance-sensitive code.
* **MATLAB** — control-theory work and simulations.
* **Kotlin / Java** — Android and JVM projects.

## Machine learning & data

* **PyTorch** and **TensorFlow** — deep learning.
* **scikit-learn**, **pandas**, **NumPy** — classic ML and data wrangling.
* **Jupyter** — exploration and write-ups.
* **OpenCV** — computer vision.

## Robotics & embedded

* **Arduino** — microcontrollers, servos and sensors.
* **ROS** — robotics middleware (getting deeper into it).
* Git, GitHub and Linux — the substrate for all of the above.

## The setup

I keep things simple: a Linux environment, a solid editor, and the terminal for everything else. The goal is that the tools stay out of the way of the work."#,
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
