# Kalam Upgrade Plan — Dictation Modes, Context Awareness, Model Library & Business Model

**Date:** March 29, 2026
**Status:** Draft for Review
**Related docs:** `superwhisper-competitor-review.md`, `kalam-vs-superwhisper.md`

---

## What This Plan Is About

This plan lays out how to transform Kalam from a "press hotkey → get text" dictation tool into a **smart, context-aware voice assistant** that rivals SuperWhisper — while keeping everything that makes Kalam special (open source, lightweight, privacy-first).

It also covers **how Kalam makes money** — a fair, affordable premium tier that funds development without locking away the core experience.

Nine phases, built in order:

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│   1. MODES + POLISH + ONBOARDING  "What kind of         │
│      (the foundation)              dictation am I doing?"│
│                                                         │
│   2. CURATED MODEL LIBRARY        "Which AI brains      │
│      (the engine room)             are available?"      │
│                                                         │
│   3. KALAM PRO + WEBSITE          "How does this stay   │
│      + SERVICE                     sustainable?"        │
│      (the business)                                     │
│                                                         │
│   4. CONTEXT AWARENESS            "What's on my screen  │
│      (the intelligence)            right now?"          │
│                                                         │
│   5. VOICE EDITING                "Edit text with my    │
│      (the power tool)              voice?"              │
│                                                         │
│   6. OVERLAY REDESIGN             "How do I interact    │
│      (the experience)              with all of this?"   │
│                                                         │
│   7. AUTO-ACTIVATION              "Switch modes         │
│      (the automation)              automatically?"      │
│                                                         │
│   8. COMMUNITY RECIPES            "What can others      │
│      (the ecosystem)               teach me?"           │
│                                                         │
│   9. SYNC                         "Same experience on   │
│      (the glue)                    all my PCs?"         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## The Big Picture — Before & After

### How Kalam Works Today

```
┌──────────────────────────────────────────────────┐
│                                                  │
│   You press hotkey → You speak → Text appears    │
│                                                  │
│   That's it. One pipeline. One behavior.         │
│   Same thing every time, no matter what          │
│   app you're in or what you're doing.            │
│                                                  │
│   (Command mode exists but it's a separate       │
│    hotkey, separate flow, separate world.)        │
│                                                  │
└──────────────────────────────────────────────────┘
```

### How Kalam Will Work After This Upgrade

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│   You press hotkey → Kalam checks your active MODE           │
│   and applies your settings:                                 │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐    │
│   │  MODES (user-created, unlimited)                    │    │
│   │                                                     │    │
│   │  Every mode has:                                    │    │
│   │  • A name you choose                               │    │
│   │  • AI instructions (what should the AI do?)         │    │
│   │  • Voice model (which STT engine)                   │    │
│   │  • Language model (which LLM)                       │    │
│   │  • Context toggles (app, clipboard, selection)      │    │
│   │  • Auto-activation rules (per app)                  │    │
│   │                                                     │    │
│   │  Examples:                                          │    │
│   │  "Email" — formats as email with greeting/sign-off  │    │
│   │  "Slack" — casual tone, short messages              │    │
│   │  "Code Comments" — formats as code comments         │    │
│   │  "French" — translates everything to French         │    │
│   │  "Meeting Notes" — bullet points, key takeaways     │    │
│   │  Or anything else you can imagine.                  │    │
│   └─────────────────────────────────────────────────────┘    │
│                                                              │
│   On top of ANY mode, two features can be toggled:           │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐    │
│   │  ✨ POLISH (toggle — works on any mode)             │    │
│   │                                                     │    │
│   │  When ON, AI cleans up your speech BEFORE the       │    │
│   │  mode's instructions run:                           │    │
│   │  • Fixes grammar and spelling                       │    │
│   │  • Removes filler words and false starts            │    │
│   │  • Proper punctuation and capitalization             │    │
│   │  • Extracts what you're really trying to say        │    │
│   │  • Formats structure (lists, paragraphs, etc.)      │    │
│   │                                                     │    │
│   │  Works with voice commands too:                     │    │
│   │  "new list" → starts a bulleted list                │    │
│   │  "new paragraph" → inserts paragraph break          │    │
│   │  "make this a heading" → formats as heading         │    │
│   │                                                     │    │
│   │  When OFF, raw transcription goes to the mode.      │    │
│   └─────────────────────────────────────────────────────┘    │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐    │
│   │  🔍 CONTEXT AWARENESS (toggle — works on any mode)  │    │
│   │                                                     │    │
│   │  When ON, Kalam reads your screen:                  │    │
│   │  • What app you're in                               │    │
│   │  • Text you've highlighted                          │    │
│   │  • What you just copied                             │    │
│   │                                                     │    │
│   │  AI uses all of this to give you smarter,           │    │
│   │  more relevant results.                             │    │
│   │                                                     │    │
│   │  When OFF, AI only sees your spoken words.          │    │
│   └─────────────────────────────────────────────────────┘    │
│                                                              │
│   THE PIPELINE (only 2 calls max — never more):              │
│                                                              │
│   You speak → STT transcribes (1 API call)                   │
│                     ↓                                        │
│   [Context ON?] → Kalam reads screen/clipboard/selection     │
│                    (local — no API call, just reading)        │
│                     ↓                                        │
│   [Polish ON or Mode has AI instructions?]                   │
│     → ONE single LLM call (1 API call) that receives:        │
│       • Your transcript                                      │
│       • Context data (if ON)                                 │
│       • Polish instructions (if ON)                          │
│       • Mode instructions                                    │
│       All in one prompt. One call. One result.               │
│                     ↓                                        │
│   Final text injected                                        │
│                                                              │
│   Voice Mode (no Polish, no AI instructions):                │
│     → ZERO LLM calls. Just STT → inject. Fastest.           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Part 1: Dictation Modes + Polish

### The Key Insight: Polish Is Not a Mode — It's a Feature

In the previous draft, "Polished" was a standalone mode. But you're right — polish (grammar cleanup, formatting, extracting what you really mean) is something you'd want on **any** mode. If you're in "Email" mode, you want polish. If you're in "Meeting Notes" mode, you want polish. It doesn't make sense to choose between "Polished" and "Email" — you want both.

So **Polish becomes a toggle** that layers on top of whatever mode you're in:

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  OLD THINKING (wrong):                                       │
│                                                              │
│  Voice Mode  OR  Polished Mode  OR  Email Mode               │
│  (pick one)                                                  │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  NEW THINKING (correct):                                     │
│                                                              │
│  [Any Mode] + [Polish ON/OFF] + [Context ON/OFF]             │
│                                                              │
│  Email Mode + Polish ON + Context ON                         │
│  → Speaks naturally, AI cleans up grammar/filler,            │
│    reads your screen, formats as proper email.               │
│                                                              │
│  Voice Mode + Polish OFF + Context OFF                       │
│  → Raw transcription. Fast. No AI. What Kalam does today.   │
│                                                              │
│  Slack Mode + Polish ON + Context OFF                        │
│  → Cleans up speech, formats as casual short message.        │
│                                                              │
│  Code Comments Mode + Polish OFF + Context ON                │
│  → Raw speech, but AI sees your code editor and formats      │
│    as comments for the language you're writing in.           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### What Is a Mode?

