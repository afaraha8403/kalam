---
name: README improvement plan
overview: A research-backed plan to improve the Kalam README with navigation, visuals, clarity, and trust signals so it better converts visitors into users and contributors.
todos: []
isProject: false
---

# README improvement plan

## Current state

The [README.md](README.md) is already strong: clear value prop, download/docs links, badges, STT/command-mode sections, quick start, and build instructions. The following improvements are based on common open-source README best practices (TOC for navigation, visuals for engagement, FAQ for clarity, system requirements for desktop apps) and optional enhancements.

---

## 1. Table of contents (recommended)

**Why:** GitHub’s outline helps, but an explicit TOC at the top improves scanning and deep links. Useful once the README has several H2 sections (you have ~10).

**Change:** Add a short TOC with anchor links after the badges and before “Features”. Use standard GitHub anchors (lowercase, spaces → hyphens), e.g. `#quick-start`, `#speech-to-text-stt-providers`, `#building-from-source`.

**Example:**

```markdown
## Contents
- [Features](#features)
- [Speech-to-Text (STT) providers](#speech-to-text-stt-providers)
- [Command mode](#command-mode-notes-tasks-reminders)
- [Quick start](#quick-start)
- [Building from source](#building-from-source)
- [Architecture](#architecture)
- [API keys](#api-keys-byok)
- [Privacy](#privacy)
- [Contributing](#contributing)
- [License](#license)
```

Keep it to one line per section so it stays compact.

---

## 2. Screenshots or demo (high impact, needs assets)

**Why:** READMEs with screenshots or a short GIF get more engagement and make the product concrete. Your repo has logos and icons ([public/logo/](public/logo/), [docs/logo.svg](docs/logo.svg), [docs/hero-bg.webp](docs/hero-bg.webp)) but no app UI screenshot or dictation demo.

**Options:**

- **Option A — Screenshot:** One or two static images (e.g. main window + overlay, or Settings → STT). Store under `docs/` (e.g. `docs/screenshots/main-window.png`) so the same assets can be used on the GitHub Pages site. In the README, add a “See it in action” or “Screenshots” section after Features with something like:  
`![Main window](docs/screenshots/main-window.png)` (or a link to the website if you prefer to keep images only there).
- **Option B — Animated GIF:** Short (5–15 s) “hold hotkey → speak → text appears” demo. More work but very effective. Again, host under `docs/` and reference from README.

**Implementation note:** No image files exist yet; you’ll need to capture them (or record a GIF) and add paths in the README. The plan is to add the section and placeholder/instructions; actual asset creation is a separate step.

---

## 3. System requirements (recommended for desktop apps)

**Why:** Desktop app READMEs often list minimum OS (and sometimes hardware) so users know if they can run the app. Your [docs/documentation.html](docs/documentation.html) covers permissions (microphone, accessibility) but not minimum OS versions.

**Change:** Add a short “Requirements” or “System requirements” subsection under **Quick start** (or its own small section):

