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
    ("site.title", "خسروان", "text", "general"),
    ("site.title_en", "Khosrowan", "text", "general"),
    ("site.logo_text", "خ", "text", "general"),
    ("site.logo_text_en", "K", "text", "general"),
    (
        "site.subtitle",
        "مهندسی برق و کنترل · هوش مصنوعی و رباتیک",
        "text",
        "general",
    ),
    (
        "site.subtitle_en",
        "Electrical & Control Engineering · AI & Robotics",
        "text",
        "general",
    ),
    (
        "site.description",
        "دانشجوی مهندسی برق (گرایش کنترل) در دانشگاه علم و صنعت ایران؛ متمرکز بر رباتیک، هوش مصنوعی، سیستم‌های کنترل، بینایی ماشین و راه‌حل‌های هوشمند مهندسی. پژوهشگر کنترل غیرمتمرکز و ایمن آرایش ربات‌های متحرک.",
        "textarea",
        "general",
    ),
    (
        "site.tagline",
        "آینده متعلق به سیستم‌هایی است که می‌توانند حس کنند، یاد بگیرند، سازگار شوند و همکاری کنند.",
        "text",
        "general",
    ),
    (
        "site.hero_headline",
        "پژوهش در رباتیک و سیستم‌های هوشمند.",
        "text",
        "general",
    ),
    (
        "site.hero_headline_en",
        "Research in robotics & intelligent systems.",
        "text",
        "general",
    ),
    (
        "site.hero_subheadline",
        "اندازه‌گیری، تنظیم، تکرار — درباره سیستم‌هایی که می‌توانند حس کنند و یاد بگیرند.",
        "textarea",
        "general",
    ),
    (
        "site.hero_subheadline_en",
        "Measure, adjust, repeat — about systems that can sense, learn, adapt and collaborate.",
        "textarea",
        "general",
    ),
    ("site.email", "amirhossein.eslami.ac@gmail.com", "text", "general"),
    ("site.phone", "", "text", "general"),
    (
        "site.location",
        "تهران، ایران · دانشگاه علم و صنعت ایران",
        "text",
        "general",
    ),
    ("site.available", "true", "bool", "general"),
    (
        "site.availability_note",
        "آماده همکاری‌های پژوهشی، کارآموزی و منتورینگ",
        "text",
        "general",
    ),
    ("site.favicon", "/assets/img/favicon.svg", "text", "general"),
    ("site.default_og_image", "/assets/img/og-default.svg", "text", "general"),
    ("site.cv_file", "/assets/files/cv.pdf", "text", "general"),
    ("site.theme_accent", "#6366f1", "color", "appearance"),
    ("site.theme_accent_2", "#22d3ee", "color", "appearance"),
    ("site.default_theme", "dark", "text", "appearance"),
    ("site.enable_dark_mode", "true", "bool", "appearance"),
    ("site.default_locale", "fa", "text", "appearance"),
    (
        "site.location_en",
        "Tehran, Iran · Iran University of Science and Technology",
        "text",
        "general",
    ),
    (
        "site.description_en",
        "Electrical Engineering (Control) student at Iran University of Science and Technology, focused on robotics, AI/ML, control systems, computer vision and intelligent engineering solutions. Researching decentralized and safe formation control for mobile robots.",
        "textarea",
        "general",
    ),
    (
        "site.tagline_en",
        "The future belongs to systems that can sense, learn, adapt, and collaborate.",
        "text",
        "general",
    ),
    (
        "site.availability_note_en",
        "Open to research collaborations, internships and mentorship.",
        "text",
        "general",
    ),
    (
        "newsletter.heading_en",
        "Subscribe for deep dives on AI, robotics and control engineering.",
        "text",
        "newsletter",
    ),
    (
        "newsletter.subheading_en",
        "One thoughtful email every couple of weeks. No spam, ever.",
        "text",
        "newsletter",
    ),
    (
        "contact.success_message_en",
        "Thanks! Your message has been delivered — I'll get back to you soon.",
        "textarea",
        "contact",
    ),
    ("site.posts_per_page", "9", "number", "content"),
    ("site.enable_comments", "true", "bool", "content"),
    ("site.enable_newsletter", "true", "bool", "content"),
    ("site.allow_registration", "false", "bool", "content"),
    (
        "seo.meta_description",
        "وب‌سایت شخصی امیرحسین اسلامی — دانشجوی مهندسی برق و کنترل در دانشگاه علم و صنعت ایران؛ فعال در حوزه رباتیک، هوش مصنوعی، بینایی ماشین و کنترل هوشمند.",
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
    ("seo.person_name", "امیرحسین اسلامی", "text", "seo"),
    ("seo.person_name_en", "Amirhossein Eslami", "text", "seo"),
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
        "متشکریم! پیام شما ارسال شد — به‌زودی پاسخ می‌دهم.",
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
        "برای دریافت نوشته‌های عمیق درباره هوش مصنوعی، رباتیک و مهندسی کنترل عضو شوید.",
        "text",
        "newsletter",
    ),
    (
        "newsletter.subheading",
        "هر چند هفته یک ایمیل مفید. بدون اسپم، همیشه.",
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
         VALUES (?, ?, ?, 'owner', 'خسروان', ?, ?, ?) RETURNING id",
    )
    .bind(&config.admin_username)
    .bind(&config.admin_email)
    .bind(&password_hash)
    .bind("دانشجوی مهندسی برق (گرایش کنترل) در دانشگاه علم و صنعت ایران (IUST)؛ علاقه‌مند به رباتیک، هوش مصنوعی، سیستم‌های کنترل، بینایی ماشین، علم داده و راه‌حل‌های هوشمند مهندسی.")
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    seed_settings(pool).await?;
    seed_taxonomy(pool).await?;
    seed_posts(pool, user_id).await?;
    seed_projects(pool).await?;
    seed_profile_sections(pool).await?;
    seed_translations(pool).await?;

    Ok(())
}

/// Seed English translations for the base (Persian) content. This is what makes
/// the site truly bilingual: each post/page/project has an `en` variant.
async fn seed_translations(pool: &SqlitePool) -> Result<()> {
    seed_post_translations(pool).await?;
    seed_page_translations(pool).await?;
    seed_project_translations(pool).await?;
    seed_profile_translations(pool).await?;
    seed_taxonomy_translations(pool).await?;
    Ok(())
}

