/* personal-site — admin panel progressive enhancement.
   - Smooth in-panel navigation (fetch + swap, no full page reload).
   - Admin language toggle (fa / en) for the panel chrome, remembered. */

(function () {
  "use strict";

  var LANG_KEY = "ps-admin-lang";

  function adminLang() {
    try {
      return window.localStorage.getItem(LANG_KEY) || "en";
    } catch (e) {
      return "en";
    }
  }

  function setAdminLang(lang) {
    try {
      window.localStorage.setItem(LANG_KEY, lang);
    } catch (e) { /* ignore */ }
    document.documentElement.setAttribute("data-admin-lang", lang);
    var links = document.querySelectorAll("[data-lang-key]");
    links.forEach(function (el) {
      var fa = el.getAttribute("data-lang-fa");
      var en = el.getAttribute("data-lang-en");
      var text = lang === "fa" && fa ? fa : (en || el.textContent);
      if (el.hasAttribute("placeholder")) {
        el.setAttribute("placeholder", text);
      } else {
        el.textContent = text;
      }
    });
  }

  function initLang() {
    setAdminLang(adminLang());
    // Bind the toggle listener once per button. initLang() also runs after
    // each smooth navigation, and the sidebar button is not re-created, so
    // without this guard duplicate listeners would double-toggle (no-op).
    var toggles = document.querySelectorAll("[data-admin-lang-toggle]");
    toggles.forEach(function (btn) {
      if (btn.dataset.langBound) return;
      btn.dataset.langBound = "1";
      btn.addEventListener("click", function (e) {
        e.preventDefault();
        setAdminLang(adminLang() === "fa" ? "en" : "fa");
      });
    });
  }

  /* ------------------------------------------------------------------
     Smooth in-panel navigation.
     Intercept clicks on links pointing to /admin/... within the main
     content area, fetch the page and swap `.admin-main` content.
     ------------------------------------------------------------------ */
  function isAdminLink(href) {
    if (!href) return false;
    return href.indexOf("/admin") === 0 || href.indexOf(window.location.origin + "/admin") === 0;
  }

  function pathOf(href) {
    try {
      return new URL(href, window.location.origin).pathname + new URL(href, window.location.origin).search;
    } catch (e) {
      return href;
    }
  }

  function initSmoothNav() {
    var main = document.querySelector(".admin-main");
    if (!main) return;

    document.addEventListener("click", function (e) {
      var link = e.target.closest ? e.target.closest("a[href]") : null;
      if (!link) return;
      var href = link.getAttribute("href");
      if (!isAdminLink(href)) return;
      if (link.target === "_blank") return;
      if (link.hasAttribute("data-no-smooth")) return;

      var path = pathOf(href);
      if (path === pathOf(window.location.href)) return;

      e.preventDefault();
      loadInto(main, path);
    });

    window.addEventListener("popstate", function () {
      loadInto(main, pathOf(window.location.href), true);
    });
  }

  // Highlight the sidebar link matching the current path. The dashboard link
  // (`/admin`) is a prefix of every other admin route, so only it matches
  // exactly; other links match on a path-segment prefix.
  function setActiveNav(path) {
    var clean = path.split("?")[0];
    document.querySelectorAll(".admin-nav a").forEach(function (a) {
      var href = (a.getAttribute("href") || "").split("?")[0];
      var active = false;
      if (href && clean.indexOf(href) === 0) {
        active = href === "/admin" ? clean === "/admin" : clean.length === href.length || clean.charAt(href.length) === "/";
      }
      a.classList.toggle("active", active);
    });
  }

  function loadInto(container, path, fromHistory) {
    // Keep the sidebar visible; only the main content area swaps.
    fetch(path, { headers: { "Accept": "text/html" } })
      .then(function (res) {
        if (!res.ok) throw new Error("HTTP " + res.status);
        return res.text();
      })
      .then(function (html) {
        var doc = new DOMParser().parseFromString(html, "text/html");
        var newMain = doc.querySelector(".admin-main");
        if (!newMain) return;
        container.innerHTML = newMain.innerHTML;
        if (!fromHistory) {
          window.history.pushState({}, "", path);
        }
        setActiveNav(path);
        if (doc.title) {
          document.title = doc.title;
        }
        // Re-run any inline initializers for the new content.
        initLang();
        document.dispatchEvent(new CustomEvent("ps:admin-content-swapped"));
        window.scrollTo({ top: 0, behavior: "smooth" });
      })
      .catch(function () {
        // Fallback: full navigation on failure.
        window.location.href = path;
      });
  }

  document.addEventListener("DOMContentLoaded", function () {
    initLang();
    initSmoothNav();
  });
})();
