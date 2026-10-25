# خسروان — وب‌سایت شخصی (استاتیک)

این نسخه، وب‌سایت شخصی را به یک وب‌سایت ایستا (Static Site)
تبدیل می‌کند که روی **Cloudflare Pages** رایگان میزبانی می‌شود.

## چرا ایستا؟

نسخه قبلی یک CMS کامل به زبان Rust (axum + SQLite + نشست) بود.
Cloudflare Pages نمی‌تواند یک سرور طولانی‌عمر با پایگاه‌داده پایدار را
رایگان اجرا کند. تبدیل به ایستا، مزایای زیر را دارد:

- **رایگان و بی‌نهایت** (پهنای باند و تعداد درخواست نامحدود در پلن Free)
- **سریع** — از لبه شبکه Cloudflare در سراسر جهان سرو می‌شود
- **امن** — HTTPS رایگان خودکار، حفاظت DDoS
- **بدون نیاز به ورود** — فایل‌های ایستا هستند، هرگز خاموش یا منقضی نمی‌شوند
- **مدیریت محتوا** — با Decap CMS از مسیر `/admin/` مقاله، پروژه و صفحه بنویسید

## ساختار

```
site/
  content/posts/     # مقاله‌ها (Markdown)
  content/projects/  # پروژه‌ها
  content/pages/     # صفحات (درباره، ابزارها، ...)
  static/admin/       # پنل مدیریت (Decap CMS)
  static/assets/      # فونت، CSS، تصویر
  layouts/            # قالب‌های Hugo (themes/khosrowan)
  hugo.toml           # پیکربندی
```

## ساخت لوکال

```bash
cd site
hugo server -D        # http://localhost:1313
hugo --minify --destination public   # خروجی نهایی
```

## استقرار

مخزن GitHub به‌صورت خودکار از طریق GitHub Action
(`.github/workflows/deploy.yml`) روی Cloudflare Pages منتشر می‌شود.
کافی است به شاخه `master` پوش کنید؛ سایت همان لحظه بازسازی و استقرار می‌یابد.

Secret مخزن `CLOUDFLARE_API_TOKEN` تنظیم شده است.

## پنل مدیریت (Decap CMS)

پنل در آدرس زیر در دسترس است:

```
https://khosrowan.pages.dev/admin/
```

برای اتصال به GitHub، یک **GitHub OAuth App** بسازید:

1. به https://github.com/settings/developers بروید → *OAuth Apps* → *New OAuth App*.
2. Homepage URL: `https://khosrowan.pages.dev`
3. Authorization callback URL: `https://khosrowan.pages.dev/admin/index.html`
4. `Client ID` را کپی کنید.
5. در فایل `site/static/admin/config.yml` مقدار `client_id` را جایگزین کنید.
6. تغییر را پوش کنید (GitHub Action استقرار را انجام می‌دهد).

## تبدیل از نسخه Rust

محتوای قبلی از پایگاه داده SQLite به فایل‌های Markdown منتقل شد:
- ۷ مقاله منتشرشده
- ۱۲ پروژه
- ۳ صفحه (درباره، ابزارها، کولوفون)
- همه تنظیمات سایت (عنوان، شبکه‌های اجتماعی و ...)
