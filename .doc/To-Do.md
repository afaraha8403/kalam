# kalam (desktop app) — deferred / Phase 3 follow-ups

This file is for **this repo** (Tauri / Rust dictation app).  
For **website, Cloudflare, Stripe, and Worker deploy**, use: **`kalam-website/.doc/To-Do.md`**.

---

## When the backend is ready

- [ ] **License API base URL** — Configure the desktop app to call the deployed Worker (e.g. `GET /api/license/validate`) instead of any placeholder; align env/build config with production vs sandbox.
- [ ] **Plan / trial / Pro state** — Implement `PlanStatus` (Free / Trial / Pro / Expired), startup + periodic validation, and **offline grace** per the upgrade plan.
- [ ] **Upgrade UX** — “Upgrade to Pro”, license key entry in Settings, link to Stripe Customer Portal / pricing on the **new** site when it replaces GitHub Pages.
- [ ] **Telemetry (optional)** — Point opt-in events to **`/api/analytics/ingest`** on the new backend if you want first-party analytics alongside or instead of PostHog.

---

## Licensing & legal (repo)

- [ ] **FSL** — When you switch from the current license to Functional Source License, update `LICENSE`, in-app copy, and **`docs/`** legal pages (or the new site’s terms) consistently.

---

## Docs & marketing site

- [ ] **`docs/` on GitHub Pages** — Stays the public site until you cut DNS to Cloudflare; then migrate or redirect as decided in `kalam-website/.doc/To-Do.md`.

---

*Cross-repo: Phase 3 website + service = `kalam-website`; app + open source = `kalam`.*
