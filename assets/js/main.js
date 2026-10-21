/* personal-site — progressive enhancement.
   The site works fully without JavaScript; this file only adds polish. */

(function () {
  "use strict";

  /* ---------------------------------------------------------------------
     Theme handling: dark / light, remembered across visits.
     --------------------------------------------------------------------- */
  var STORAGE_KEY = "ps-theme";

  function readStoredTheme() {
    try {
      return window.localStorage.getItem(STORAGE_KEY);
    } catch (e) {
      return null;
    }
  }

  function storeTheme(theme) {
    try {
      window.localStorage.setItem(STORAGE_KEY, theme);
    } catch (e) {
      /* storage may be unavailable — ignore */
    }
  }

  function systemTheme() {
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }

  function currentTheme() {
    return document.documentElement.getAttribute("data-theme") || systemTheme();
  }

  function applyTheme(theme) {
    document.documentElement.setAttribute("data-theme", theme);
    storeTheme(theme);
    document.querySelectorAll("[data-theme-toggle]").forEach(function (button) {
      var sun = button.querySelector("[data-icon='sun']");
      var moon = button.querySelector("[data-icon='moon']");
      if (sun && moon) {
        sun.style.display = theme === "dark" ? "none" : "";
        moon.style.display = theme === "dark" ? "" : "none";
      }
      button.setAttribute("aria-label", theme === "dark" ? "Switch to light theme" : "Switch to dark theme");
    });
  }

  function initTheme() {
    var stored = readStoredTheme();
    applyTheme(stored || currentTheme());

    document.querySelectorAll("[data-theme-toggle]").forEach(function (button) {
      button.addEventListener("click", function () {
        applyTheme(currentTheme() === "dark" ? "light" : "dark");
      });
    });
  }

  /* ---------------------------------------------------------------------
     Mobile navigation
     --------------------------------------------------------------------- */
  function initNav() {
    var toggle = document.querySelector("[data-nav-toggle]");
    var links = document.querySelector("[data-nav-links]");

    if (!toggle || !links) return;

    toggle.addEventListener("click", function () {
      var open = links.classList.toggle("open");
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    });

    links.addEventListener("click", function (event) {
      if (event.target.tagName === "A") {
        links.classList.remove("open");
        toggle.setAttribute("aria-expanded", "false");
      }
    });

    document.addEventListener("keydown", function (event) {
      if (event.key === "Escape") {
        links.classList.remove("open");
        toggle.setAttribute("aria-expanded", "false");
      }
    });
  }

  /* ---------------------------------------------------------------------
     Sticky header shadow
     --------------------------------------------------------------------- */
  function initHeader() {
    var header = document.querySelector("[data-header]");
    if (!header) return;

    function onScroll() {
      header.classList.toggle("scrolled", window.scrollY > 12);
    }

    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
  }

  /* ---------------------------------------------------------------------
     Reveal on scroll
     --------------------------------------------------------------------- */
  function initReveal() {
    var elements = document.querySelectorAll(".reveal");
    if (!elements.length) return;

    if (!("IntersectionObserver" in window)) {
      elements.forEach(function (el) {
        el.classList.add("visible");
      });
      return;
    }

    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add("visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
    );

    elements.forEach(function (el, index) {
      el.style.transitionDelay = Math.min(index % 6, 5) * 60 + "ms";
      observer.observe(el);
    });
  }

  /* ---------------------------------------------------------------------
     Animated skill bars
     --------------------------------------------------------------------- */
  function initSkills() {
    var fills = document.querySelectorAll("[data-skill-fill]");
    if (!fills.length) return;

    function fill() {
      fills.forEach(function (el) {
        el.style.width = el.getAttribute("data-skill-fill") + "%";
      });
    }

    if (!("IntersectionObserver" in window)) {
      fill();
      return;
    }

    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            fill();
            observer.disconnect();
          }
        });
      },
      { threshold: 0.2 }
    );

    var section = document.querySelector(".skill-grid") || document.body;
    observer.observe(section);
  }

  /* ---------------------------------------------------------------------
     Hero word rotator
     --------------------------------------------------------------------- */
  function initRotator() {
    var rotator = document.querySelector("[data-rotator]");
    if (!rotator) return;

    var words = rotator.getAttribute("data-words");
    if (!words) return;

    var list = words.split("|").filter(Boolean);
    if (list.length < 2) return;

    var index = 0;
    var span = rotator.querySelector("[data-rotator-text]");

    function tick() {
      index = (index + 1) % list.length;
      if (!span) return;
      span.classList.remove("word");
      span.textContent = list[index];
      /* force reflow so the animation replays */
      void span.offsetWidth;
      span.classList.add("word");
    }

    setInterval(tick, 2600);
  }

  /* ---------------------------------------------------------------------
     Copy-to-clipboard for share buttons and code blocks
     --------------------------------------------------------------------- */
  function initCopy() {
    document.querySelectorAll("[data-copy]").forEach(function (button) {
      button.addEventListener("click", function () {
        var selector = button.getAttribute("data-copy");
        var target = selector ? document.querySelector(selector) : null;
        var text = target ? target.innerText : "";

        if (!text || !navigator.clipboard) return;

        navigator.clipboard.writeText(text).then(function () {
          var original = button.getAttribute("title") || button.textContent;
          button.setAttribute("title", "Copied!");
          button.textContent = "Copied";
          setTimeout(function () {
            button.setAttribute("title", original);
            button.textContent = original;
          }, 1600);
        });
      });
    });
  }

  /* ---------------------------------------------------------------------
     Auto-dismiss flash messages
     --------------------------------------------------------------------- */
  function initFlash() {
    document.querySelectorAll("[data-flash]").forEach(function (el) {
      setTimeout(function () {
        el.style.transition = "opacity 400ms ease, transform 400ms ease";
        el.style.opacity = "0";
        el.style.transform = "translateY(-6px)";
        setTimeout(function () {
          el.remove();
        }, 420);
      }, 6000);
    });
  }

  /* ---------------------------------------------------------------------
     Reading progress bar on articles
     --------------------------------------------------------------------- */
  function initProgress() {
    var bar = document.querySelector("[data-progress]");
    var article = document.querySelector("[data-article]");
    if (!bar || !article) return;

    function update() {
      var rect = article.getBoundingClientRect();
      var total = rect.height - window.innerHeight;
      var scrolled = total <= 0 ? 0 : -rect.top / total;
      var pct = Math.max(0, Math.min(1, scrolled)) * 100;
      bar.style.width = pct + "%";
    }

    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
  }

  /* ---------------------------------------------------------------------
     Admin: Markdown editor with live preview
     --------------------------------------------------------------------- */
  function initEditor() {
    var editor = document.querySelector("[data-editor]");
    if (!editor) return;

    var preview = document.querySelector("[data-preview]");
    var stats = document.querySelector("[data-editor-stats]");
    var csrf = document.querySelector("[data-csrf]");

    if (!preview) return;

    var timer = null;

    function render() {
      var body = new URLSearchParams();
      body.append("content", editor.value);
      body.append("csrf_token", csrf ? csrf.value : "");

      fetch("/admin/api/markdown", {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          "X-CSRF-Token": csrf ? csrf.value : ""
        },
        body: body.toString()
      })
        .then(function (response) {
          return response.json();
        })
        .then(function (data) {
          preview.innerHTML = data.html || "<p class='text-faint'>Nothing to preview yet.</p>";
          if (stats) {
            stats.textContent =
              (data.word_count || 0) + " words · " + (data.reading_time || 1) + " min read";
          }
        })
        .catch(function () {
          preview.innerHTML = "<p class='text-faint'>Preview unavailable.</p>";
        });
    }

    editor.addEventListener("input", function () {
      if (timer) window.clearTimeout(timer);
      timer = window.setTimeout(render, 400);
    });

    var toggle = document.querySelector("[data-preview-toggle]");
    if (toggle) {
      toggle.addEventListener("click", function () {
        var wrap = document.querySelector("[data-preview-wrap]");
        if (!wrap) return;
        var hidden = wrap.hasAttribute("hidden");
        if (hidden) {
          wrap.removeAttribute("hidden");
          render();
        } else {
          wrap.setAttribute("hidden", "");
        }
      });
    }

    if (editor.value.trim().length > 0) render();
  }

  /* ---------------------------------------------------------------------
     Admin: slug auto-generation from the title
     --------------------------------------------------------------------- */
  function initSlug() {
    var title = document.querySelector("[data-slug-source]");
    var slug = document.querySelector("[data-slug-target]");
    if (!title || !slug) return;

    var touched = slug.value.trim().length > 0;

    slug.addEventListener("input", function () {
      touched = true;
    });

    title.addEventListener("input", function () {
      if (touched) return;
      slug.value = title.value
        .toLowerCase()
        .trim()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-+|-+$/g, "");
    });
  }

  /* ---------------------------------------------------------------------
     Admin: confirm destructive actions
     --------------------------------------------------------------------- */
  function initConfirm() {
    document.querySelectorAll("[data-confirm]").forEach(function (form) {
      form.addEventListener("submit", function (event) {
        var message = form.getAttribute("data-confirm") || "Are you sure?";
        if (!window.confirm(message)) {
          event.preventDefault();
        }
      });
    });
  }

  /* ---------------------------------------------------------------------
     Admin: auto-submit filters
     --------------------------------------------------------------------- */
  function initFilter() {
    document.querySelectorAll("[data-filter-select]").forEach(function (select) {
      select.addEventListener("change", function () {
        var form = select.closest("form");
        if (form) form.submit();
      });
    });
  }

  /* ---------------------------------------------------------------------
     Boot
     --------------------------------------------------------------------- */
  function boot() {
    initTheme();
    initNav();
    initHeader();
    initReveal();
    initSkills();
    initRotator();
    initCopy();
    initFlash();
    initProgress();
    initEditor();
    initSlug();
    initConfirm();
    initFilter();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