A mode is a **recipe** for how Kalam processes your voice after transcription (we actually call them "recipes" when they're shared — more on that below). Every mode has:

- **A name** — you choose it ("Email", "Slack", "Meeting Notes", "French", anything)
- **AI instructions** — what should the AI do with your text? (can be empty for raw dictation)
- **Voice model** — which STT engine to use (local or cloud)
- **Language model** — which LLM to use for AI processing
- **Context toggles** — should the AI read your app, clipboard, selected text?
- **Auto-activation rules** — automatically switch to this mode when certain apps are active

Users can create **any mode they want**, save it, and reuse it. Modes are the user's personal library of dictation recipes. And when they create something great, they can **share it as a recipe** for others to use.

### Recipes — Shareable Mode Templates

A **recipe** is a mode packaged for sharing. Think of it like a cooking recipe — it tells you the ingredients (which models, which settings) and the instructions (what the AI should do), but you cook it in your own kitchen (your own API keys, your own computer).

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  WHAT IS A RECIPE?                                           │
│                                                              │
│  A recipe is a JSON file containing:                         │
│  • Mode name and description                                 │
│  • AI instructions (the prompt)                              │
│  • Recommended voice model (e.g., "Groq whisper-large-v3")  │
│  • Recommended language model (e.g., "GPT-4.1 mini")        │
│  • Context settings (app, clipboard, selection toggles)      │
│  • Auto-activation rules (optional)                          │
│  • Polish recommendation (ON/OFF)                            │
│  • Example inputs/outputs (so users know what to expect)     │
│                                                              │
│  A recipe does NOT contain:                                  │
│  • API keys (never shared)                                   │
│  • Personal data                                             │
│  • License information                                       │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  HOW RECIPES WORK:                                           │
│                                                              │
│  CREATING:                                                   │
│  1. User creates a mode and configures it                    │
│  2. Clicks "Export as Recipe" → saves a .json file           │
│  3. Can share it anywhere (GitHub, Discord, email, etc.)     │
│                                                              │
│  IMPORTING:                                                  │
│  1. User gets a recipe file (or clicks a link)               │
│  2. Clicks "Import Recipe" on the Dictation page             │
│  3. Kalam creates a new mode from the recipe                 │
│  4. User can customize it further (it's their mode now)      │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  BUILT-IN RECIPES (ship with Kalam):                         │
│                                                              │
│  Kalam ships with a set of starter recipes that users can    │
│  install, customize, or delete. These are NOT locked modes   │
│  — they're just pre-made templates to get started:           │
│                                                              │
│  📧 Email — "Format as professional email with greeting      │
│     and sign-off. Match the formality of the conversation."  │
│  💬 Message — "Keep it casual and concise. Good for chat."   │
│  📝 Notes — "Structure as organized notes with bullet        │
│     points, headings, and key takeaways."                    │
│  ⚡ Command — "Voice commands → actions (notes, tasks,       │
│     search, reminders)."                                     │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  COMMUNITY RECIPE LIBRARY (future):                          │
│                                                              │
│  A public collection of community-created recipes:           │
│  • Hosted on GitHub (simple, free, version-controlled)       │
│  • Or a simple web page on kalam's website                   │
│  • Users can browse, preview, and install with one click     │
│  • Categories: "Work", "Creative", "Developer", "Languages"  │
│  • Ratings and usage counts                                  │
│                                                              │
│  Example community recipes:                                  │
│  • "Slack — Casual" — short, emoji-friendly messages         │
│  • "Legal Brief" — formal legal writing style                │
│  • "Code Review" — formats as code review comments           │
│  • "Spanish Translator" — translates everything to Spanish   │
│  • "Meeting Minutes" — structured meeting notes              │
│  • "Tweet Thread" — formats as a Twitter/X thread            │
│  • "Jira Ticket" — formats as a bug report / user story      │
│                                                              │
│  This is a powerful community flywheel: users create          │
│  recipes → share them → more people use Kalam → more         │
│  recipes get created → Kalam becomes more valuable.          │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### What Is Polish?

Polish is a **feature toggle** (ON/OFF) that applies to any mode. When Polish is ON, the AI includes cleanup instructions **in the same LLM call** as the mode's instructions — not a separate call. One prompt, one response.

Polish absorbs the best features from SuperWhisper's "Super Mode" — the things that make dictation feel intelligent — and makes them available on every mode, not just one special mode.

**Important: Polish + Context + Mode = ONE LLM call.** The AI receives your transcript, any context data, polish instructions, and mode instructions all at once. It processes everything together and returns one clean result. This keeps latency low and API costs minimal.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  ✨ POLISH — What It Does                                    │
│                                                              │
│  GRAMMAR & SPELLING                                          │
│  • Fixes grammar mistakes                                    │
│  • Corrects spelling                                         │
│  • Proper punctuation and capitalization                      │
│  • Context-aware spelling: if Context is ON, Polish reads    │
│    names, terms, and jargon from your screen/clipboard and   │
│    uses them to spell things correctly in your dictation     │
│    (from Super Mode — no need to add words to dictionary)   │
│                                                              │
│  CLEANUP                                                     │
│  • Removes filler words (um, uh, like, you know)             │
│  • Removes false starts ("I mean, actually, what I...")      │
│  • Smart self-correction: if you trip over words or rephrase │
│    mid-sentence, Polish detects the correction and keeps     │
│    only the clean final version (from Super Mode)           │
│  • Extracts what you're REALLY trying to say                 │
│    (if you ramble, it distills your intent)                  │
│                                                              │
│  FORMATTING                                                  │
│  • Converts spoken URLs and emails into proper format        │
│    "john at gmail dot com" → john@gmail.com                  │
│    "w w w dot example dot com" → www.example.com             │
│    (from Super Mode)                                        │
│  • Responds to voice commands:                               │
│    "new list" → starts a bulleted list                       │
│    "new paragraph" → inserts paragraph break                 │
│    "number one... number two..." → numbered list             │
│  • Structures text into readable paragraphs                  │
│  • Adapts formatting to context (if Context is ON)           │
│                                                              │
│  TONE & INTENT                                               │
│  • Preserves your natural voice and meaning                  │
│  • If Context is ON, adapts tone to match the app you're in  │
│    (more formal in Outlook, casual in Slack — from Super)   │
│  • Doesn't add content you didn't say                        │
│  • Doesn't translate or transform (that's the mode's job)    │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  HOW POLISH WORKS TECHNICALLY:                               │
│  Polish is NOT a separate API call. It's additional           │
│  instructions added to the SAME LLM prompt as the mode.      │
│  The AI gets one prompt: "Here's the transcript. Clean it    │
│  up (polish). Apply these mode instructions. Here's the      │
│  context." → One response back. Done.                        │
│                                                              │
│  POLISH WITHOUT CONTEXT (Polish ON, Context OFF):            │
│  Grammar, filler removal, self-correction, formatting,       │
│  URL/email conversion. Good baseline cleanup.                │
│                                                              │
│  POLISH WITH CONTEXT (Polish ON, Context ON):                │
│  All of the above PLUS context-aware spelling, tone          │
│  adaptation, and smarter formatting based on what app        │
│  you're in. This is our equivalent of "Super Mode" —         │
│  but it works on ANY mode, not just one special mode.        │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### The Default Mode

Kalam ships with one built-in mode: **Voice** (the default). This is what Kalam does today — pure transcription, no AI, fastest possible. Polish is OFF by default. Context is OFF by default.

Users can immediately start creating their own modes. We also ship a few **built-in recipes** (not locked modes — just pre-made templates the user can customize or delete). Users can also import community recipes or export their own.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  BUILT-IN (user can edit, rename, or delete these)           │
│                                                              │
│  🎙️ Voice (default — always present, cannot be deleted)      │
│     No AI. Raw transcription. Fastest.                       │
│     Polish: OFF | Context: OFF                               │
│                                                              │
│  ⚡ Command (built-in recipe)                                │
│     Voice commands → actions (notes, tasks, search).         │
│     "New task buy groceries by Friday"                       │
│     "Online search best restaurants nearby"                  │
│     With Context ON, the AI can read your screen to          │
│     create smarter notes/tasks from what you're looking at.  │
│     Polish: OFF | Context: OFF (can turn ON)                 │
│                                                              │
│  📧 Email (built-in recipe)                                  │
│     "Format as a professional email with greeting            │
│      and sign-off. Keep my natural tone."                    │
│     Polish: ON | Context: ON                                 │
│                                                              │
│  💬 Message (built-in recipe)                                │
│     "Keep it casual and concise. Good for chat apps."        │
│     Polish: ON | Context: OFF                                │
│                                                              │
│  📝 Notes (built-in recipe)                                  │
│     "Structure as organized notes with bullet points,        │
│      headings, and key takeaways."                           │
│     Polish: ON | Context: OFF                                │
│                                                              │
│  Users can create unlimited additional modes, import          │
│  community recipes, or export their own to share.            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Where Modes Live in the App

A new **"Dictation"** item appears in the left sidebar, between Overview and History:

```
┌──────────────┐
│  📊 Overview │
│  🎙️ Dictation│  ← NEW
│  🕐 History  │
│  📓 Notes    │
│  ✅ Tasks    │
│  🔔 Reminders│
│  📖 Dictionary│
│  📝 Snippets │
│              │
│  ⚙️ Settings │
└──────────────┘
```

### The Dictation Page — What You See

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  DICTATION                                                   │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ACTIVE MODE                                           │  │
│  │                                                        │  │
│  │  📧 Email                            [Change Mode ▾]  │  │
│  │  "Format as professional email with greeting/sign-off" │  │
│  │                                                        │  │
│  │  Voice model: Groq (cloud)                             │  │
│  │  Language model: GPT-4.1 mini                          │  │
│  │  Polish: ✨ ON    Context: 🔍 ON (app + clipboard)     │  │
│  │  Hotkey: Ctrl + Win                                    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  GLOBAL TOGGLES                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ✨ Polish .............. [ON ●]  (applies to all AI   │  │
│  │                                    modes globally)     │  │
│  │  🔍 Context Awareness ... [ON ●]  (can override       │  │
│  │                                    per mode)           │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  YOUR MODES                                                  │
│                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ 🎙️ Voice │ │ ⚡Command│ │ 📧 Email │ │ 💬Message│       │
│  │          │ │          │ │  ●active │ │          │       │
│  │ Default  │ │          │ │          │ │          │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
│                                                              │
│  ┌──────────┐ ┌──────────┐                                   │
│  │ 📝 Notes │ │ 🇫🇷French│                                   │
│  │          │ │          │                                   │
│  │          │ │ (custom) │                                   │
│  └──────────┘ └──────────┘                                   │
│                                                              │
│  [+ Create New Mode]  [📥 Import Recipe]  [🌐 Browse Recipes] │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  MODE SETTINGS (for selected mode: 📧 Email)                 │
│                                                              │
│  Name ................. [Email                      ]        │
│  Voice Model .......... [Groq (cloud)        ▾]              │
│  Language Model ....... [GPT-4.1 mini        ▾]              │
│                                                              │
│  AI Instructions                                             │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ Format as a professional email. Add an appropriate     │  │
│  │ greeting and sign-off. Keep my natural tone but make   │  │
│  │ it sound professional. If context shows a reply        │  │
│  │ thread, match the formality of the conversation.       │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  Context Awareness (override for this mode)                  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ☑ Read active app content                             │  │
│  │  ☑ Read clipboard                                      │  │
│  │  ☐ Read selected text                                  │  │
│  │  ☑ Include system info (date, time, username)          │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  Auto-Activate Rules                                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  When "Outlook" is active → switch to this mode        │  │
│  │  When "Gmail" is active → switch to this mode          │  │
│  │  [+ Add Rule]                                          │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  [📤 Export as Recipe]  [🗑️ Delete Mode]                     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### How You Switch Modes

Six ways, from quickest to most deliberate:

1. **Combo hotkey** — press a key combination to cycle through modes (e.g., Ctrl+Shift+M cycles to next mode)
2. **From the overlay (dormant state)** — hover to reveal mode switcher dropdown, click to change
3. **From the overlay (mini state)** — hover to reveal mode switcher
4. **From the overlay (full state)** — mode picker is always visible
5. **Dictation page** — click a mode card
6. **Auto-activation** — Kalam switches automatically when you open certain apps (e.g., switch to "Email" when Outlook is active, back to "Voice" when VS Code is active)

### What Moves Out of Settings

Some things currently buried in Settings make more sense on the Dictation page:

| Currently in Settings | Moves to Dictation Page |
|---|---|
| STT mode (Cloud/Local/Hybrid) | Per-mode voice model selection |
| Command mode config (provider, keys, models) | Shared LLM config (used by all AI-powered modes) |
| Formatting options (filler removal, voice commands) | Polish toggle + per-mode formatting |

Settings keeps: audio device, global hotkeys, privacy, notifications, overlay appearance, logging, about.

---

## Part 2: Model Library — Curated & Pre-Configured

### The Goal: No Googling for Base URLs

The problem with "bring your own key" is that users have to figure out: *Where do I get a key? What's the base URL? Which model name do I type?* That's too much friction. The Model Library solves this by **curating the models we recommend** and **pre-configuring everything except the API key**.

### How It Works

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  MODEL LIBRARY — CURATED MODELS                              │
│                                                              │
│  Kalam ships with a curated list of recommended models.      │
│  Each model card has everything pre-configured:              │
│  • Provider name and logo                                    │
│  • Model name and ID (pre-filled)                            │
│  • Base URL (pre-filled)                                     │
│  • What it covers (STT, LLM, or both)                        │
│  • Speed / quality rating                                    │
│  • Cost estimate (free tier? pay-per-use?)                   │
│  • "Get API Key" link (opens the provider's console)         │
│                                                              │
│  The user's ONLY job: paste their API key. Done.             │
│  No searching for base URLs. No guessing model names.        │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  CURATED VOICE MODELS (STT):                                 │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ⚡ Groq — whisper-large-v3-turbo          ⭐ FASTEST  │  │
│  │  Speed: ●●●●● | Quality: ●●●●○ | Cost: Free tier      │  │
│  │  Covers: STT + LLM (same key)                         │  │
│  │  [Get API Key ↗] [Paste Key: ________________]         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🤖 OpenAI — whisper-1                                 │  │
│  │  Speed: ●●●○○ | Quality: ●●●●● | Cost: $0.006/min     │  │
│  │  Covers: STT + LLM (same key)                         │  │
│  │  [Get API Key ↗] [Paste Key: ________________]         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  📦 SenseVoice (Local)                     🔒 PRIVATE  │  │
│  │  Speed: ●●●●○ | Quality: ●●●○○ | Cost: Free           │  │
│  │  No API key needed. Runs on your computer.             │  │
│  │  [Download (245 MB)]                                   │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  📦 Whisper Base (Local)                   🔒 PRIVATE  │  │
│  │  Speed: ●●●○○ | Quality: ●●○○○ | Cost: Free           │  │
│  │  No API key needed. Smaller, faster, less accurate.    │  │
│  │  [Download (75 MB)]                                    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  CURATED LANGUAGE MODELS (LLM):                              │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ⚡ Groq — llama-3.3-70b                   ⭐ FASTEST  │  │
│  │  Speed: ●●●●● | Quality: ●●●○○ | Cost: Free tier      │  │
│  │  Same key as Groq STT above (auto-linked)              │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🤖 OpenAI — GPT-4.1 mini                 ⭐ BALANCED  │  │
│  │  Speed: ●●●●○ | Quality: ●●●●○ | Cost: ~$0.40/M tok   │  │
│  │  Same key as OpenAI STT above (auto-linked)            │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🧠 Anthropic — Claude 4.5 Sonnet         ⭐ SMARTEST  │  │
│  │  Speed: ●●●○○ | Quality: ●●●●● | Cost: ~$3/M tok      │  │
│  │  LLM only — needs separate STT provider               │  │
│  │  [Get API Key ↗] [Paste Key: ________________]         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  💎 Google Gemini — Gemini 2.5 Flash                   │  │
│  │  Speed: ●●●●○ | Quality: ●●●●○ | Cost: Free tier      │  │
│  │  LLM only — needs separate STT provider               │  │
│  │  [Get API Key ↗] [Paste Key: ________________]         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🌐 OpenRouter — (access to 100+ models)               │  │
│  │  Speed: varies | Quality: varies | Cost: varies        │  │
│  │  LLM only — needs separate STT provider               │  │
│  │  [Get API Key ↗] [Paste Key: ________________]         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  🔧 CUSTOM MODEL                                             │
│                                                              │
│  For advanced users who want to use a provider or model      │
│  not in the curated list:                                    │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Provider Name: [________________]                     │  │
│  │  Base URL:      [________________]                     │  │
│  │  Model ID:      [________________]                     │  │
│  │  API Key:       [________________]                     │  │
│  │  Type:          [STT ▾] or [LLM ▾] or [Both ▾]        │  │
│  │  [Test Connection]                                     │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  This covers: self-hosted models (Ollama, vLLM), niche       │
│  providers, enterprise endpoints, or future providers        │
│  that Kalam doesn't know about yet.                          │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Smart Key Linking

When a provider covers both STT and LLM (like Groq or OpenAI), entering the API key once automatically makes it available for both voice models and language models. No need to enter it twice.

### Recommended Setups (shown in onboarding wizard)

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  EASIEST SETUP (one key does everything):                    │
│                                                              │
│  Groq API key → covers both STT (whisper-large-v3-turbo)    │
│  and LLM (llama-3.3-70b). Fastest latency. Free tier        │
│  available. One key, done.                                   │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  BEST QUALITY (two keys):                                    │
│                                                              │
│  Groq for STT (fastest transcription)                        │
│  + Anthropic or OpenAI for LLM (best AI quality)             │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  FULLY OFFLINE (no keys needed):                             │
│                                                              │
│  SenseVoice or Whisper local model for STT.                  │
│  No LLM — Voice Mode only (raw transcription).              │
│  100% free, 100% private, no internet required.              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### First-Run Setup — Making AI Provider Setup a Breeze

There are many AI providers out there. Some offer STT, some offer LLM, some offer both. Some need one key, some need two. This can be confusing. The setup experience needs to be dead simple — especially since users might not know what "STT" or "LLM" even means.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  FIRST-RUN WIZARD — "How do you want to use Kalam?"         │
│                                                              │
│  When the user first opens Kalam, a friendly wizard walks    │
│  them through setup. Three paths, plain language:            │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                                        │  │
│  │  🔒 FULLY OFFLINE — No internet needed                 │  │
│  │  "I want maximum privacy. Everything stays on my PC."  │  │
│  │                                                        │  │
│  │  → Downloads a local voice model (SenseVoice or        │  │
│  │    Whisper). No API keys needed. No sign-up.           │  │
│  │  → You get: Voice Mode (pure transcription).           │  │
│  │  → Limitations: No AI features (Polish, modes, etc.)   │  │
│  │    unless you add a local LLM later.                   │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                                        │  │
│  │  ⚡ ONE KEY DOES EVERYTHING — Fastest setup             │  │
│  │  "I want the easiest setup. One key, done."            │  │
│  │                                                        │  │
│  │  → Recommends Groq (free tier available).              │  │
│  │    One API key covers both voice transcription AND      │  │
│  │    AI processing.                                      │  │
│  │  → Step-by-step: "Go to console.groq.com → Create     │  │
│  │    account → Copy your API key → Paste here"           │  │
│  │  → Test connection button: ✅ "Connected! You're       │  │
│  │    ready to go."                                       │  │
│  │  → You get: Everything. Voice, Polish, modes, context. │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                                        │  │
│  │  🎯 BEST QUALITY — Two keys, best results              │  │
│  │  "I want the highest quality transcription and AI."    │  │
│  │                                                        │  │
│  │  → Recommends Groq for voice (fastest transcription)   │  │
│  │    + OpenAI or Anthropic for AI (best quality).        │  │
│  │  → Walks through getting both keys step by step.       │  │
│  │  → Test connection for each: ✅ ✅                      │  │
│  │  → You get: Everything, with premium AI quality.       │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                                        │  │
│  │  🔧 I ALREADY HAVE API KEYS — Let me configure         │  │
│  │  "I know what I'm doing. Let me pick my providers."    │  │
│  │                                                        │  │
│  │  → Goes straight to the provider settings page.        │  │
│  │  → User picks their STT provider and LLM provider.    │  │
│  │  → Full control, no hand-holding.                      │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  The wizard can be re-run anytime from Settings.             │
│  Users can always change providers later.                    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Provider Cards in Settings — Clear at a Glance

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  AI PROVIDERS                                                │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ⚡ Groq                              ✅ Connected     │  │
│  │  Covers: Voice (STT) + AI (LLM)                       │  │
│  │  Key: sk-••••••••••••••3f2a                            │  │
│  │  Models: whisper-large-v3-turbo, llama-3.3-70b         │  │
│  │  [Test Connection] [Edit Key] [Remove]                 │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🧠 Anthropic                         ✅ Connected     │  │
│  │  Covers: AI (LLM) only                                │  │
│  │  Key: sk-••••••••••••••8b1c                            │  │
│  │  Models: claude-4.5-sonnet, claude-4-sonnet            │  │
│  │  [Test Connection] [Edit Key] [Remove]                 │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🤖 OpenAI                            ○ Not connected  │  │
│  │  Covers: Voice (STT) + AI (LLM)                       │  │
│  │  [Add API Key]                                         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🌐 OpenRouter                        ○ Not connected  │  │
│  │  Covers: AI (LLM) only                                │  │
│  │  [Add API Key]                                         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  💎 Google Gemini                     ○ Not connected  │  │
│  │  Covers: AI (LLM) only                                │  │
│  │  [Add API Key]                                         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  📦 Local Models                      ✅ 1 installed   │  │
│  │  Covers: Voice (STT) — no key needed                   │  │
│  │  Installed: SenseVoice (245 MB)                        │  │
│  │  [Manage Local Models]                                 │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  Each card clearly shows:                                    │
│  • What the provider covers (STT, LLM, or both)             │
│  • Connection status (green ✅ or grey ○)                    │
│  • Which models are available                                │
│  • Quick actions (test, edit, remove)                        │
│                                                              │
│  [Re-run Setup Wizard]                                       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Part 3: Context Awareness

### What Is Context Awareness?

When you dictate with context awareness turned on, Kalam doesn't just listen to your voice — it also **looks at your screen**. It reads:

1. **The app you're in** — what's in the text field you're typing into
2. **Text you've highlighted** — any selected text in the active window
3. **Your clipboard** — whatever you last copied (within a few seconds)

This information gets sent to the AI along with your speech, so the AI can give you smarter results.

### How It Works — A Real Example

**Without context awareness:**
> You say: "Send a message to John about the project deadline"
> AI output: "Send a message to John about the project deadline."
> (Just cleaned-up transcription. The AI doesn't know who John is or what project.)

**With context awareness:**
> You're in Slack, in a conversation with "John Peterson"
> You have an email open mentioning "Project Atlas deadline March 31"
> You say: "Send a message to John about the project deadline"
> AI output: "Hey John, just wanted to check in on the Project Atlas deadline. Are we still on track for March 31?"
> (The AI saw the Slack conversation and the email context, and wrote something relevant.)

### The Three Context Types

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  📱 APP CONTEXT                                             │
│  ──────────────                                             │
│  Reads text from the app you're currently using.            │
│  If you're in Notepad, it reads what's in Notepad.          │
│  If you're in Chrome, it reads the page content.            │
│                                                             │
│  Also includes: current date/time, your username.           │
│                                                             │
│  When: Captured right after you finish speaking,            │
│  before the AI processes your text.                         │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ✂️ SELECTED TEXT                                            │
│  ────────────────                                           │
│  Reads whatever text you've highlighted/selected.           │
│                                                             │
│  This is what enables VOICE EDITING:                        │
│  Highlight a paragraph → say "make this more formal"        │
│  → AI rewrites it → Kalam replaces the selection.           │
│                                                             │
│  When: Captured the moment you press the hotkey.            │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  📋 CLIPBOARD                                               │
│  ─────────────                                              │
│  Reads the last thing you copied (Ctrl+C).                  │
│                                                             │
│  Useful for: Copy someone's name before dictating,          │
│  and the AI will spell it correctly in your text.           │
│  Like temporary vocabulary hints.                           │
│                                                             │
│  When: Only if you copied something in the last             │
│  few seconds before pressing the hotkey.                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Voice-Activated Editing — The Killer Feature

This deserves its own spotlight because it changes what dictation IS.

Today, dictation = input. You speak, text appears. One direction.

With voice-activated editing, dictation = **input AND editing**:

```
BEFORE (today):
  1. You see a paragraph you want to change
  2. You select it
  3. You delete it
  4. You type the new version
  — or —
  5. You copy it to ChatGPT
  6. You ask ChatGPT to rewrite it
  7. You copy the result
  8. You paste it back

AFTER (with this upgrade):
  1. You highlight the paragraph
  2. You press the hotkey
  3. You say "make this more concise"
  4. Done. The text is replaced.
```

Examples of what you can say:
- "Make this sound more professional"
- "Turn this into bullet points"
- "Translate this to French"
- "Fix the grammar"
- "Make this shorter"
- "Add more detail about the deadline"

### Privacy — How Sensitive Apps Work in the New System

Kalam already detects sensitive applications (banking apps, password managers, etc.) and forces local-only STT. With the new modes + context system, this privacy enforcement **cascades through everything**:

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  SENSITIVE APP DETECTED — PRIVACY LOCKDOWN                   │
│                                                              │
│  When Kalam detects a sensitive app is active:               │
│                                                              │
│  1. STT → ForceLocal (already exists today)                  │
│     No audio leaves your computer.                           │
│                                                              │
│  2. Context Awareness → FORCED OFF                           │
│     Even if the mode has Context ON, it's overridden.        │
│     No screen content, clipboard, or selected text is        │
│     read or sent anywhere.                                   │
│                                                              │
│  3. Polish → FALLS BACK to local-only processing             │
│     • Regex-based filler word removal (works offline)        │
│     • Basic formatting rules (works offline)                 │
│     • Voice commands (new line, new paragraph — offline)     │
│     • NO LLM call — nothing sent to cloud                   │
│                                                              │
│  4. Mode AI instructions → DISABLED                          │
│     The mode's LLM instructions don't run.                   │
│     You get clean transcription with basic formatting only.  │
│                                                              │
│  5. Overlay → Shows 🔒 lock icon                             │
│     "Sensitive app detected — privacy mode active.           │
│      Using local STT only. No data leaves your computer."   │
│                                                              │
│  RESULT: Maximum privacy. The user's mode settings are       │
│  remembered but temporarily overridden. When they switch     │
│  to a non-sensitive app, everything goes back to normal.     │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  FUTURE: LOCAL LLM PROCESSING                                │
│                                                              │
│  For users who want context awareness AND privacy, the       │
│  future path is local LLM processing (Ollama, llama.cpp).   │
│  With a local LLM, even sensitive apps could use Polish      │
│  and Context — because nothing leaves the computer.          │
│  SuperWhisper can't do this (cloud LLM only).                │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  GENERAL PRIVACY TRADE-OFF:                                  │
│                                                              │
│  Context OFF (default for Voice mode):                       │
│  Nothing is read. Pure transcription. Maximum privacy.       │
│                                                              │
│  Context ON (opt-in for AI modes):                           │
│  Screen content is sent to whatever LLM provider you've      │
│  configured. The user chooses their provider and trusts      │
│  them with that data. Kalam never stores or forwards it.     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Part 4: The Overlay / Pill — Dormant, Mini & Full States

### The Three States

The overlay (floating pill) now has three distinct states. The user controls which states they want and can toggle between them.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  THREE OVERLAY STATES:                                       │
│                                                              │
│  1. DORMANT — idle, not recording, just sitting there        │
│     (user chooses in Settings whether this is always         │
│      visible or hidden)                                      │
│                                                              │
│  2. MINI — compact pill during recording/processing          │
│     (small waveform, mode name, essential controls)          │
│                                                              │
│  3. FULL — expanded view during recording/processing         │
│     (context panel, Polish toggle, pipeline stages)          │
│                                                              │
│  User sets their preferred ACTIVE state (mini or full)       │
│  in Settings or by toggling within the pill itself.          │
│  When recording starts, the pill enters their chosen state.  │
│  When recording ends and processing completes, it returns    │
│  to dormant (if always-visible) or hides.                    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Dormant State (Idle — Always Visible or Hidden)

The dormant state is the pill when nothing is happening. The user can toggle in Settings whether the dormant pill stays visible at all times or hides when not recording.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  DORMANT STATE — IDLE (not recording)                        │
│                                                              │
│  ┌──────────────────────────┐                                │
│  │  📧 Email            🎙️  │                                │
│  └──────────────────────────┘                                │
│   Shows: active mode name + mic icon                         │
│   Small, unobtrusive. Just tells you what mode is active.    │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  DORMANT STATE — HOVER (controls appear)                     │
│                                                              │
│  ┌─────────────────────────────────────────────────┐         │
│  │  📧 Email  [▾ Mode] [📋 Copy] [● Rec] [⬜ ↗]  │         │
│  └─────────────────────────────────────────────────┘         │
│                                                              │
│  [▾ Mode]  = dropdown to switch modes                        │
│  [📋 Copy] = copy last transcription to clipboard            │
│  [● Rec]   = click to start recording                        │
│  [⬜ ↗]    = expand to full state (for next recording)       │
│                                                              │
│  Right-click on dormant state shows:                         │
│  • Switch Mode → (submenu with all modes)                    │
│  • Copy Last Transcription                                   │
│  • Open Dictation Page                                       │
│  • Open History                                              │
│  • Open Settings                                             │
│  • Set Active State → Mini / Full                            │
│  • Hide Pill (if always-visible is on)                       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Mini State (Compact — During Recording/Processing)

The mini state is the compact active pill. It appears when recording starts (if the user's preferred active state is "mini").

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  MINI STATE — RECORDING                                      │
│                                                              │
│  ┌──────────────────────────────────────┐                    │
│  │  📧 Email  ≋≋≋≋≋≋≋≋≋≋≋≋  ■ Stop    │                    │
│  └──────────────────────────────────────┘                    │
│   Shows: mode name + waveform + stop button                  │
│   Hover also reveals [⬜ ↗] expand button                    │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  MINI STATE — PROCESSING                                     │
│                                                              │
│  ┌──────────────────────────────────────┐                    │
│  │  📧 Email  ◐ Processing...          │                    │
│  └──────────────────────────────────────┘                    │
│   Color-coded dot: yellow = transcribing, blue = AI          │
│   processing, green = done                                   │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  MINI STATE — CONTEXT INDICATORS                             │
│                                                              │
│  When context awareness is ON and items are captured:        │
│                                                              │
│  ┌──────────────────────────────────────┐                    │
│  │  📧 Email  ≋≋≋≋≋≋≋≋  📋 ✂️  ■ Stop  │                    │
│  └──────────────────────────────────────┘                    │
│                                                              │
│  📋 = clipboard content was captured (lights up)             │
│  ✂️ = selected text was captured (lights up)                  │
│  These icons only appear when context is ON for the mode.    │
│  They light up/animate when content is actually captured.    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Full State (Expanded — Rich Information)

The full state is a larger overlay that shows everything happening during dictation. The user can see context, switch modes, toggle polish, and more — all without opening the main window.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  FULL STATE — RECORDING                                      │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                            [🔽 Mini]  │  │
│  │                                                        │  │
│  │  📧 Email                                              │  │
│  │  ✨ Polish: ON                                         │  │
│  │                                                        │  │
│  │  ≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋  │  │
│  │                                                        │  │
│  │  CONTEXT                                               │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │  📱 App: Outlook — reading "RE: Project Atlas"  │  │  │
│  │  │  📋 Clipboard: "John Peterson" (captured ✓)      │  │  │
│  │  │  ✂️ Selected: (none)                              │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐            │  │
│  │  │ ▾ Mode   │  │ ✨Polish │  │ ✕ Cancel │            │  │
│  │  │  Email   │  │   ON     │  │          │            │  │
│  │  └──────────┘  └──────────┘  └──────────┘            │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  What you see:                                               │
│  • Active mode name + icon (large, prominent)                │
│  • Polish status (ON/OFF — clickable to toggle)              │
│  • Full waveform visualization                               │
│  • Context panel showing exactly what the AI is reading:     │
│    - Which app and what content it captured                  │
│    - Whether clipboard was captured (and a preview)          │
│    - Whether selected text was captured (and a preview)      │
│  • Mode switcher dropdown                                    │
│  • Polish toggle (quick on/off)                              │
│  • Cancel button                                             │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  FULL STATE — PROCESSING                                     │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                            [🔽 Mini]  │  │
│  │                                                        │  │
│  │  📧 Email                                              │  │
│  │  ✨ Polish: ON                                         │  │
│  │                                                        │  │
│  │  ◐ Processing with GPT-4.1 mini...                     │  │
│  │                                                        │  │
│  │  CONTEXT SENT TO AI                                    │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │  📱 App: Outlook — "RE: Project Atlas"          │  │  │
│  │  │  📋 Clipboard: "John Peterson" ✓                 │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                        │  │
│  │  Status: ◐ Transcribing → ◐ AI Processing → ✓ Done     │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  Shows the pipeline stages as they happen:                   │
│  1. "Transcribing..." (STT — one API call)                   │
│  2. "Processing..." (one LLM call: polish + context + mode)  │
│  3. "Done ✓" (text injected)                                 │
│                                                              │
│  Only 2 stages max. If Voice Mode with no Polish/AI:         │
│  1. "Transcribing..." → 2. "Done ✓" (no LLM call at all)    │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  FULL STATE — SENSITIVE APP DETECTED                         │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                            [🔽 Mini]  │  │
│  │                                                        │  │
│  │  🔒 Voice (ForceLocal)                                 │  │
│  │  Sensitive app detected — using local STT only.        │  │
│  │  No data leaves your computer.                         │  │
│  │                                                        │  │
│  │  ≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋  │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### How SuperWhisper Does It (For Reference)

SuperWhisper's recording window has these elements we're drawing inspiration from:

| SuperWhisper Element | What It Does | Kalam Equivalent |
|---|---|---|
| Resize toggle | Switch between main and mini views | Same — click to toggle mini/full |
| Audio waveform | Real-time mic feedback | Same (already exists in Kalam) |
| Status indicator dot | Yellow=loading, Blue=processing, Green=done | Same — color-coded status |
| Mode display area | Shows active mode + keyboard shortcut; in Super Mode shows app/website context | Mode name + context panel (more detailed) |
| Context capture indicator | Lights up when clipboard or selected text is captured | Context icons (📋 ✂️) that light up on capture |
| Stop/Cancel buttons | End recording or cancel | Same |
| Mini window hover controls | Change Mode, Start Recording, Expand | Same three controls |
| Mini window right-click | Expand, Settings, History | Same + Dictation Page |
| Mini window always-active option | Stays visible when idle | Same (configurable) |

**Where Kalam goes further than SuperWhisper:**
- **Context panel in full state** — shows exactly what the AI is reading (app name, clipboard preview, selected text preview), not just an indicator light
- **Polish toggle in overlay** — quick on/off without opening settings
- **Pipeline stage display** — shows "Transcribing → Processing → Done" so the user knows exactly what's happening (only 2 stages max — one STT call, one LLM call)
- **Mode switcher in both states** — SuperWhisper only shows mode switching on hover in mini; Kalam shows it in full state too

---

## Part 5: Kalam Pro — The Business Model

### The Core Philosophy

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  KALAM'S PROMISE:                                            │
│                                                              │
│  "Dictation is free. Always. No limits. No catch."           │
│                                                              │
│  You can download Kalam, set it up with a local model,       │
│  and dictate forever without paying anyone anything.          │
│                                                              │
│  Or bring your own API key for cloud STT, and dictate         │
│  with the best models — still free. You pay your provider    │
│  directly, Kalam takes nothing.                              │
│                                                              │
│  WHAT COSTS MONEY:                                           │
│                                                              │
│  Pro features that go beyond basic dictation.                │
│  Polish, custom modes, context awareness, voice editing.     │
│  The "smart" stuff. Unlocked with a small subscription.      │
│                                                              │
│  BUT: You still bring your own API keys.                     │
│  Kalam doesn't serve ANY AI services.                        │
│  Kalam never pays for your usage. You control your costs.    │
│  The subscription pays for the PRO FEATURES, not compute.    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### The BYOK Reality — What Users Actually Pay For

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  WHAT THE USER PAYS:                                         │
│                                                              │
│  1. Their API provider (for STT + LLM)                       │
│     • Some providers use ONE key for both:                   │
│       Groq → same key for STT (whisper) and LLM (llama)     │
│       OpenAI → same key for STT (whisper) and LLM (gpt)     │
│     • Others are LLM-only (need a separate STT key):        │
│       Anthropic, Gemini, OpenRouter → LLM only              │
│     • Or use a local model → no key needed at all            │
│                                                              │
│  2. Kalam Pro subscription (for premium features)            │
│     • This is what keeps Kalam development alive             │
│     • You're paying for the SOFTWARE, not the AI             │
│                                                              │
│  WHAT KALAM PAYS:                                            │
│                                                              │
│  Nothing per user. Zero API costs. Zero compute costs.       │
│  The only costs are:                                         │
│  • A license validation service (~$9-15/mo)                  │
│  • Payment platform fees (~5-8% of revenue)                  │
│  • GitHub Releases for auto-updates (free)                   │
│  • Your time building and supporting the app                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Why This Model Works

The biggest headache for AI apps is **paying for user compute**. Every word a user speaks costs money if you're footing the bill for cloud STT and LLM calls. That's why SuperWhisper charges $8.49/month — they need to cover API costs.

Kalam flips this:

- **You never pay for their words.** Users bring their own API keys (BYOK) or use free local models.
- **You charge for the features, not the compute.** The subscription unlocks capabilities — the user pays their own provider for actual usage.
- **This means your costs stay near zero** regardless of how much users dictate. No surprise bills. No usage caps. No throttling.
- **This is why you can charge less than everyone else** and still be sustainable.

### The Two Tiers

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │                                                      │    │
│  │  🆓  KALAM FREE                                     │    │
│  │  ──────────────                                      │    │
│  │                                                      │    │
│  │  Everything you need to dictate. No account needed.  │    │
│  │                                                      │    │
│  │  ✅ Voice Mode (pure transcription)                  │    │
│  │  ✅ Local STT models (SenseVoice, Whisper)           │    │
│  │  ✅ Cloud STT with your own key (Groq, OpenAI)      │    │
│  │  ✅ Filler word removal, voice commands              │    │
│  │  ✅ Custom dictionary & formatting rules             │    │
│  │  ✅ Notes, Tasks, Reminders                          │    │
│  │  ✅ Dashboard & analytics                            │    │
│  │  ✅ History with search & export                     │    │
│  │  ✅ Snippets                                         │    │
│  │  ✅ Overlay with waveform                            │    │
│  │  ✅ Privacy enforcement (sensitive app detection)    │    │
│  │  ✅ All hotkey configurations                        │    │
│  │  ✅ Light/Dark theme                                 │    │
│  │  ✅ Auto-updates                                     │    │
│  │                                                      │    │
│  │  Price: $0. Forever.                                 │    │
│  │                                                      │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │                                                      │    │
│  │  ⭐ KALAM PRO                                        │    │
│  │  ────────────                                        │    │
│  │                                                      │    │
│  │  AI-powered features that make dictation smarter.    │    │
│  │  Still BYOK — you bring your own keys for AI.        │    │
│  │                                                      │    │
│  │  Everything in Free, PLUS:                           │    │
│  │                                                      │    │
│  │  ⭐ Polish (AI cleanup on any mode)                  │    │
│  │  ⭐ Custom Modes (your own AI instructions)          │    │
│  │  ⭐ Command Mode (voice → notes/tasks/search)        │    │
│  │  ⭐ Context Awareness (read screen, clipboard)       │    │
│  │  ⭐ Voice-Activated Editing (highlight → speak)      │    │
│  │  ⭐ Auto-Activation Rules (auto-switch per app)      │    │
│  │  ⭐ Enhanced Overlay (full state, context panel)     │    │
│  │  ⭐ Priority support                                 │    │
│  │                                                      │    │
│  │  Price: $2.99/month or $32.89/year (1 month free)     │    │
│  │  You still bring your own API keys.                  │    │
│  │                                                      │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Pricing — What the Market Looks Like

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  COMPETITOR PRICING (March 2026)                             │
│                                                              │
│  SuperWhisper Pro ........... $8.49/mo  ($85/yr)             │
│    → includes cloud STT + LLM compute                       │
│  Spokenly Pro ............... $9.99/mo                       │
│    → includes cloud compute                                 │
│  OpenTypeless Pro ........... $4.99/mo                       │
│    → includes 10 hrs STT + 5M tokens/mo                     │
│  VoiceInk ................... $25–49 one-time (GPLv3)        │
│    → local only, no cloud compute included                  │
│  Voibe ...................... $4.90/mo  ($99 lifetime)        │
│    → local only                                             │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  WHY KALAM CAN CHARGE LESS:                                  │
│                                                              │
│  Those competitors charge $5–10/mo because they PAY for      │
│  your cloud STT and LLM usage. That's expensive.             │
│                                                              │
│  Kalam doesn't pay for any of that. Users bring their own    │
│  keys. So the subscription is nearly pure revenue.           │
│                                                              │
│  At $2.99/mo, Kalam is:                                      │
│  • Cheaper than a single coffee                              │
│  • Less than half of SuperWhisper                            │
│  • Less than OpenTypeless (which includes compute)           │
│  • An absolute no-brainer for anyone who loves the app       │
│                                                              │
│  And the user is ALREADY paying their API provider           │
│  (typically $0–5/mo for normal dictation usage with Groq     │
│  or OpenAI). Adding $2.99 on top is trivial.                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Recommended Pricing

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  KALAM PRO PRICING                                           │
│                                                              │
│  Monthly:  $2.99/mo                                          │
│  Yearly:   $32.89/yr  (1 month free — pay for 11 months)    │
│                        Works out to ~$2.74/mo                │
│                                                              │
│  No lifetime plan. Recurring revenue keeps development       │
│  alive and features coming.                                  │
│                                                              │
│  Why $2.99:                                                  │
│  • Users are ALREADY paying for their own API keys           │
│    (STT key + possibly a separate LLM key)                  │
│  • Adding $2.99 on top of that should feel like nothing      │
│  • It's the "I won't even think about it" price             │
│  • Cheaper than every single competitor                      │
│  • At this price, conversion rate should be higher           │
│    (more people subscribe = more total revenue)             │
│  • You can always raise it later as features grow            │
│                                                              │
│  Why no lifetime plan:                                       │
│  • Lifetime plans generate upfront cash but kill recurring   │
│    revenue — the thing that actually funds development       │
│  • A $60 lifetime = 20 months of monthly. After that, the   │
│    user costs you support time with zero revenue.            │
│  • Better to keep everyone on recurring and earn trust       │
│    through consistent updates and value.                     │
│                                                              │
│  Why 1 month free (not 30% off) for yearly:                  │
│  • Simple to understand: "pay for 11, get 12"               │
│  • Feels like a gift, not a discount                         │
│  • Encourages yearly commitment without deep discounting     │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  EXAMPLE REVENUE (at $2.99/mo):                              │
│                                                              │
│  If 5% of free users convert to Pro:                         │
│                                                              │
│  1,000 free users → 50 Pro  → ~$150/mo  → ~$1,800/yr        │
│  5,000 free users → 250 Pro → ~$750/mo  → ~$9,000/yr        │
│  10,000 free users → 500 Pro → ~$1,500/mo → ~$18,000/yr     │
│  50,000 free users → 2,500 Pro → ~$7,500/mo → ~$90,000/yr   │
│                                                              │
│  At $2.99 you might see HIGHER conversion (7-10%) because    │
│  the price is so low. At 8% conversion:                      │
│                                                              │
│  10,000 free users → 800 Pro → ~$2,400/mo → ~$28,800/yr     │
│  50,000 free users → 4,000 Pro → ~$12,000/mo → ~$144,000/yr │
│                                                              │
│  The yearly plan ($32.89) is the real money maker —          │
│  people commit for a year and retention is much higher.      │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### The User Journey — How Someone Goes from Free to Pro

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  STEP 1: DISCOVER                                            │
│  User finds Kalam. Downloads it. It's free and open source.  │
│  No account. No credit card. No sign-up wall.                │
│                                                              │
│         ↓                                                    │
│                                                              │
│  STEP 2: USE FREE                                            │
│  Sets up a local model (or adds their Groq key for both      │
│  STT and LLM with a single key). Dictates in Voice Mode.     │
│  Uses notes, tasks, dashboard. Feels like a full app.        │
│                                                              │
│         ↓                                                    │
│                                                              │
│  STEP 3: SEE THE PREMIUM                                     │
│  On the Dictation page, they see Command mode and the        │
│  Polish toggle — but they have a small "⭐ Pro" badge.       │
│  Clicking shows a friendly explanation of what it does        │
│  and a "Try Pro" or "Unlock with Kalam Pro" button.          │
│                                                              │
│  NOT a hard paywall. NOT a nag screen. Just a clear          │
│  "this is what Pro gives you" moment.                        │
│                                                              │
│         ↓                                                    │
│                                                              │
│  STEP 4: TRY IT (14-day free trial)                          │
│  User agrees to terms → gets 14 days of Pro features.        │
│  No credit card. No email. No account. Just agree and go.    │
│  They experience Polish, custom modes, context awareness.    │
│  After 14 days, Pro features gracefully lock again.          │
│                                                              │
│         ↓                                                    │
│                                                              │
│  STEP 5: SUBSCRIBE                                           │
│  User clicks "Upgrade to Pro" → payment page.                │
│  $2.99/mo or $32.89/yr (1 month free). Done.                 │
│  License key activates Pro features instantly.               │
│                                                              │
│         ↓                                                    │
│                                                              │
│  STEP 6: STAY                                                │
│  Pro features keep working. Free features never go away.     │
│  If they cancel, they drop back to Free — no data lost,      │
│  no punishment, no guilt. They can re-subscribe anytime.     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Licensing — FSL (Functional Source License)

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  SWITCHING FROM CURRENT LICENSE TO FSL                        │
│                                                              │
│  Kalam's current license (MIT noncommercial + commercial     │
│  by permission) is being replaced with the Functional        │
│  Source License (FSL). This is happening NOW, not later.     │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  WHAT FSL MEANS:                                             │
│                                                              │
│  ✅ Anyone can read, modify, and use the code                │
│  ✅ Anyone can contribute (PRs, forks, patches)              │
│  ✅ Businesses can use Kalam internally — no permission      │
│     needed (more permissive than current license!)           │
│  ✅ Individuals, freelancers, companies — all free to use    │
│  ❌ Nobody can take the code and build a COMPETING           │
│     dictation/voice product                                  │
│  ✅ After 2 years, code auto-converts to Apache 2.0          │
│     (fully open, no restrictions)                            │
│                                                              │
│  WHO USES FSL:                                               │
│  • Sentry (error tracking)                                   │
│  • CockroachDB (database)                                    │
│  • MariaDB (database)                                        │
│  • HashiCorp (Terraform, Vault, etc.)                        │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  WHY FSL IS BETTER THAN THE CURRENT LICENSE:                 │
│                                                              │
│  Current (MIT noncommercial + commercial by permission):     │
│  • Unusual, not well-known — companies aren't sure if        │
│    they can use it                                           │
│  • "Commercial use requires permission" is vague             │
│  • Hard to enforce — you can't detect violations             │
│  • Scares away legitimate business users                     │
│                                                              │
│  FSL:                                                        │
│  • Standard, well-known license — legal teams recognize it   │
│  • Clear rule: "don't build a competing product"             │
│  • More permissive for businesses using Kalam (no need to    │
│    ask permission for internal use)                          │
│  • More protective against actual competitors                │
│  • 2-year auto-convert to Apache 2.0 builds trust            │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  "CAN'T SOMEONE JUST FORK AND REMOVE THE LICENSE CHECK?"     │
│                                                              │
│  Yes, if they compile from source. And that's fine:          │
│                                                              │
│  • The check lives in Rust (not trivially editable)          │
│  • FSL legally prohibits building a competing product        │
│  • Someone who compiles from source was never going to pay   │
│  • Your paying customers value convenience, updates,         │
│    and support — not just the code                           │
│  • The majority of users will never do this                  │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  ENFORCEMENT = FEATURE GATING (not legal threats)            │
│                                                              │
│  Don't rely on license enforcement for revenue.              │
│  Rely on FEATURE GATING instead:                             │
│                                                              │
│  • The compiled app has a license key check in Rust          │
│  • Pro features are locked unless a valid key is present     │
│  • Everyone pays the same $2.99/mo for Pro features          │
│  • No separate "commercial license" to negotiate             │
│  • FSL is the legal backstop if someone does compete         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### The Website + Service Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  KALAM'S ONLINE PRESENCE — TWO THINGS:                       │
│                                                              │
│  1. THE WEBSITE (public-facing)                              │
│     • Marketing site, download links, documentation          │
│     • Recipe library (browse community recipes)              │
│     • Stripe checkout for Pro subscriptions                  │
│     • Customer portal (manage subscription, billing)         │
│                                                              │
│  2. THE SERVICE (backend API)                                │
│     • License key validation (is this user Pro?)             │
│     • Sync API (tasks, notes, modes, settings across PCs)   │
│     • Stripe webhooks (subscription created/cancelled/etc.)  │
│     • Recipe submission API (future)                         │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  WHERE IT LIVES:                                             │
│                                                              │
│  Cloudflare — everything on one platform:                    │
│  • Cloudflare Pages → website (static site, fast, free)      │
│  • Cloudflare Workers → API (serverless, scales to zero)     │
│  • Cloudflare D1 → database (SQLite at the edge, cheap)      │
│  • Cloudflare KV → fast key-value store (license cache)      │
│                                                              │
│  Why Cloudflare:                                             │
│  • Generous free tier (good for starting out)                │
│  • Scales automatically (no server management)               │
│  • Global edge network (fast for users everywhere)           │
│  • All-in-one: no need to juggle AWS + Vercel + PlanetScale  │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  REPO: PRIVATE                                               │
│                                                              │
│  The website + service code lives in a PRIVATE repo.         │
│  The Kalam app (FSL) is the open-source part.                │
│  The service is business infrastructure — not open source.   │
│                                                              │
│  Why private:                                                │
│  • Contains Stripe webhook handling and license logic         │
│  • Security: vulnerabilities in the backend shouldn't be     │
│    visible to attackers                                      │
│  • Competitors can't clone your entire service               │
│  • This is standard: every open-core company does it         │
│    (app = open, service = private)                           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Payments — Stripe (No-Code / Low-Code First)

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  STRIPE — USE AS MUCH NO-CODE AS POSSIBLE                    │
│                                                              │
│  Stripe has built-in tools that handle most of what we       │
│  need without writing custom code:                           │
│                                                              │
│  STRIPE CHECKOUT (no-code)                                   │
│  • Hosted payment page — Stripe handles the entire UI        │
│  • User clicks "Upgrade to Pro" in Kalam → opens Stripe      │
│    Checkout in browser → pays → done                         │
│  • Handles: card input, validation, 3D Secure, receipts      │
│  • Supports: monthly ($2.99) and yearly ($32.89) plans       │
│  • No custom payment UI needed                               │
│                                                              │
│  STRIPE CUSTOMER PORTAL (no-code)                            │
│  • Hosted page where users manage their subscription         │
│  • Cancel, upgrade, downgrade, update payment method         │
│  • User clicks "Manage Subscription" in Kalam → opens        │
│    Stripe portal in browser → self-service                   │
│  • No custom billing UI needed                               │
│                                                              │
│  STRIPE WEBHOOKS (low-code)                                  │
│  • Stripe sends events to our Cloudflare Worker:             │
│    - subscription.created → generate license key, store it   │
│    - subscription.cancelled → mark license as expired        │
│    - subscription.renewed → extend license validity          │
│    - payment.failed → grace period, then expire              │
│  • This is the only custom code needed for payments          │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  HOW IT WORKS IN THE APP:                                    │
│                                                              │
│  1. User clicks "Upgrade to Pro" → opens Stripe Checkout     │
│  2. User pays → Stripe webhook fires → our Worker creates    │
│     a license key and stores it in D1 database               │
│  3. User enters license key in Kalam Settings (or it's       │
│     auto-filled via a callback URL)                          │
│  4. On startup: Rust backend validates key against our        │
│     Cloudflare Worker API (quick check)                      │
│  5. Once a day: silent online check to refresh status        │
│  6. If offline: 7-day grace period (Pro stays active)        │
│  7. If key expires: Pro features lock, Free stays intact     │
│  8. "Manage Subscription" → opens Stripe Customer Portal     │
│                                                              │
│  COST:                                                       │
│  • Stripe: 2.9% + 30¢ per transaction                        │
│  • Cloudflare Workers: free tier covers ~100K requests/day   │
│  • Cloudflare D1: free tier covers 5M reads/day              │
│  • Total: essentially just Stripe's cut until you scale      │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Sync — Pro Feature (Multi-PC)

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  SYNC — WHAT SYNCS AND WHAT DOESN'T                          │
│                                                              │
│  Pro users can sync across multiple PCs. But not everything  │
│  should sync — some data is local to the machine where it    │
│  was created.                                                │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  ✅ SYNCS ACROSS PCs:                                        │
│                                                              │
│  • Tasks (created on PC1, appears on PC2)                    │
│  • Notes (created on PC1, appears on PC2)                    │
│  • Reminders (created on PC1, fires on all PCs)              │
│  • Modes / Recipes (your custom modes travel with you)       │
│  • Settings (hotkeys, preferences, theme)                    │
│  • Dictionary entries (custom words)                         │
│  • Formatting rules                                          │
│  • Snippets                                                  │
│  • API keys (encrypted — so you don't re-enter on each PC)  │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  ❌ DOES NOT SYNC:                                           │
│                                                              │
│  • Dictation history (what you spoke stays on that PC)       │
│    → Privacy: dictation is personal to the context/machine   │
│    → Performance: history can be large, syncing is expensive │
│    → If you dictated on PC1, it was for PC1's context        │
│                                                              │
│  • Dashboard analytics (per-machine stats)                   │
│    → Streak, word count, activity — these are per-device     │
│                                                              │
│  • Local STT models (too large to sync — re-download)        │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  HOW SYNC WORKS:                                             │
│                                                              │
│  • Data stored in Cloudflare D1 (per-user, encrypted)        │
│  • Kalam app syncs on startup + periodically in background   │
│  • Conflict resolution: last-write-wins (simple, good        │
│    enough for settings/tasks/notes)                          │
│  • Offline: works normally, syncs when back online            │
│  • Privacy: data is encrypted in transit and at rest          │
│  • User can disable sync (Pro feature, not mandatory)        │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  SYNC IS A PRO FEATURE                                       │
│                                                              │
│  Free users: everything stays local on one machine.          │
│  Pro users: opt-in sync across unlimited PCs.                │
│  This is a strong incentive to upgrade — especially for      │
│  users with a work PC and a home PC.                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Feature Gating — Exactly What's Free and What's Pro

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  DICTATION                                                   │
│  ──────────                                                  │
│  Voice Mode (pure transcription) .............. 🆓 FREE     │
│  Local STT models (no key needed) ............. 🆓 FREE     │
│  Cloud STT with your own key .................. 🆓 FREE     │
│  Filler word removal .......................... 🆓 FREE     │
│  Voice commands (punctuation, new line, etc) .. 🆓 FREE     │
│  Custom formatting rules ...................... 🆓 FREE     │
│  Custom dictionary ............................ 🆓 FREE     │
│  Snippets ..................................... 🆓 FREE     │
│                                                              │
│  Polish (AI cleanup toggle on any mode) ...... ⭐ PRO       │
│  Custom Modes (your AI instructions) ......... ⭐ PRO       │
│  Command Mode (voice → actions) .............. ⭐ PRO       │
│  Context Awareness (screen + clipboard) ...... ⭐ PRO       │
│  Voice-Activated Editing ..................... ⭐ PRO       │
│  Auto-Activation Rules ....................... ⭐ PRO       │
│  Polish granularity (pick what Polish does) .. ⭐ PRO       │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  PRODUCTIVITY                                                │
│  ────────────                                                │
│  Notes (create, edit, organize) ............... 🆓 FREE     │
│  Tasks (create, subtasks, priorities) ......... 🆓 FREE     │
│  Reminders .................................... 🆓 FREE     │
│  History (search, export) ..................... 🆓 FREE     │
│  Dashboard & analytics ........................ 🆓 FREE     │
│                                                              │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  APP & SETTINGS                                              │
│  ──────────────                                              │
│  All hotkey configurations .................... 🆓 FREE     │
│  Audio device selection ....................... 🆓 FREE     │
│  Overlay with waveform ........................ 🆓 FREE     │
│  Light/Dark theme ............................. 🆓 FREE     │
│  Privacy enforcement (ForceLocal) ............. 🆓 FREE     │
│  Auto-updates ................................. 🆓 FREE     │
│  Logging & diagnostics ........................ 🆓 FREE     │
│                                                              │
│  Enhanced overlay (full state + context) ..... ⭐ PRO       │
│  Sync across PCs (tasks, notes, modes, etc.) . ⭐ PRO       │
│  Priority support ............................ ⭐ PRO       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Student Discount

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  Consider offering:                                          │
│                                                              │
│  🎓 Students: 50% off                                       │
│     → $1.49/mo or $16.44/yr                                  │
│                                                              │
│  🏢 Teams/Business: Custom pricing (future)                  │
│     → Centralized billing, admin controls                    │
│     → Only worth building when there's demand                │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Part 6: What We Build and When

### Phase 1 — Modes Foundation + Polish + Onboarding

**Goal:** Replace the current "one pipeline" with switchable modes, introduce Polish as a cross-cutting feature, and make first-run setup effortless.

What gets built:
- The `DictationMode` data structure (name, AI instructions, voice model, language model, context toggles, auto-activation rules — stored in config)
- Voice mode as the default (always present, cannot be deleted)
- Command mode migrated into the modes system
- Built-in recipes: Email, Message, Notes (user can edit/rename/delete)
- User can create unlimited custom modes with their own AI instructions
- The Dictation page in the sidebar with mode cards + mode settings editor
- Mode switching from: Dictation page, keyboard shortcut, status bar
- **Polish toggle** — global ON/OFF that layers grammar/formatting cleanup on any mode
- Pipeline: STT (1 call) → one LLM call with polish + context + mode instructions combined → inject
- **First-run setup wizard** — three paths: Fully Offline, One Key Does Everything, Best Quality, or "I already have keys"
- **Provider cards in Settings** — unified view of all AI providers with clear "Covers: STT + LLM" or "Covers: LLM only" labels, connection status, test buttons
- **Recipe import/export** — export any mode as a shareable JSON recipe, import recipes from files

What this gives users:
- Create any mode they want, save it, reuse it
- Polish their speech on any mode (not just one "polished" mode)
- Switch between modes easily
- See their active mode + polish status at a glance
- Get set up in under 2 minutes with the wizard
- Share their modes as recipes with others

### Phase 2 — Curated Model Library

**Goal:** Make AI provider setup dead simple with a curated, pre-configured model library.

What gets built:
- Curated model cards for recommended STT and LLM models (Groq, OpenAI, Anthropic, Gemini, OpenRouter, local models)
- Each card pre-configured with: base URL, model ID, speed/quality rating, cost estimate, "Get API Key" link
- User's only job: paste their API key — everything else is pre-filled
- Smart key linking: one key for providers that cover both STT + LLM (Groq, OpenAI)
- Custom model option for advanced users (manual base URL, model ID, API key)
- Per-mode model selection (pick which STT and LLM to use for each mode)
- Local model management (download, start, stop) integrated into the library

What this gives users:
- No Googling for base URLs or model names
- One-click setup: pick a model, paste a key, done
- Clear understanding of which models are for voice vs AI
- Advanced users can still add any custom model

### Phase 3 — Kalam Pro (Subscription System) + Website + Service

**Goal:** Add the payment/licensing infrastructure, the website, and the backend service. $2.99/mo or $32.89/yr, BYOK.

What gets built:
- **FSL license switch** — replace current dual-license with Functional Source License
- **Kalam website** on Cloudflare Pages (marketing, docs, download, recipe library)
- **Backend service** on Cloudflare Workers + D1 (license validation, sync API, Stripe webhooks)
- **Private repo** for website + service (separate from the open-source app repo)
- **Stripe integration** (no-code/low-code first):
  - Stripe Checkout for payment (hosted page, no custom UI)
  - Stripe Customer Portal for subscription management (hosted page, no custom UI)
  - Stripe Webhooks → Cloudflare Worker (subscription events → license key management)
- `PlanStatus` in AppConfig (Free / Trial / Pro / Expired)
- License key validation in Rust backend (startup check + daily online refresh)
- Offline grace period (Pro stays active for 7 days without internet)
- "Upgrade to Pro" UI in the app (Pro badge on locked features, upgrade button, license key input in Settings)
- **14-day free trial** (no credit card, no email, no account — just agree to terms)
- Feature gating at hotkey registration, LLM pipeline, and context reading
- Customer portal link in Settings (opens Stripe portal in browser)

What this gives users:
- Clear understanding of what's free and what's Pro
- Frictionless upgrade path (click → Stripe Checkout → pay → instant activation)
- Self-service subscription management (Stripe portal)
- No disruption to free features if they cancel

**Why this comes before Context Awareness:** You want the gating in place *before* you ship the premium features. Otherwise you ship them free and then take them away — that feels bad. Better to launch them as Pro from day one.

### Phase 4 — Context Awareness

**Goal:** Make Kalam understand what's on your screen. (Pro feature)

What gets built:
- Read active app's text content (Windows UI Automation APIs)
- Read selected/highlighted text
- Read clipboard content
- Context toggles per mode (app, selection, clipboard, system info)
- Context indicator in the overlay ("reading Notepad")
- Privacy: context awareness is OFF by default, opt-in per mode
- **Sensitive app cascade**: when a sensitive app is detected, Context is forced OFF, LLM calls are blocked, Polish falls back to local-only regex processing, overlay shows 🔒 lock icon

What this gives users:
- AI that understands what they're working on
- Better spelling of names and terms from screen content
- Clipboard-as-vocabulary (copy a name, AI spells it right)
- Automatic privacy protection for sensitive apps

### Phase 5 — Voice-Activated Editing

**Goal:** Highlight text → speak → AI transforms it in place. (Pro feature)

What gets built:
- Capture selected text when hotkey is pressed
- Send selected text + voice command to LLM
- Replace the selection with the AI's output
- Works in any app (uses existing text injection system)

What this gives users:
- Edit any text with their voice
- "Make this more professional", "translate to French", "turn into bullet points"
- No more copy-paste to ChatGPT and back

### Phase 6 — Overlay: Dormant, Mini & Full States

**Goal:** Redesign the overlay with three distinct states — dormant (idle), mini (compact active), and full (rich active).

What gets built:
- **Dormant state**: idle pill showing mode name + mic icon. Hover reveals: mode switcher, copy last transcription, start recording, expand to full. Right-click menu for quick actions. User toggles always-visible in Settings.
- **Mini state**: compact active pill showing mode name, waveform during recording, status dot, context capture indicators (📋 ✂️), stop button, expand button
- **Full state**: expanded active overlay showing mode name (large), Polish ON/OFF toggle, full waveform, context panel (app, clipboard preview, selected text preview), mode switcher dropdown, pipeline stage display ("Transcribing → Processing → Done"), cancel button, collapse to mini
- User sets preferred active state (mini or full) in Settings or within the pill
- Sensitive app state in all three states (🔒 ForceLocal indicator)

What this gives users:
- **Dormant**: unobtrusive presence, always know what mode is active, quick actions on hover
- **Mini**: compact recording view with essential info
- **Full**: complete visibility into what Kalam is doing — context, pipeline stage, Polish toggle, mode switching
- Confidence that the AI is looking at the right content
- No need to open the main window for common actions

### Phase 7 — Auto-Activation Rules

**Goal:** Kalam switches modes automatically based on what app you're using. (Pro feature)

What gets built:
- Per-mode app rules ("when Outlook is active, use Email mode")
- Auto-switch back when leaving the app (unlike SuperWhisper, which doesn't switch back)
- Rule editor in the mode settings

What this gives users:
- Hands-free mode switching
- Always the right mode for the right app

### Phase 8 — Community Recipe Library

**Goal:** A public collection of community-created recipes that users can browse and install.

What gets built:
- Recipe section on the Kalam website (hosted on Cloudflare Pages)
- In-app "Browse Recipes" button that opens the library
- Categories: Work, Creative, Developer, Languages, etc.
- One-click import from the library into the app
- Recipe submission (users can submit their modes as recipes)

What this gives users:
- Discover new ways to use Kalam without creating modes from scratch
- Share their best modes with the community
- A growing ecosystem that makes Kalam more valuable over time

### Phase 9 — Sync (Multi-PC)

**Goal:** Pro users can sync tasks, notes, modes, settings, and more across multiple PCs. (Pro feature)

What gets built:
- Sync API on Cloudflare Workers (per-user encrypted storage in D1)
- Sync on startup + periodic background sync
- Conflict resolution: last-write-wins
- Offline support: works normally, syncs when back online
- User can enable/disable sync in Settings

What syncs: Tasks, Notes, Reminders, Modes/Recipes, Settings, Dictionary, Formatting Rules, Snippets, API keys (encrypted)

What does NOT sync: Dictation history (stays on the machine where it was spoken), Dashboard analytics (per-device), Local STT models (too large — re-download)

What this gives users:
- Work PC and home PC stay in sync
- Create a task at work, see it at home
- Custom modes travel with you
- Strong incentive to upgrade to Pro

---

## What Stays the Same

These Kalam strengths are untouched by this upgrade:

- **Voice mode as default** — fast, private, no AI required, always free
- **Notes, Tasks, Reminders** — SuperWhisper doesn't have this at all, always free
- **Dashboard & analytics** — streak tracking, top apps, activity heatmap, always free
- **Voice commands** — "new paragraph", "delete that", "undo" (deterministic, no AI needed), always free
- **Dictionary & formatting rules** — regex-powered, more powerful than SuperWhisper, always free
- **Privacy enforcement** — sensitive app detection → ForceLocal, always free
- **Open source** — code is public, community can contribute, inspect, and trust
- **BYOK** — bring your own keys, you control your AI costs, Kalam never charges for usage
- **Recipes** — shareable mode templates, community-driven ecosystem
- **Lightweight** — 10-30MB RAM, tiny bundle size

---

## How This Compares to SuperWhisper After the Upgrade

| Feature | SuperWhisper | Kalam (after upgrade) |
|---|---|---|
| **Price** | $8.49/mo (includes compute) | Free + $2.99/mo or $32.89/yr Pro (BYOK) |
| **Free dictation** | Limited (small models only) | ✅ Unlimited (local or BYOK cloud) |
| Dictation modes | ✅ 7 built-in + custom | ✅ Unlimited user-created + recipes (Pro) |
| Polish (AI cleanup) | Baked into each mode separately | ✅ Toggle that works on ANY mode (Pro, more flexible) |
| Context awareness | ✅ Super Mode only | ✅ Toggle on any mode (Pro, more flexible) |
| Voice-activated editing | ✅ | ✅ (Pro) |
| Auto-activation rules | ✅ (sticky, doesn't switch back) | ✅ (smarter — switches back) (Pro) |
| Model library | ✅ | ✅ Curated + pre-configured + custom model support |
| Overlay | Main + mini, context indicator light | Dormant + mini + full with context panel, Polish toggle, pipeline stages |
| Recipe sharing | ❌ | ✅ Export/import modes as recipes, community library |
| Multi-PC sync | ❌ | ✅ Tasks, notes, modes, settings sync across PCs (Pro) |
| Notes & tasks | ❌ | ✅ (unique to Kalam, free) |
| Dashboard | ❌ | ✅ (unique to Kalam, free) |
| Voice commands | ❌ (not documented) | ✅ (unique to Kalam, free) |
| Local LLM processing | ❌ (cloud only) | 🔮 Future (privacy advantage) |
| Open source | ❌ | ✅ (FSL — readable, modifiable, no competing use) |
| Privacy enforcement | Basic | ✅ Advanced (auto ForceLocal + context lockdown) |
| Lightweight | Unknown | ✅ 10-30MB RAM |
| **You pay for AI usage** | No (included in sub) | Yes (BYOK — you control costs) |

---

## Open Questions for Discussion

### Features
1. **Local LLM support (future)?** — Running a language model locally (via Ollama or similar) would let users have context-aware AI dictation with zero cloud dependency. This is a major privacy differentiator. Should this be on the roadmap?

2. **Recipe curation?** — Should community recipes be curated (reviewed before publishing) or open (anyone can submit)? Curated = higher quality but more work. Open = more recipes but risk of low quality.

3. **Sync conflict edge cases?** — Last-write-wins is simple but could lose data if two PCs edit the same note simultaneously. Is this acceptable, or do we need smarter merging later?

4. **API key sync security?** — Syncing encrypted API keys across PCs is convenient but adds attack surface. Should we sync keys or require re-entry on each PC?

### Resolved Decisions
- ~~Lifetime plan~~ → **No.** Recurring revenue only. No lifetime plan.
- ~~Yearly discount~~ → **1 month free** (pay for 11, get 12). $32.89/yr.
- ~~Polish as separate mode~~ → **No.** Polish is a toggle on any mode.
- ~~Polish + Context = two API calls~~ → **No.** One single LLM call with everything in one prompt.
- ~~Mode sharing~~ → **Yes.** Recipes — shareable mode templates (JSON export/import).
- ~~Sensitive apps + Context~~ → **Context forced OFF** for sensitive apps. Privacy lockdown cascades through everything.
- ~~Polish granularity~~ → **Yes.** Users can pick what Polish does (fix grammar ON, remove filler ON, restructure OFF, etc.). Pro feature.
- ~~Per-mode hotkeys~~ → **No dedicated per-mode hotkeys.** One hotkey + combo key to cycle through modes. Mode switching also available in the overlay.
- ~~Auto-activation switch back~~ → **Yes, switch back.** If auto-activation switched the mode in, it switches back when the user leaves that app.
- ~~Overlay always-visible~~ → **User's choice.** Configurable in Settings — dormant pill can be always-visible or hidden. User decides.
- ~~Full state auto-expand~~ → **User's choice.** User sets preferred active state (mini or full) in Settings or within the pill. Stays in whatever state they chose.
- ~~Free trial length~~ → **14 days.** No credit card, no email, no account. Just agree to terms.
- ~~Trial requires account~~ → **No.** Maximum frictionless. Just agree to terms and go.
- ~~OS contributor perk~~ → **No.** Not offering free Pro for contributors.
- ~~Model Library UI~~ → **Yes, curated.** Pre-configured model cards with everything filled in except the API key. Plus custom model option for advanced users.
- ~~FSL license switch~~ → **Yes, now.** Switching from current dual-license to FSL immediately. Stronger protection, more permissive for business users, standard and well-known.
- ~~Payment platform~~ → **Stripe.** No-code/low-code first. Stripe Checkout for payments, Stripe Customer Portal for subscription management, Stripe Webhooks for license key lifecycle.
- ~~Website/service hosting~~ → **Cloudflare.** Pages for website, Workers for API, D1 for database, KV for cache.
- ~~Website repo~~ → **Private.** App = open source (FSL). Website + service = private repo. Standard open-core practice.
- ~~Recipe library hosting~~ → **Kalam website.** Section on the Cloudflare-hosted site, not a separate GitHub repo.
- ~~Sync~~ → **Yes, Pro feature.** Tasks, notes, modes, settings sync across PCs. Dictation history does NOT sync (stays local).