async fn seed_profile_translations(pool: &SqlitePool) -> Result<()> {
    // Experiences: [kind, title_persian, company_persian, title_en, company_en, location_en, description_en]
    let experiences: Vec<(&str, &str, &str, &str, &str, &str, &str)> = vec![
        (
            "work",
            "دستیار پژوهش و داده",
            "Lotus Futurists",
            "Research & Data Assistant",
            "Lotus Futurists",
            "Tehran, Iran · Remote",
            "Supporting data research: building asynchronous scraping pipelines, cleaning datasets and extracting structured data from e-commerce and agricultural sources.",
        ),
        (
            "work",
            "کارآموز پژوهشی بینایی ماشین",
            "CVLab، دانشگاه علم و صنعت ایران",
            "Computer Vision Research Intern",
            "CVLab, Iran University of Science and Technology",
            "Tehran, Iran",
            "Worked on computer-vision research experiments, dataset preparation and model prototyping in an academic lab setting.",
        ),
        (
            "work",
            "دستیار آموزشی",
            "هوش مصنوعی · آزمایشگاه کنترل صنعتی · برنامه‌نویسی",
            "Teaching Assistant",
            "Artificial Intelligence · Industrial Control Lab · Programming",
            "Iran University of Science and Technology",
            "Assisting courses in artificial intelligence, the industrial control laboratory and programming — running lab sessions, reviewing assignments and helping students debug.",
        ),
        (
            "work",
            "بنیان‌گذار و میزبان",
            "EnglishConvey",
            "Founder & Host",
            "EnglishConvey",
            "Online",
            "Founded and hosted an English-speaking initiative that helps peers practice conversation in a friendly, structured setting.",
        ),
        (
            "education",
            "کارشناسی مهندسی برق (کنترل)",
            "دانشگاه علم و صنعت ایران",
            "B.Sc. Electrical Engineering (Control)",
            "Iran University of Science and Technology",
            "Tehran, Iran",
            "Focus on control systems, robotics, signal processing and AI/ML. Research interests include intelligent control, multi-agent systems and autonomous robotics.",
        ),
    ];
    for (kind, fa_title, fa_company, en_title, en_company, en_loc, en_desc) in experiences {
        let id = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM experiences WHERE title = ? AND company = ? AND kind = ?",
        )
        .bind(fa_title)
        .bind(fa_company)
        .bind(kind)
        .fetch_optional(pool)
        .await?;
        if let Some(id) = id {
            sqlx::query(
                "INSERT OR REPLACE INTO experience_translations
                     (experience_id, locale, title, company, location, description)
                 VALUES (?, 'en', ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(en_title)
            .bind(en_company)
            .bind(en_loc)
            .bind(en_desc)
            .execute(pool)
            .await?;
        }
    }

    // Services: [fa_title, en_title, en_desc, en_features, en_price]
    let services: Vec<(&str, &str, &str, &str, &str)> = vec![
        (
            "آموزش خصوصی برنامه‌نویسی و ریاضیات",
            "Programming & Maths Tutoring",
            "One-on-one tutoring in Python, C/C++, MATLAB and engineering mathematics — for students who want to go deeper than the syllabus.",
            r#"["Python / C++ / MATLAB","Engineering mathematics","Sessions for students at any level"]"#,
            "on request",
        ),
        (
            "کمک در بینایی ماشین و یادگیری ماشین",
            "Computer Vision & ML Help",
            "Hands-on help with computer-vision and machine-learning projects: from data preparation to training, evaluating and deploying models.",
            r#"["Data prep & labelling","Model training & evaluation","PyTorch / TensorFlow / OpenCV"]"#,
            "on request",
        ),
        (
            "جمع‌آوری داده و خراش وب",
            "Data Collection & Web Scraping",
            "Asynchronous scraping pipelines and dataset cleaning for research and small-business needs.",
            r#"["Playwright / asyncio pipelines","Structured CSV & Excel output","Data cleaning & normalization"]"#,
            "on request",
        ),
        (
            "پروژه‌های سیستم کنترل و آردوینو",
            "Control Systems & Arduino Projects",
            "Guidance on control theory, embedded programming and microcontroller builds with Arduino.",
            r#"["Control theory fundamentals","Arduino & servo projects","Simulation in MATLAB/Simulink"]"#,
            "on request",
        ),
    ];
    for (fa_title, en_title, en_desc, en_features, en_price) in services {
        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM services WHERE title = ?")
            .bind(fa_title)
            .fetch_optional(pool)
            .await?;
        if let Some(id) = id {
            sqlx::query(
                "INSERT OR REPLACE INTO service_translations
                     (service_id, locale, title, description, features, price_from)
                 VALUES (?, 'en', ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(en_title)
            .bind(en_desc)
            .bind(en_features)
            .bind(en_price)
            .execute(pool)
            .await?;
        }
    }

    // Skills: [fa_name, en_name, en_category]
    let skills: Vec<(&str, &str, &str)> = vec![
        ("C", "C", "Languages"),
        ("C++", "C++", "Languages"),
        ("Python", "Python", "Languages"),
        ("MATLAB", "MATLAB", "Languages"),
        ("Java", "Java", "Languages"),
        ("Kotlin", "Kotlin", "Languages"),
        ("JavaScript / TypeScript", "JavaScript / TypeScript", "Languages"),
        ("PyTorch", "PyTorch", "Machine Learning"),
        ("TensorFlow", "TensorFlow", "Machine Learning"),
        ("Scikit-learn", "Scikit-learn", "Machine Learning"),
        ("Pandas & NumPy", "Pandas & NumPy", "Machine Learning"),
        ("یادگیری تقویتی", "Reinforcement Learning", "Machine Learning"),
        ("یادگیری عمیق", "Deep Learning", "Machine Learning"),
        ("OpenCV", "OpenCV", "Computer Vision"),
        ("YOLO و تشخیص شیء", "YOLO & object detection", "Computer Vision"),
        ("پردازش تصویر", "Image processing", "Computer Vision"),
        ("آردوینو", "Arduino", "Robotics & Embedded"),
        ("سیستم‌عامل C++", "C++ firmware", "Robotics & Embedded"),
        ("ROS", "ROS", "Robotics & Embedded"),
        ("کنترل موتور / سروو", "Motor / servo control", "Robotics & Embedded"),
        ("لینوکس", "Linux", "Tools"),
        ("Git و GitHub", "Git & GitHub", "Tools"),
        ("Docker", "Docker", "Tools"),
        ("Jupyter", "Jupyter", "Tools"),
        ("VS Code", "VS Code", "Tools"),
    ];
    for (fa_name, en_name, en_cat) in skills {
        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM skills WHERE name = ?")
            .bind(fa_name)
            .fetch_optional(pool)
            .await?;
        if let Some(id) = id {
            sqlx::query(
                "INSERT OR REPLACE INTO skill_translations (skill_id, locale, name, category)
                 VALUES (?, 'en', ?, ?)",
            )
            .bind(id)
            .bind(en_name)
            .bind(en_cat)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn seed_taxonomy_translations(pool: &SqlitePool) -> Result<()> {
    // Categories: [fa, en, en_desc]
    let categories: Vec<(&str, &str, &str)> = vec![
        ("یادگیری ماشین", "Machine Learning", "Models, algorithms and the mathematics behind learning from data."),
        ("رباتیک", "Robotics", "Robots, embedded systems and the software that makes them move."),
        ("بینایی ماشین", "Computer Vision", "Teaching machines to see: detection, segmentation and beyond."),
        ("سیستم‌های کنترل", "Control Systems", "Feedback, dynamics and making physical systems behave."),
        ("یادداشت‌ها", "Notes", "Short, opinionated notes."),
        ("آموزش", "Tutorial", "Step-by-step walkthroughs."),
        ("ایده‌ها و فرهنگ", "Ideas & Culture", "Essays on the ideas behind engineering: history, philosophy and mathematics."),
    ];
    for (fa_name, en_name, en_desc) in categories {
        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM categories WHERE name = ?")
            .bind(fa_name)
            .fetch_optional(pool)
            .await?;
        if let Some(id) = id {
            sqlx::query(
                "INSERT OR REPLACE INTO category_translations (category_id, locale, name, description)
                 VALUES (?, 'en', ?, ?)",
            )
            .bind(id)
            .bind(en_name)
            .bind(en_desc)
            .execute(pool)
            .await?;
        }
    }

    // Tags: [fa, en]
    let tags: Vec<(&str, &str)> = vec![
        ("یادگیری تقویتی", "Reinforcement Learning"),
        ("سیستم‌های چندعاملی", "Multi-Agent Systems"),
        ("یادگیری عمیق", "Deep Learning"),
        ("بینایی ماشین", "Computer Vision"),
        ("پایتون", "Python"),
        ("آردوینو", "Arduino"),
        ("علم داده", "Data Science"),
        ("پژوهش", "Research"),
        ("منبع‌باز", "Open Source"),
        ("فلسفه", "Philosophy"),
        ("تاریخ", "History"),
        ("ریاضیات", "Mathematics"),
    ];
    for (fa_name, en_name) in tags {
        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM tags WHERE name = ?")
            .bind(fa_name)
            .fetch_optional(pool)
            .await?;
        if let Some(id) = id {
            sqlx::query(
                "INSERT OR REPLACE INTO tag_translations (tag_id, locale, name)
                 VALUES (?, 'en', ?)",
            )
            .bind(id)
            .bind(en_name)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn seed_post_translations(pool: &SqlitePool) -> Result<()> {
    let entries: Vec<(&str, &str, &str, &str)> = vec![
        (
            "building-a-worm-like-crawling-robot-with-rl",
            "Building a Worm-like Crawling Robot with Reinforcement Learning",
            "How I taught an Arduino-powered servo worm to crawl using reinforcement learning — and what the project taught me about reward shaping, sim-to-real gaps and patience.",
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
        ),
        (
            "detecting-cars-in-aerial-images-with-yolov8",
            "Detecting Cars in Aerial Parking Imagery with YOLOv8",
            "Training a YOLOv8 model to count vehicles in aerial parking-lot images — from data labelling to a working detection pipeline.",
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
        ),
        (
            "handwritten-digit-recognition-with-a-simple-ann",
            "Handwritten Digit Recognition with a Simple ANN",
            "A course project that distils deep learning to its essence: pixels in, probabilities out — built from scratch with a feed-forward neural network.",
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
        ),
        (
            "solving-equations-with-a-genetic-algorithm",
            "Solving Equation Systems with a Genetic Algorithm",
            "When brute force is too slow and calculus is awkward, evolution can find an answer — a solver for linear and non-linear equation systems built on natural selection.",
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
        ),
        (
            "notes-on-multi-agent-reinforcement-learning",
            "Notes on Multi-Agent Reinforcement Learning",
            "Coordination, emergence and the curse of non-stationarity — my reading notes on getting multiple agents to learn together.",
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
        ),
        (
            "what-pid-control-can-teach-us-about-life",
            "What a PID Controller Can Teach Us About Life and Science",
            "Feedback is everywhere — in machines, economies and the way we learn. A short essay on control theory as a lens for understanding the world.",
            r#"## Control theory as a worldview

A PID controller is one of the oldest and most widespread ideas in engineering. It looks at the difference between where you are and where you want to be (the *error*), then reacts proportionally, integrally and derivatively.

### The lesson

- **Proportional** — respond to the size of the gap. Too little, and you drift; too much, and you overshoot.
- **Integral** — remember the past and keep pushing until long-standing errors disappear.
- **Derivative** — anticipate the future and dampen wild swings.

Most of life follows the same logic. Learning, science, even personal growth are feedback loops: measure, adjust, repeat. The best systems — biological, social, technical — are the ones that keep their feedback loops healthy.

## Why I keep this close

I came to control theory through engineering, but I find it is also a philosophy. It reminds me that the world is dynamic, that measurement matters, and that stable systems are not static ones. Understanding feedback is, in a sense, understanding how things *change* — which is why I think it belongs in the same mental bucket as history and mathematics."#,
        ),
        (
            "why-minimal-sensing-matters-in-formation-control",
            "Why Minimal Sensing Matters in Formation Control",
            "Robots with fewer sensors are cheaper, lighter and more reliable — but controlling them is harder. These are my notes on a research direction I care about.",
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
        ),
    ];

    for (slug, title, excerpt, body) in entries {
        let post_id = sqlx::query_scalar::<_, i64>("SELECT id FROM posts WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await?;
        if let Some(post_id) = post_id {
            let html = crate::markdown::to_html(body);
            sqlx::query(
                "INSERT OR REPLACE INTO post_translations
                     (post_id, locale, title, excerpt, content_md, content_html)
                 VALUES (?, 'en', ?, ?, ?, ?)",
            )
            .bind(post_id)
            .bind(title)
            .bind(excerpt)
            .bind(body)
            .bind(&html)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn seed_page_translations(pool: &SqlitePool) -> Result<()> {
    let entries: Vec<(&str, &str, &str)> = vec![
        (
            "about",
            "About",
            r#"## The short version

I am **Amirhossein Eslami** (publishing on this site as **Khosrowan**), an Electrical Engineering (Control) student at Iran University of Science and Technology (IUST). I care about systems that can sense, learn, adapt and collaborate — which is why my interests sit at the intersection of robotics, artificial intelligence, control systems and computer vision. This site is where I publish research notes, technical blog posts and engineering experiments.

I am currently working on decentralized and safe formation control for differential-drive mobile robots with minimal sensing: how a fleet of cheap robots can hold formation and stay safe using only local, relative measurements and no central coordinator.

## What I work on

* **Robotics & control** — formation control, embedded control, and getting physical systems to behave.
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

Beyond robotics and control, I read history and philosophy and keep a soft spot for pure mathematics. I find that the great engineers were often great thinkers too — understanding the history of an idea makes it easier to push it forward. I am always happy to talk about robotics, RL, a tricky control problem, or a good book."#,
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
        ),
    ];

    for (slug, title, body) in entries {
        let page_id = sqlx::query_scalar::<_, i64>("SELECT id FROM pages WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await?;
        if let Some(page_id) = page_id {
            let html = crate::markdown::to_html(body);
            sqlx::query(
                "INSERT OR REPLACE INTO page_translations
                     (page_id, locale, title, content_md, content_html)
                 VALUES (?, 'en', ?, ?, ?)",
            )
            .bind(page_id)
            .bind(title)
            .bind(body)
            .bind(&html)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn seed_project_translations(pool: &SqlitePool) -> Result<()> {
    let entries: Vec<(&str, &str, &str, &str)> = vec![
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
        ),
        (
            "ANN-Handwritten-Digit-Recognition",
            "ANN-Handwritten-Digit-Recognition",
            "Handwritten digit recognition using a feed-forward artificial neural network, developed for an AI course.",
            r#"## About

A course project on handwritten digit recognition. The core is a feed-forward ANN — pixels in, class probabilities out — trained with backpropagation.

I first built the forward and backward passes from scratch to see the mathematics clearly, then reproduced the result with a deep-learning framework to appreciate what it abstracts away."#,
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
        ),
        (
            "RL_Crawling_Robot",
            "RL_Crawling_Robot",
            "A worm-like crawling robot trained with reinforcement learning using Arduino and servo motors.",
            r#"## About

A physical, worm-like robot made from Arduino-driven servos that learns to crawl using reinforcement learning.

The project combines embedded programming with a learning-based controller — rewarding forward displacement while penalising harsh joint motion until a smooth gait emerges.

A hands-on lesson in reward shaping, physical constraints and sample efficiency on real hardware."#,
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
        ),
        (
            "SVM-Kernel-Analysis",
            "SVM-Kernel-Analysis",
            "Comparing support-vector-machine kernels on standard classification datasets.",
            r#"## About

An analysis notebook comparing SVM kernels — linear, polynomial and RBF — on standard datasets, examining how kernel choice affects decision boundaries and accuracy."#,
        ),
        (
            "Currency-Convertor-Via-MCP",
            "Currency-Convertor-Via-MCP",
            "A currency converter driven through the Model Context Protocol.",
            r#"## About

A currency-conversion tool exercised through the Model Context Protocol (MCP) — an experiment in exposing a simple utility as an MCP-capable service."#,
        ),
    ];

    for (slug, title, summary, body) in entries {
        let project_id = sqlx::query_scalar::<_, i64>("SELECT id FROM projects WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await?;
        if let Some(project_id) = project_id {
            let html = crate::markdown::to_html(body);
            sqlx::query(
                "INSERT OR REPLACE INTO project_translations
                     (project_id, locale, title, summary, description_md, description_html)
                 VALUES (?, 'en', ?, ?, ?, ?)",
            )
            .bind(project_id)
            .bind(title)
            .bind(summary)
            .bind(body)
            .bind(&html)
            .execute(pool)
            .await?;
        }
    }
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
            "یادگیری ماشین",
            "machine-learning",
            "مدل‌ها، الگوریتم‌ها و ریاضیات یادگیری از داده.",
        ),
        (
            "رباتیک",
            "robotics",
            "ربات‌ها، سیستم‌های نهفته و نرم‌افزاری که آن‌ها را به حرکت درمی‌آورد.",
        ),
        (
            "بینایی ماشین",
            "computer-vision",
            "آموزش دیدن به ماشین‌ها: تشخیص، بخش‌بندی و فراتر از آن.",
        ),
        (
            "سیستم‌های کنترل",
            "control-systems",
            "بازخورد، دینامیک و واداشتن سیستم‌های فیزیکی به رفتار درست.",
        ),
        ("یادداشت‌ها", "notes", "یادداشت‌های کوتاه و گویا."),
        ("آموزش", "tutorial", "راهنمای گام‌به‌گام."),
        (
            "ایده‌ها و فرهنگ",
            "ideas-culture",
            "جستارهایی درباره ایده‌های پشت مهندسی: تاریخ، فلسفه و ریاضیات.",
        ),
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
        ("یادگیری تقویتی", "reinforcement-learning", "#f59e0b"),
        ("سیستم‌های چندعاملی", "multi-agent-systems", "#8b5cf6"),
        ("یادگیری عمیق", "deep-learning", "#6366f1"),
        ("بینایی ماشین", "computer-vision", "#22d3ee"),
        ("پایتون", "python", "#10b981"),
        ("آردوینو", "arduino", "#0ea5e9"),
        ("علم داده", "data-science", "#14b8a6"),
        ("پژوهش", "research", "#ec4899"),
        ("منبع‌باز", "open-source", "#22d3ee"),
        ("YOLO", "yolo", "#ef4444"),
        ("فلسفه", "philosophy", "#a855f7"),
        ("تاریخ", "history", "#d97706"),
        ("ریاضیات", "mathematics", "#0d9488"),
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
            "ساخت ربات کرمی‌شکل با یادگیری تقویتی",
            "چطور با یادگیری تقویتی یک ربات کرمی ساخته‌شده با آردوینو و سروو را به خزیدن آموزش دادم — و این پروژه درباره شکل‌دهی پاداش، شکاف شبیه‌سازی تا واقعیت و صبوری چه چیزهایی به من آموخت.",
            "robotics",
            "reinforcement-learning",
            r#"## رباتی که خزیدن را یاد می‌گیرد

فرضیه ساده بود: یک ربات کرمی از سروو و آردوینو بسازیم و به‌جای کدنویسی دستی، با یادگیری تقویتی حرکت خزیدن را کشف کنیم.

### سخت‌افزار

- یک برد آردوینو که پنج سروو موتور را در یک خط هدایت می‌کند و یک کرم بخش‌بندی‌شده را شبیه‌سازی می‌کند.
- هر بخش به‌طور مستقل خم می‌شود و کل بدنه با هم برای حرکت رو به جلو کار می‌کند.

### چیدمان یادگیری

حالت، نمایشی فشرده از زاویه سرووهاست. کنش، مجموعه زاویه‌های هدف برای گام زمانی بعد است. پاداش، ربات را به حداکثر جابه‌جایی رو به جلو در عین حفظ محدودیت‌های ایمن مفاصل سوق می‌دهد:

```python
reward = forward_displacement - angular_penalty
```

### چه چیزی واقعاً زمان برد

1. **شکل‌دهی پاداش.** پاداش ساده «به جلو حرکت کن»، حرکت دیوانه‌وار و ناکارآمد تولید می‌کند. افزودن جریمه نرمی، راه رفتن را از لرزشی به سینوسی تبدیل کرد.
2. **نگهبان‌ها.** بدون جریمه محدودیت مفصل، عامل یاد می‌گرفت سرووها را محکم بکوبد — و این چرخ‌دنده‌ها را می‌شکند. فیزیک حرص را مجازات می‌کند.
3. **صبوری.** اجراهای اولیه خراب به نظر می‌رسیدند. قبل از ظهور یک راه رفتن منسجم، اپیزودهای زیادی لازم بود.

## درس‌ها

ساختن یک سیستم یادگیری فیزیکی اساساً با یک سیستم شبیه‌سازی‌شده تفاوت دارد. هر اپیزود زمان واقعی، انرژی واقعی و فرسایش واقعی هزینه دارد — بنابراین در مورد کارایی نمونه و ایمنی بیشتر فکر می‌کنید. این قید، پروژه را بسیار ارزشمندتر از هر شبیه‌سازی تمیزی کرد."#,
            30,
        ),
        (
            "detecting-cars-in-aerial-images-with-yolov8",
            "تشخیص خودرو در تصاویر هوایی با YOLOv8",
            "آموزش یک مدل YOLOv8 برای شمارش خودروها در تصاویر هوایی پارکینگ — از برچسب‌گذاری داده تا یک خط لوله تشخیص کارآمد.",
            "computer-vision",
            "yolo",
            r#"## مسئله

تصاویر هوایی پارکینگ‌ها یک مسئله دیداری فریبنده دشوار ارائه می‌دهند: خودروها کوچک، متراکم، اغلب پوشیده و از زاویه‌های غیرمعمول دیده می‌شوند.

## رویکرد

یک مدل YOLOv8 را روی تصاویر هوایی تنظیم دقیق کردم و سپس با استفاده از تشخیص‌ها، تعداد خودروهای حاضر در یک پارکینگ را تخمین زدم.

### خط لوله

1. **داده‌ها** — تصاویر هوایی دارای حاشیه‌نویسی از مناطق پارکینگ.
2. **آموزش** — یک مدل YOLOv8 که روی ستون فقرات تنظیم دقیق شد و انتخاب دقیق اندازه تصویر برای قابل تشخیص نگه‌داشتن خودروهای کوچک.
3. **استنتاج** — اجرای مدل روی فریم‌های هوایی جدید.
4. **شمارش** — تجمیع تشخیص‌ها در یک شمار خودرو و ثبت آن به‌عنوان نتیجه.

## درس‌های عملی

- **اشیا کوچک به تفکیک نیاز دارند.** خودروهای کوچک در اندازه‌های ورودی پیش‌فرض ناپدید می‌شوند؛ افزایش وضوح استنتاج، بزرگ‌ترین دستاورد دقت بود.
- **آستانه اطمینان اهمیت دارد.** پایین خیلی زیاد، سایه‌ها را «خودرو» می‌کند؛ بالا خیلی زیاد، خودروهای زیر درختان را از دست می‌دهید.
- **تقویت داده.** چرخش افقی و چرخش به مدل کمک کرد با جهت‌گیری دلخواه دوربین کنار بیاید.

سیستم نهایی خودروها را تشخیص می‌دهد و شمار هر تصویر را ثبت می‌کند — یک پایه تمیز و مفید برای تحلیل پارکینگ هوشمند."#,
            24,
        ),
        (
            "handwritten-digit-recognition-with-a-simple-ann",
            "بازشناسی ارقام دست‌نویس با یک شبکه عصبی ساده",
            "یک پروژه درسی که یادگیری عمیق را به جوهره‌اش می‌رساند: پیکسل‌ها در، احتمال‌ها بیرون — با یک شبکه عصبی پیش‌خور از پایه ساخته شده است.",
            "machine-learning",
            "deep-learning",
            r#"## تکلیف

بازشناسی ارقام دست‌نویس (۰ تا ۹) با یک شبکه عصبی مصنوعی. در ابتدا هیچ فریم‌ورکی — فقط ریاضیات خام انتشار پیشرو و پسرو.

## چه ساختم

- یک شبکه پیش‌خور با یک لایه ورودی از پیکسل‌های تصویر، یک لایه پنهان با فعال‌سازی غیرخطی و یک لایه خروجی softmax روی ده کلاس.
- آموزش با نزول گرادیان با استفاده از انتشار پسرو و تابع زیان cross-entropy.
- یک فریم‌ورک ارزیابی کوچک برای گزارش دقت و ماتریس درهم‌ریختگی روی مجموعه آزمون.

## این پروژه چه چیزی را برای من روشن کرد

برای من ارزش هرگز در مدل نبود — بلکه در کنار زدن فریم‌ورک و دیدن ریاضیات بود:

1. **گذر پیشرو** — ضرب ماتریس، فعال‌سازی و softmax که لاجیت‌ها را به احتمال تبدیل می‌کند.
2. **گذر پسرو** — قاعده زنجیره‌ای که لایه‌به‌لایه اعمال می‌شود و هر وزن را بر اساس سهمش در خطا به‌روزرسانی می‌کند.
3. **چشم‌انداز زیان** — تماشای کاهش زیان آموزش در طول دوره‌ها، نزول گرادیان را به شکلی ملموس‌تر از هر نمودار نشان داد.

## پس از اصول

پس از اینکه نسخه از پایه کار کرد، همان نتیجه را با یک کتابخانه یادگیری عمیق بازتولید کردم و یک خط لوله آماده تولید در کسر خطوط به دست آوردم — راهی عالی برای دیدن دقیقاً آنچه یک فریم‌ورک انتزاع می‌کند."#,
            18,
        ),
        (
            "solving-equations-with-a-genetic-algorithm",
            "حل دستگاه معادلات با الگوریتم ژنتیک",
            "وقتی جستجوی ساده خیلی کند و حساب دیفرانسیل نامناسب است، تکامل می‌تواند پاسخ بیابد — یک حل‌کننده برای دستگاه معادلات خطی و غیرخطی که بر انتخاب طبیعی بنا شده است.",
            "machine-learning",
            "research",
            r#"## چرا الگوریتم ژنتیک؟

برخی دستگاه معادلات — به‌ویژه دستگاه‌های غیرخطی — به‌شکل تحلیلی دشوارند. یک الگوریتم ژنتیک فضای راه‌حل را همان‌طور که زیست‌شناسی به دنبال ارگانیسم‌های مناسب می‌گردد، جستجو می‌کند.

## دستور پخت

1. **رمزگذاری** — هر راه‌حل کاندید یک کروموزوم حاوی مقادیر متغیرهاست.
2. **جمعیت اولیه** — حدس‌های تصادفی.
3. **برازش** — کاندید چقدر به ارضای هر معادله نزدیک است (مجموع مجذور باقی‌مانده‌ها).
4. **انتخاب** — کاندیدهای برازنده‌تر احتمال بیشتری برای والد شدن دارند.
5. **تقاطع و جهش** — ترکیب و اختلال برای کاوش مناطق جدید.
6. **تکرار** — تا همگرایی جمعیت روی یک راه‌حل کم‌خطا.

```python
fitness = 1.0 / (1.0 + total_error)
```

## رساندن به همگرایی

- **نرخ جهش** یک دیال بین کاوش و بهره‌برداری است. خیلی زیاد، جمعیت هرگز آرام نمی‌گیرد؛ خیلی کم، در بهینه محلی متوقف می‌شود.
- **زیت‌گرایی** — همیشه بهترین فرد را به نسل بعد ببرید تا بهترین پاسخ را از دست ندهید.
- **تجزیه معادله** — از تجزیه نمادین برای تایپکردن طبیعی معادلات غیرخطی توسط کاربر استفاده کردم.

نتیجه یک حل‌کننده کوچک و تعاملی است که هم حالت خطی ۲ متغیره و هم دستگاه‌های غیرخطی با ابعاد بالاتر را مدیریت می‌کند."#,
            12,
        ),
        (
            "notes-on-multi-agent-reinforcement-learning",
            "یادداشت‌هایی درباره یادگیری تقویتی چندعاملی",
            "هماهنگی، ظهور و نفرین ناپایداری — یادداشت‌های مطالعاتی من درباره اینکه چگونه عوامل متعدد با هم یاد می‌گیرند.",
            "notes",
            "multi-agent-systems",
            r#"## چرا یادگیری تقویتی چندعاملی دشوار است

در یادگیری تقویتی تک‌عاملی، محیط در حالی که عامل یاد می‌گیرد ایستاست. با داشتن چند عامل، محیط هر عامل شامل *سایر عوامل* نیز می‌شود که آن‌ها هم در حال یادگیری‌اند. این ناپایداری، دشواری اصلی است.

## رویکردها

- **یادگیری مستقل** — هر عامل دیگران را بخشی از محیط در نظر می‌گیرد. ساده اما شکننده: وقتی دیگران تغییر می‌کنند، «محیط» شما زیر پایتان تغییر می‌کند.
- **آموزش متمرکز، اجرای غیرمتمرکز** — عامل‌ها در طول آموزش اطلاعات را به اشتراک می‌گذارند (مثلاً یک منتقد مرکزی) اما هنگام اجرا بر اساس مشاهده‌های محلی عمل می‌کنند. این نقطه مطلوب برای بسیاری از وظایف رباتیک است.
- **ارتباطات** — اجازه مبادله پیام بین عامل‌ها، یادگیری آنچه *ارتباط* برقرار شود در کنار آنچه *انجام* شود.

## مرتبط با پژوهش من

کنترل آرایش ربات‌های متحرک به‌طور طبیعی یک مسئله چندعاملی است. هر ربات باید آرایش را حفظ کند و همزمان از برخورد و از دست رفتن پیام جلوگیری کند. ایده‌های یادگیری تقویتی چندعاملی — منتقدهای مرکزی و شکل‌دهی پاداش برای هماهنگی — مستقیماً به این تنظیم نگاشت می‌شوند.

## منابعی که مدام به آن‌ها برمی‌گردم

مقالات کلاسیک یادگیری تقویتی چندعاملی درباره توابع ارزش همکارانه و آموزش متمرکز کوتاه‌اند و هر بار که یک مسئله هماهنگی غیرقابل حل به نظر می‌رسد، ارزش بازخوانی دارند."#,
            6,
        ),
        (
            "what-pid-control-can-teach-us-about-life",
            "یک کنترل‌کننده PID چه چیزهایی می‌تواند درباره زندگی و علم به ما بیاموزد",
            "بازخورد همه‌جا هست — در ماشین‌ها، اقتصاد و روش یادگیری ما. یک جستار کوتاه درباره نظریه کنترل به‌عنوان یک لنز برای فهمیدن جهان.",
            "notes",
            "philosophy",
            r#"## نظریه کنترل به‌عنوان یک جهان‌بینی

یک کنترل‌کننده PID یکی از قدیمی‌ترین و فراگیرترین ایده‌های مهندسی است. به تفاوت بین جایی که هستید و جایی که می‌خواهید باشید نگاه می‌کند (خطای *e*)، سپس به‌صورت نسبی، انتگرالی و مشتقی واکنش نشان می‌دهد.

### درس

- **نسبی** — به اندازه شکاف واکنش نشان دهید. خیلی کم، منحرف می‌شوید؛ خیلی زیاد، از حد می‌گذرید.
- **انتگرالی** — گذشته را به یاد آورید و تا ناپدید شدن خطاهای طولانی‌مدت به فشار ادامه دهید.
- **مشتقی** — آینده را پیش‌بینی کنید و نوسان‌های وحشیانه را میرا کنید.

بیشتر زندگی از همین منطق پیروی می‌کند. یادگیری، علم و حتی رشد شخصی حلقه‌های بازخوردند: اندازه بگیر، تنظیم کن، تکرار کن. بهترین سیستم‌ها — زیستی، اجتماعی، فنی — آن‌هایی هستند که حلقه‌های بازخوردشان را سالم نگه می‌دارند.

## چرا این را نزدیک نگه می‌دارم

من از طریق مهندسی به نظریه کنترل رسیدم، اما آن را یک فلسفه نیز می‌یابم. به من یادآوری می‌کند که جهان پویاست، اندازه‌گیری اهمیت دارد و سیستم‌های پایدار، سیستم‌های ایستا نیستند. درک بازخورد، به تعبیری، درک چگونگی *تغییر* چیزهاست — به همین دلیل فکر می‌کنم در همان سطل ذهنی تاریخ و ریاضیات جای می‌گیرد."#,
            5,
        ),
        (
            "why-minimal-sensing-matters-in-formation-control",
            "چرا سنجش حداقلی در کنترل آرایش اهمیت دارد",
            "ربات‌هایی با سنسورهای کمتر ارزان‌تر، سبک‌تر و قابل‌اعتمادترند — اما کنترل آن‌ها دشوارتر است. این یادداشت‌های من درباره یک جهت پژوهشی است که به آن اهمیت می‌دهم.",
            "control-systems",
            "research",
            r#"## استدلال

بسیاری از کنترل ربات فرض می‌کند حس غنی وجود دارد: بازخورد کامل حالت، GPS و ارتباط قابل‌اعتماد. اما استقرارهای واقعی — گروه‌ها، ناوگان ارزان، ربات‌های موجود در محیط‌های بدون GNSS — اغلب هیچ‌کدام را ندارند.

## سنجش حداقلی

«سنجش حداقلی» یعنی انجام یک وظیفه با کمترین اطلاعات ممکن. برای کنترل آرایش این می‌تواند به این معنا باشد:

- بدون مکانیابی خارجی — فقط اندازه‌گیری‌های محلی و نسبی (مثلاً فاصله و جهت به همسایه‌ها).
- بدون هماهنگ‌کننده مرکزی — تصمیمات محلی و غیرمتمرکزند.
- ارتباط محدود — یا اصلاً هیچ، اتکای کامل به ادراک محلی.

## چرا اهمیت دارد

1. **هزینه** — سنسورهای کمتر و ارزان‌تر ناوگان‌ها را مقرون به صرفه می‌کند.
2. **تاب‌آوری** — وابستگی‌های کمتر یعنی نقاط شکست منفرد کمتر.
3. **ایمنی** — رفتار ایمن و غیرمتمرکز به از دست رفتن یک ربات یا یک پیوند مقاوم است.

## پرسش باز

چگونه *ایمنی* (بدون برخورد، خطای محدود) را تضمین می‌کنیم در حالی که هر ربات فقط یک برش از جهان را می‌بیند؟ آن تنش بین اطلاعات محلی و تضمین‌های سراسری، به نظر من، جذاب‌ترین مسئله این حوزه است — و مسئله‌ای است که در حال حاضر روی آن کار می‌کنم."#,
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
    let draft_body = r#"## یادداشت‌های پژوهشی: کنترل غیرمتمرکز آرایش

یادداشت‌های کاری فعلی من درباره پژوهش کنترل آرایش:

- [x] صورتبندی مسئله و مرور ادبیات
- [x] تنظیم سنجش حداقلی
- [ ] تضمین‌های ایمنی تحت ارتباط محدود
- [ ] شبیه‌سازی و آزمایش‌ها
- [ ] نگارش"#;
    sqlx::query(
        "INSERT INTO posts (slug, title, excerpt, content_md, content_html, status, author_id, created_at, updated_at)
         VALUES ('decentralized-formation-control-for-mobile-robots', 'کنترل غیرمتمرکز آرایش برای ربات‌های متحرک',
                 'یادداشت‌های کاری درباره کنترل ایمن و غیرمتمرکز آرایش با سنجش حداقلی.',
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
         VALUES ('from-zero-to-policy-gradient-rl', 'از صفر تا یادگیری تقویتی گرادیان سیاست',
                 'یک راهنمای مبتدی‌پسند برای روش‌های گرادیان سیاست در پنجاه خط پایتون.',
                 ?, ?, 'scheduled', ?, ?, ?, ?, ?)",
    )
    .bind(r#"## به‌زودی

یک مقدمه کوتاه و عملی درباره یادگیری تقویتی گرادیان سیاست — با کدی که می‌توانید اجرا کنید، نه فقط نمودار."#)
    .bind(markdown::to_html(r#"## به‌زودی

یک مقدمه کوتاه و عملی درباره یادگیری تقویتی گرادیان سیاست — با کدی که می‌توانید اجرا کنید، نه فقط نمودار."#))
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
            "یک بازی تعاملی ساخته‌شده با Next.js و TypeScript، با بومی‌سازی انگلیسی/فارسی و ساختار موتور بازی ماژولار.",
            r#"## درباره

یک بازی تعاملی در مرحله اولیه ساخته‌شده با Next.js (App Router) و TypeScript. پایگاه کد به‌جای یک صفحه واحد، به‌عنوان یک موتور بازی کوچک سازماندهی شده است: یک زمینه بازی وضعیت را مدیریت می‌کند، کامپوننت‌های اختصاصی HUD و پانل مأموریت را رندر می‌کنند و i18n هر رشته را هم به انگلیسی و هم به فارسی نگه می‌دارد.

## نکات برجسته

- **Next.js App Router** — صفحات مبتنی بر کامپوننت با رندر سمت سرور.
- **موتور بازی ماژولار** — یک ماژول `features/game` با زمینه، کامپوننت و خدمات.
- **رابط دوزبانه** — کاتالوگ پیام انگلیسی و فارسی با یک تعویض‌کننده زبان.
- **سیستم مأموریت** — تعاریف مأموریت مبتنی بر داده با رابط پانل.
- **لایه‌های نقشه** — رندر مستقل از کاشی برای نقشه بازی.

## وضعیت

توسعه فعال. ساختار سر جای خود است؛ محتوای بازی در حال ساخت روی آن است."#,
            "https://github.com/amirhosseineslami/iran-game",
            "",
            "#22d3ee",
            r#"["Next.js","TypeScript","React"]"#,
        ),
        (
            "khosrowan-website-rust",
            "وب‌سایت شخصی (CMS راست)",
            "یک وب‌سایت شخصی کامل، وبلاگ و سیستم مدیریت محتوا که کاملاً در راست نوشته شده — یک باینری، بدون فریم‌ورک جاوااسکریپت، بدون خدمات خارجی.",
            r#"## ایده

یک پلتفرم وب‌سایت شخصی تمام‌ویژگی که در یک باینری راست کامپایل می‌شود. قالب‌ها، CSS، جاوااسکریپت و مهاجرت‌های پایگاه داده درون‌ساختند — فایل اجرایی را کپی کنید، اجرا کنید، تمام.

## پشته

- **axum** روی **tokio** برای لایه وب.
- **askama** برای قالب‌های کامپایل‌وقت و امن ازنظر نوع.
- **sqlx** در برابر SQLite (حالت WAL).
- **comrak** برای مارک‌داون و **ammonia** برای پالایش HTML.

## چه چیزهایی شامل می‌شود

- وب‌سایت عمومی: خانه، درباره، پروژه‌ها، وبلاگ، تجربه، مهارت‌ها، خدمات، نظرها، افتخارات، سخنرانی‌ها، انتشارات، گالری، رزومه، تماس و جستجو.
- پانل مدیریت کامل برای پست‌ها، صفحات، پروژه‌ها، رسانه، کاربران، تنظیمات و بیشتر.
- امنیت: هش Argon2id، نشست‌های مبتنی بر پایگاه داده، CSRF، CSP سخت‌گیرانه، محدودیت نرخ و پالایش HTML.
- باتری‌های همراه: RSS، sitemap، Open Graph، JSON-LD، تم تیره/روشن، جستجو و خبرنامه با اعلان ایمیل.

همین وب‌سایت روی آن اجرا می‌شود."#,
            "https://github.com/amirhosseineslami/khosrowan-website-rust",
            "",
            "#f74c00",
            r#"["Rust","axum","SQLite"]"#,
        ),
        (
            "truthordarebot",
            "truthordarebot",
            "یک ربات گفتگوی ناشناس جفت‌کننده برای پلتفرم پیام‌رسان روبیکا، نوشته‌شده در پایتون.",
            r#"## چه کار می‌کند

Truthordarebot غریبه‌ها را برای گفتگوهای خصوصی ناشناس در پلتفرم پیام‌رسان روبیکا جفت می‌کند.

## ویژگی‌ها

- **جفت‌سازی ناشناس** — یک صف انتظار کاربران را به‌صورت تصادفی متصل می‌کند.
- **کیبورد درون خطی** — منوهای تعاملی برای اتصال، قطع اتصال و لغو جستجو.
- **اول فارس** — رابط کاربری برای فارسی بومی‌سازی شده است.
- **ماندگاری MySQL** — پروفایل کاربران، سکه‌ها، امتیازها، سطح‌ها و وضعیت از راه‌اندازی مجدد جان به در می‌برند.
- **ماژولار** — کلاس‌های جداگانه برای سرویس‌گیرنده API، تجزیه‌گر، کیبورد و مدیریت اتصال.

## نکات

در برابر API ربات روبیکا با استفاده از poll با به‌روزرسانی‌های offset ساخته شده است. پیکربندی توسط متغیرهای محیط انجام می‌شود و جدول‌ها در اولین اجرا به‌طور خودکار ساخته می‌شوند."#,
            "https://github.com/amirhosseineslami/truthordarebot",
            "",
            "#10b981",
            r#"["Python","MySQL","Bot API"]"#,
        ),
        (
            "CarDetection_yolov8",
            "تشخیص خودرو با YOLOv8",
            "تشخیص خودرو در پارکینگ هوایی با YOLOv8 که تعداد خودروها را تخمین می‌زند و نتایج را ثبت می‌کند.",
            r#"## هدف

تشخیص خودروها در تصاویر هوایی مناطق پارکینگ و تخمین تعداد خودروهای حاضر.

## رویکرد

- تنظیم دقیق یک تشخیص‌گر YOLOv8 روی تصاویر پارکینگ هوایی.
- اجرای استنتاج روی فریم‌های هوایی جدید.
- تجمیع تشخیص‌ها در یک شمار خودرو و ثبت آن به‌عنوان نتیجه.

## درس‌ها

خودروهای کوچک، متراکم و تا حدی پوشیده یک مورد دشوار کلاسیک‌اند. افزایش وضوح استنتاج و انتخاب دقیق آستانه اطمینان بیشترین تفاوت را ایجاد کرد.

یک پایه تمیز و مفید برای تحلیل پارکینگ هوشمند."#,
            "https://github.com/amirhosseineslami/CarDetection_yolov8",
            "",
            "#ef4444",
            r#"["Python","YOLOv8","Jupyter"]"#,
        ),
        (
            "ANN-Handwritten-Digit-Recognition",
            "بازشناسی ارقام دست‌نویس با ANN",
            "بازشناسی ارقام دست‌نویس با یک شبکه عصبی پیش‌خور، ساخته‌شده برای یک درس هوش مصنوعی.",
            r#"## درباره

یک پروژه درسی درباره بازشناسی ارقام دست‌نویس. هسته یک شبکه عصبی پیش‌خور است — پیکسل‌ها داخل، احتمال کلاس بیرون — که با انتشار پسرو آموزش می‌بیند.

اول گذرهای پیشرو و پسرو را از پایه ساختم تا ریاضیات را به‌روشنی ببینم، سپس نتیجه را با یک فریم‌ورک یادگیری عمیق بازتولید کردم تا بفهمم چه چیزی را انتزاع می‌کند."#,
            "https://github.com/amirhosseineslami/ANN-Handwritten-Digit-Recognition",
            "",
            "#6366f1",
            r#"["Python","NumPy"]"#,
        ),
        (
            "Othello-AI-MiniMax",
            "اتللو — هوش مصنوعی MiniMax",
            "اتللو / ریورسی در پایتون با حریف هوش مصنوعی مبتنی بر الگوریتم MiniMax.",
            r#"## درباره

یک بازی کامل اتللو (ریورسی) در پایتون با:

- یک حریف هوش مصنوعی **MiniMax** با هرس آلفا-بتا.
- یک رابط گرافیکی **pygame**.
- یک نسخه اجرایی مستقل ویندوز.

یک تمرین عالی در جستجوی درخت بازی و طراحی تابع ارزیابی."#,
            "https://github.com/amirhosseineslami/Othello-AI-MiniMax",
            "",
            "#8b5cf6",
            r#"["Python","pygame"]"#,
        ),
        (
            "RL_Crawling_Robot",
            "ربات کرمی با یادگیری تقویتی",
            "یک ربات کرمی شکل که با یادگیری تقویتی با استفاده از آردوینو و سروو آموزش دیده است.",
            r#"## درباره

یک ربات فیزیکی کرمی شکل ساخته‌شده از سرووهای آردوینو که با یادگیری تقویتی خزیدن را یاد می‌گیرد.

پروژه برنامه‌نویسی نهفته را با یک کنترل‌کننده مبتنی بر یادگیری ترکیب می‌کند — پاداش جابه‌جایی رو به جلو در عین مجازات حرکت خشن مفصل، تا زمانی که یک راه رفتن نرم ظهور کند.

یک درس عملی در شکل‌دهی پاداش، قیدهای فیزیکی و کارایی نمونه روی سخت‌افزار واقعی."#,
            "https://github.com/amirhosseineslami/RL_Crawling_Robot",
            "",
            "#f59e0b",
            r#"["C++","Arduino"]"#,
        ),
        (
            "genetic-algorithm-solver",
            "حل‌کننده الگوریتم ژنتیک",
            "یک حل‌کننده الگوریتم ژنتیک برای دستگاه معادلات خطی و غیرخطی.",
            r#"## درباره

یک حل‌کننده معادله پایتون که از الگوریتم ژنتیک برای یافتن راه‌حل‌های تقریبی برای دستگاه معادلات خطی و غیرخطی استفاده می‌کند.

- حل دستگاه‌های خطی ۲ متغیره و دستگاه‌های غیرخطی با ابعاد بالاتر.
- رابط تعاملی مبتنی بر منو با تجزیه نمادین معادله.
- استفاده از انتخاب طبیعی، تقاطع و جهش برای همگرایی روی راه‌حل‌های کم‌خطا.

همراه با گزارش‌های انگلیسی و فارسی درباره روش."#,
            "https://github.com/amirhosseineslami/genetic-algorithm-solver",
            "",
            "#14b8a6",
            r#"["Python","sympy"]"#,
        ),
        (
            "Agricultural-Web-Scraper",
            "خراش‌دهنده وب کشاورزی",
            "یک خراش‌دهنده وب ناهمگام برای منابع تجارت الکترونیک و کشاورزی، توسعه‌داده‌شده برای Lotus Futurist.",
            r#"## درباره

یک ابزار خراش وب توسعه‌داده‌شده برای Lotus Futurist برای استخراج داده ساخت‌یافته از وب‌سایت‌های تجارت الکترونیک و کشاورزی.

- **خراش ناهمگام** با `asyncio` و Playwright.
- **پشتیبانی چند‌سایتی** (دیجی‌کالا، باسلام، آگری‌پلاس، ترب و بیشتر)، هر کدام با ماژول خود.
- **عادی‌سازی اعداد فارسی** و ابزارهای پاک‌سازی داده.
- استخراج قیمت و واحد، با سازماندهی نتایج در خروجی‌های CSV/Excel و آرشیو."#,
            "https://github.com/amirhosseineslami/Agricultural-Web-Scraper",
            "",
            "#0ea5e9",
            r#"["Python","Playwright"]"#,
        ),
        (
            "student-depression-detection",
            "تشخیص افسردگی دانشجو",
            "تشخیص ریسک افسردگی دانشجویان با درخت تصمیم و پیش‌پردازش داده در پایتون.",
            r#"## درباره

یک پروژه علم داده که ریسک افسردگی دانشجویان را طبقه‌بندی می‌کند.

- طبقه‌بندی مبتنی بر درخت تصمیم.
- تأکید بر پاک‌سازی و پیش‌پردازش داده‌های نظرسنجی دنیای واقعی قبل از مدل‌سازی.

یک تمرین عملی در اعمال یادگیری ماشین کلاسیک روی یک مسئله معنادار اجتماعی."#,
            "https://github.com/amirhosseineslami/student-depression-detection",
            "",
            "#ec4899",
            r#"["Python","scikit-learn"]"#,
        ),
        (
            "SVM-Kernel-Analysis",
            "تحلیل هسته SVM",
            "مقایسه هسته‌های ماشین بردار پشتیبان روی مجموعه داده‌های طبقه‌بندی استاندارد.",
            r#"## درباره

یک نوت‌بوک تحلیل که هسته‌های SVM — خطی، چندجمله‌ای و RBF — را روی مجموعه داده‌های استاندارد مقایسه می‌کند و بررسی می‌کند که انتخاب هسته چگونه بر مرزهای تصمیم و دقت اثر می‌گذارد."#,
            "https://github.com/amirhosseineslami/SVM-Kernel-Analysis",
            "",
            "#22d3ee",
            r#"["Python","scikit-learn"]"#,
        ),
        (
            "Currency-Convertor-Via-MCP",
            "مبدل ارز از طریق MCP",
            "یک مبدل ارز که از طریق پروتکل زمینه مدل (MCP) هدایت می‌شود.",
            r#"## درباره

یک ابزار تبدیل ارز که از طریق پروتکل زمینه مدل (MCP) اعمال می‌شود — آزمایشی در نمایان‌سازی یک ابزار ساده به‌عنوان یک سرویس سازگار با MCP."#,
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
        ("C", "زبان‌ها", 85, 4.0),
        ("C++", "زبان‌ها", 88, 5.0),
        ("Python", "زبان‌ها", 92, 6.0),
        ("MATLAB", "زبان‌ها", 80, 3.0),
        ("Java", "زبان‌ها", 72, 3.0),
        ("Kotlin", "زبان‌ها", 70, 2.0),
        ("JavaScript / TypeScript", "زبان‌ها", 65, 2.0),
        ("PyTorch", "یادگیری ماشین", 82, 3.0),
        ("TensorFlow", "یادگیری ماشین", 78, 3.0),
        ("Scikit-learn", "یادگیری ماشین", 84, 4.0),
        ("Pandas & NumPy", "یادگیری ماشین", 86, 5.0),
        ("یادگیری تقویتی", "یادگیری ماشین", 80, 3.0),
        ("یادگیری عمیق", "یادگیری ماشین", 78, 3.0),
        ("OpenCV", "بینایی ماشین", 85, 5.0),
        ("YOLO و تشخیص شیء", "بینایی ماشین", 78, 3.0),
        ("پردازش تصویر", "بینایی ماشین", 82, 4.0),
        ("آردوینو", "رباتیک و نهفته", 86, 5.0),
        ("سیستم‌عامل C++", "رباتیک و نهفته", 80, 4.0),
        ("ROS", "رباتیک و نهفته", 68, 2.0),
        ("کنترل موتور / سروو", "رباتیک و نهفته", 78, 4.0),
        ("لینوکس", "ابزارها", 88, 6.0),
        ("Git و GitHub", "ابزارها", 90, 6.0),
        ("Docker", "ابزارها", 65, 2.0),
        ("Jupyter", "ابزارها", 85, 5.0),
        ("VS Code", "ابزارها", 88, 6.0),
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
            "دستیار پژوهش و داده",
            "Lotus Futurists",
            "تهران، ایران · دورکاری",
            "2024",
            None,
            true,
            "پشتیبانی پژوهش داده: ساخت خط‌لوله‌های خراش ناهمگام، پاک‌سازی مجموعه داده‌ها و استخراج داده ساخت‌یافته از منابع تجارت الکترونیک و کشاورزی.",
            "work",
        ),
        (
            "کارآموز پژوهشی بینایی ماشین",
            "CVLab، دانشگاه علم و صنعت ایران",
            "تهران، ایران",
            "2023",
            Some("2024"),
            false,
            "کار روی آزمایش‌های پژوهشی بینایی ماشین، آماده‌سازی داده و نمونه‌سازی مدل در یک محیط آزمایشگاهی دانشگاهی.",
            "work",
        ),
        (
            "دستیار آموزشی",
            "هوش مصنوعی · آزمایشگاه کنترل صنعتی · برنامه‌نویسی",
            "دانشگاه علم و صنعت ایران",
            "2023",
            None,
            true,
            "کمک به دروس هوش مصنوعی، آزمایشگاه کنترل صنعتی و برنامه‌نویسی — برگزاری جلسات آزمایشگاه، بازبینی تکالیف و کمک به رفع اشکال دانشجویان.",
            "work",
        ),
        (
            "بنیان‌گذار و میزبان",
            "EnglishConvey",
            "برخط",
            "2022",
            None,
            true,
            "بنیان‌گذاری و میزبانی یک ابتکار انگلیسی‌زبان که به هم‌تایی‌ها کمک می‌کند در یک محیط دوستانه و منظم گفتگو تمرین کنند.",
            "work",
        ),
        (
            "کارشناسی مهندسی برق (کنترل)",
            "دانشگاه علم و صنعت ایران",
            "تهران، ایران",
            "2021",
            None,
            true,
            "تمرکز بر سیستم‌های کنترل، رباتیک، پردازش سیگنال و هوش مصنوعی. علایق پژوهشی شامل کنترل هوشمند، سیستم‌های چندعاملی و رباتیک خودمختار است.",
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
            "آموزش خصوصی برنامه‌نویسی و ریاضیات",
            "آموزش یک‌به‌یک Python، C/C++، MATLAB و ریاضیات مهندسی — برای دانشجویانی که می‌خواهند فراتر از سرفصل بروند.",
            "code",
            "درخواستی",
            r#"["Python / C++ / MATLAB","ریاضیات مهندسی","جلسات برای دانشجو در هر سطح"]"#,
        ),
        (
            "کمک در بینایی ماشین و یادگیری ماشین",
            "کمک عملی با پروژه‌های بینایی ماشین و یادگیری ماشین: از آماده‌سازی داده تا آموزش، ارزیابی و استقرار مدل‌ها.",
            "vision",
            "درخواستی",
            r#"["آماده‌سازی و برچسب‌گذاری داده","آموزش و ارزیابی مدل","PyTorch / TensorFlow / OpenCV"]"#,
        ),
        (
            "جمع‌آوری داده و خراش وب",
            "خط‌لوله‌های خراش ناهمگام و پاک‌سازی داده برای نیازهای پژوهشی و کسب‌وکار کوچک.",
            "data",
            "درخواستی",
            r#"["خط‌لوله‌های Playwright / asyncio","خروجی CSV و Excel ساخت‌یافته","پاک‌سازی و عادی‌سازی داده"]"#,
        ),
        (
            "پروژه‌های سیستم کنترل و آردوینو",
            "راهنمایی درباره نظریه کنترل، برنامه‌نویسی نهفته و ساخت میکروکنترلر با آردوینو.",
            "chip",
            "درخواستی",
            r#"["مبانی نظریه کنترل","پروژه‌های آردوینو و سروو","شبیه‌سازی در MATLAB/Simulink"]"#,
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
            "برای ادغام درخواست‌های کشش اعطا شد — ساختن نرم‌افزار در ملأعام، یک PR در یک زمان.",
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
        ("github", "https://github.com/amirhosseineslami", "گیت‌هاب"),
        (
            "linkedin",
            "https://www.linkedin.com/in/amir-hossein-eslami",
            "لینکدین",
        ),
        ("website", "https://about.me/amirhosseineslami", "نمونه‌کار"),
        ("orcid", "https://orcid.org/0009-0000-5547-1764", "ORCID"),
        (
            "email",
            "mailto:amirhossein.eslami.ac@gmail.com",
            "ایمیل",
        ),
        ("rss", "/feed.xml", "خوراک RSS"),
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
            "درباره",
            r#"## نسخه کوتاه

من **امیرحسین اسلامی** (نویسنده وب‌سایت **خسروان**) دانشجوی مهندسی برق (گرایش کنترل) در دانشگاه علم و صنعت ایران (IUST) هستم. به سیستم‌هایی اهمیت می‌دهم که می‌توانند حس کنند، یاد بگیرند، سازگار شوند و همکاری کنند — به همین دلیل علایق من در تقاطع رباتیک، هوش مصنوعی، سیستم‌های کنترل و بینایی ماشین قرار دارد. این وب‌سایت محل انتشار یادداشت‌های پژوهشی، بلاگ‌های تخصصی و تجربه‌های فنی من است.

در حال حاضر روی کنترل غیرمتمرکز و ایمن آرایش ربات‌های متحرک دیفرانسیل با سنجش حداقلی کار می‌کنم: چگونه یک ناوگان از ربات‌های ارزان می‌تواند آرایش را حفظ کند و فقط با اندازه‌گیری‌های محلی و نسبی و بدون هماهنگ‌کننده مرکزی ایمن بماند.

## روی چه چیزی کار می‌کنم

* **رباتیک و کنترل** — کنترل آرایش، کنترل نهفته و واداشتن سیستم‌های فیزیکی به رفتار درست.
* **یادگیری ماشین و عمیق** — از طبقه‌بندی‌کننده‌های کلاسیک تا شبکه‌های عصبی و یادگیری تقویتی.
* **بینایی ماشین** — تشخیص و شمارش با YOLOv8، پردازش تصویر و ابزارهای منبع‌باز.
* **مهندسی داده** — خط‌لوله‌های خراش ناهمگام، پاک‌سازی داده و تبدیل داده بههم‌ریخته به پاسخ.

## تجربه‌ای که ارزش اشاره دارد

* دستیار پژوهش و داده در **Lotus Futurists** — ساختن خط‌لوله‌های خراش و پاک‌سازی داده.
* کارآموز پژوهشی بینایی ماشین در **CVLab، IUST**.
* دستیار آموزشی هوش مصنوعی، آزمایشگاه کنترل صنعتی و برنامه‌نویسی.
* بنیان‌گذار و میزبان **EnglishConvey**، ابتکاری برای کمک به هم‌تایی‌ها در تمرین انگلیسی.

## چگونه کار می‌کنم

چیزها را می‌نویسم. یادداشت‌های پژوهشی، گزارش‌های آزمایش و مقاله‌های کاری — آن‌قدر که هر ایده‌ای ماه‌ها بعد قابل ازسرگیری باشد. من باور دارم بهترین مهندسی، مهندسی خسته‌کننده، اندازه‌گیری‌شده و مستند است.

## بیرون از آزمایشگاه

فراتر از رباتیک و کنترل، تاریخ و فلسفه می‌خوانم و جای نرمی برای ریاضیات محض نگه می‌دارم. می‌یابم که مهندسان بزرگ اغلب متفکران بزرگ هم بودند — درک تاریخ یک ایده، پیش بردن آن را آسان‌تر می‌کند. همیشه خوشحالم درباره رباتیک، یادگیری تقویتی، یک مسئله کنترل دشوار یا یک کتاب خوب صحبت کنم."#,
            false,
            8,
        ),
        (
            "uses",
            "ابزارها",
            r#"## زبان‌ها و ابزارها

ابزارهایی که بیشتر به آن‌ها دست می‌یازم، به ترتیب تقریبی میزان زندگی در آن‌ها.

* **Python** — زبان اصلی من برای یادگیری ماشین، داده و اسکریپت.
* **C / C++** — کار نهفته (آردوینو) و کد حساس به عملکرد.
* **MATLAB** — کار نظریه کنترل و شبیه‌سازی.
* **Kotlin / Java** — پروژه‌های اندروید و JVM.

## یادگیری ماشین و داده

* **PyTorch** و **TensorFlow** — یادگیری عمیق.
* **scikit-learn**، **pandas**، **NumPy** — یادگیری ماشین کلاسیک و آماده‌سازی داده.
* **Jupyter** — اکتشاف و نوشتن.
* **OpenCV** — بینایی ماشین.

## رباتیک و نهفته

* **آردوینو** — میکروکنترلرها، سرووها و سنسورها.
* **ROS** — میان‌افزار رباتیک (در حال عمیق‌تر شدن).
* Git، GitHub و لینوکس — بستر همه موارد بالا.

## تنظیمات

همه‌چیز را ساده نگه می‌دارم: یک محیط لینوکس، یک ویرایشگر خوب و ترمینال برای بقیه چیزها. هدف این است که ابزارها سر راه کار نباشند."#,
            true,
            3,
        ),
        (
            "colophon",
            "کولوفون",
            r#"## این وب‌سایت چگونه ساخته شده است

این وب‌سایت یک باینری راست است. بدون فریم‌ورک جاوااسکریپت، بدون CMS، بدون گام ساخت برای فرانت‌اند.

* **فریم‌ورک** — [Axum](https://github.com/tokio-rs/axum) روی Tokio.
* **قالب‌بندی** — [Askama](https://github.com/askama-rs/askama)، که در زمان ساخت کامپایل می‌شود، بنابراین یک قالب شکسته یک خطای کامپایل است نه یک 500 در تولید.
* **پایگاه داده** — SQLite در حالت WAL از طریق [SQLx](https://github.com/launchbadge/sqlx).
* **محتوا** — مارک‌داون رندر شده با [comrak](https://github.com/kivikakk/comrak) و پالایش شده با [ammonia](https://github.com/raphaelm/ammonia) قبل از هرگز ذخیره شدن.
* **امنیت** — هش رمز عبور Argon2id، نشست‌های مبتنی بر پایگاه داده، محافظت CSRF و هدرهای امنیت محتوای سخت‌گیرانه.

## طراحی

زبان بصری به‌طور عمدی لاک‌پوش است: یک بوم خنثی عمیق، یک رنگ تأکیدی، فاصله سفید سخاوتمندانه و تایپوگرافی که کار را انجام می‌دهد. تم روشن و تیره هر دو درجه اول هستند و انتخاب به خاطر سپرده می‌شود.

## فلسفه

سایت یک سند نیمه‌ایستا با یک هسته پویای کوچک است. هر درخواستی که می‌توانست یک فایل باشد، یک فایل است. بقیه چیزها از یک پایگاه داده سرو می‌شود که می‌توانم درباره آن استدلال کنم، با یک فرمان نسخه پشتیبان بگیرم و با یک اسکریپت مهاجرت کنم.

> اگر برای رندر متن به یک فریم‌ورک جاوااسکریپت نیاز دارد، ابزار اشتباهی استفاده می‌شود."#,
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