- **OS:** e.g. Windows 10+ (64-bit), macOS 11+, or a modern Linux distro (with typical Tauri deps: GTK, WebKit, etc.). You can align with what your CI uses ([.github/workflows/ci.yml](.github/workflows/ci.yml): `ubuntu-latest`, `windows-latest`, `macos-latest`) and any Tauri docs.
- **Permissions:** Microphone and accessibility (for text injection). One line with a link to the [documentation](https://afaraha8403.github.io/kalam/documentation.html#getting-started) “System permissions” part is enough.

Keeps the README short while answering “will this run on my machine?”

---

## 4. FAQ (optional but high value)

**Why:** A small FAQ addresses common objections and search-style questions (e.g. “Do I need an API key?”, “Does it work offline?”). Your [docs/index.html](docs/index.html) already has JSON-LD FAQ; mirroring 3–5 questions in the README improves clarity and SEO on the repo.

**Change:** Add an “FAQ” section (e.g. after Privacy, before Contributing) with 3–5 short Q&As. Examples:

- Do I need an API key? (Cloud: yes, BYOK; local: no.)
- Does it work offline? (Yes with local STT.)
- How do I start dictating? (Hotkey; link to Quick start or docs.)
- What’s the difference between dictation and command mode? (Dictation = type into focused app; command = create note/task/reminder.)
- Is my audio stored or sent? (One line; link to Privacy section or docs.)

Keep each answer to 1–2 sentences and link to the [documentation](https://afaraha8403.github.io/kalam/documentation.html) for more.

---

## 5. Download by OS (optional)

**Why:** Some users prefer an explicit “Download for Windows / macOS / Linux” instead of a single “Releases” link. GitHub’s “latest” release page lists assets by filename; you can’t have stable anchor links to specific asset files (names include version numbers).

**Options:**

- **A:** Keep a single “Download” link to [Releases](https://github.com/afaraha8403/kalam/releases/latest) and one line: “Pick the installer for your OS (Windows, macOS, or Linux).”
- **B:** Add three bullet or badge-style links that all point to the same latest release URL, with labels “Windows | macOS | Linux” so it’s clear all three are there. No per-OS deep links unless you add a small script or redirect layer.

Recommendation: A is enough unless you add a download page on the website that does OS detection and offers one direct link per platform.

---

## 6. Problem–solution line (optional)

**Why:** Some best-practice guides suggest opening with the problem (“Typing is slow”, “Voice input shouldn’t be locked to one app”) then the solution. Your first paragraph already says what Kalam is and that it’s a Whisperflow alternative.

**Change:** Optionally add one sentence before or after the current first paragraph, e.g. “Type less and stay in flow: use your voice in any app, with your own API key or fully offline.” Skip if you prefer to keep the intro minimal.

---

## 7. Badges (optional)

**Current:** Release version, Platform, License, Tauri, Open source.

**Possible additions:**

- **CI status:** `https://img.shields.io/github/actions/workflow/status/afaraha8403/kalam/ci.yml?branch=main` — shows build passing/failing; good for contributors. Your [.github/workflows/ci.yml](.github/workflows/ci.yml) is named `ci.yml`.
- **Download count:** e.g. `https://img.shields.io/github/downloads/afaraha8403/kalam/total` — social proof once you have a meaningful number of downloads.

Add only if you want the README to emphasize build health and/or download stats.

---

## 8. Contributing (optional)

**Current:** “Issues and pull requests are welcome. See GitHub to get started.”

**Improvement:** If you later add a [CONTRIBUTING.md](CONTRIBUTING.md) (code of conduct, how to run tests, PR process), link it here: “See [CONTRIBUTING.md](CONTRIBUTING.md) and [GitHub](url) to get started.” No CONTRIBUTING.md exists in the repo today, so this is a future tweak.

---

## 9. Order of sections (no content change)

Current order is already good: value → links → badges → features → STT → command mode → quick start → build → architecture → API keys → privacy → contributing → license → acknowledgments. No need to reorder unless you add a “See it in action” block (then place it right after Features or after the first paragraph).

---

## Summary: what to do first


| Priority | Item                                 | Effort               | Impact                                     |
| -------- | ------------------------------------ | -------------------- | ------------------------------------------ |
| 1        | Table of contents                    | Low                  | Navigation, clarity                        |
| 2        | System requirements                  | Low                  | Trust, fewer “does it run here?” questions |
| 3        | FAQ (3–5 questions)                  | Low                  | Clarity, SEO, conversion                   |
| 4        | Screenshots or demo                  | Medium (need assets) | Engagement, “sell”                         |
| 5        | CI badge (optional)                  | Low                  | Contributor trust                          |
| 6        | Problem–solution line (optional)     | Low                  | Slight copy improvement                    |
| 7        | Download by OS (optional)            | Low                  | Minor UX                                   |
| 8        | CONTRIBUTING link (when file exists) | Low                  | Contributor onboarding                     |


Recommended minimum for “improve further”: **TOC + system requirements + FAQ**. Add **screenshots/demo** once you have assets; then consider **CI (and optionally download) badge** and the optional one-line **problem–solution** and **Contributing** link.