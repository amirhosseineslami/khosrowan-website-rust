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
      el.textContent = lang === "fa" && fa ? fa : (en || el.textContent);
    });
  }

  function initLang() {
    setAdminLang(adminLang());
    var toggles = document.querySelectorAll("[data-admin-lang-toggle]");
    toggles.forEach(function (btn) {
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
        // Re-run any inline initializers for the new content.
        initLang();
        document.querySelectorAll("[data-preview-toggle]").forEach(function (b) { /* noop */ });
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
