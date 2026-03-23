# Apply Prototype to Main App — Section-by-Section Plan (revised)

## Principle

- **Apply the entire prototype.** Use the prototype’s structure, markup, and CSS everywhere. No “Option A vs Option B”: the prototype is the single source of truth.
- **When the app has a feature the prototype doesn’t:** adapt that feature so it follows the prototype’s design language (same classes, same layout patterns, same styling). Do not introduce a different UX or “minimal” variant.

---

## Context

- **Prototype:** [src/pages/Prototype.svelte](src/pages/Prototype.svelte) — all views and design system in one file (~2500+ lines of CSS from ~2522).
- **Main app:** [src/App.svelte](src/App.svelte) + [Home](src/pages/Home.svelte), [Settings](src/pages/Settings.svelte), [Snippets](src/pages/Snippets.svelte), [History](src/components/views/History.svelte), [Notes](src/components/views/Notes.svelte), [Tasks](src/components/views/Tasks.svelte), [Reminders](src/components/views/Reminders.svelte). Detail add/edit currently uses SidePanel; the prototype uses full-page detail routes.

---

## 1. Design system and shell (App.svelte)

- Copy the full prototype design system (`.kalam-sleek`, theme vars, sidebar, main, page-content, and all page-specific CSS) into the main app (e.g. App.svelte `:global(.kalam-sleek ...)` or shared stylesheet). Remove/override conflicting app CSS.
- Shell markup: match prototype exactly (sidebar structure, main, page-content). **App-only:** sidebar collapse — keep the feature but style it with the prototype’s language (same spacing, same icon/button style, no extra visual language).

---

## 2. Home (Overview)

- [Home.svelte](src/pages/Home.svelte): use prototype’s exact structure and classes (`.page`, `.page-header`, `.stats-row`, `.stat-box`, `.dashboard-grid`, `.dash-section`, `.list-item`, `.chip.chip-mode` / `.chip.chip-stt`, `.simple-list`, `.simple-item`, etc.). Add any missing prototype classes (e.g. STT chips). Keep real data and `navigate()`.

---

## 3. History (list)

- [History.svelte](src/components/views/History.svelte): replace current layout with prototype’s (`.search-bar`, `.timeline`, `.day-group`, `.entry-row`, `.entry-time`, `.entry-content`, `.entry-actions`, chips, copy `.icon-btn.small`). Group by day like prototype. Keep `invoke`, search, copy behavior.

---

## 4. Notes (list)

- [Notes.svelte](src/components/views/Notes.svelte): use prototype’s list structure (`.notes-header`, `.btn-primary` “New Note”, `.notes-subnav`, `.notes-search-bar`, `.notes-label-filters`, `.notes-lists-container`, `.notes-section`, `.notes-masonry`, `.note-card`). Keep scope, search, labels, API; clicking New Note / card will navigate to full-page note-detail (see 5).

---

## 5. Note detail (add/edit)

- **Apply prototype exactly:** full-page note detail as in prototype (`currentPage === 'note-detail'`).
- Add `note-detail` route in App.svelte. When opening a note (new or edit), `navigate('note-detail')` and pass/store note id. Render the prototype’s note-detail markup (`.sleek-header`, `.sleek-back`, `.sleek-actions`, `.sleek-body`, `.sleek-footer`, color/reminder/pin tools). Wire to existing Notes API. No SidePanel for notes; use full-page only.

---

## 6. Tasks (list)

- [Tasks.svelte](src/components/views/Tasks.svelte): use prototype’s structure (`.notes-header` + “New Task”, `.notes-search-bar`, `.task-list-large`, `.task-row`, `.drag-handle`, `.checkbox`, `.task-info`, `.task-meta`, `.priority-indicator`, etc.). Keep data, search, reorder; “New Task” / row click navigate to full-page task-detail (see 7).

---

## 7. Task detail (add/edit)

- **Apply prototype exactly:** full-page task detail (`currentPage === 'task-detail'`).
- Add `task-detail` route. Use prototype’s task-detail markup (`.sleek-header`, `.sleek-body`, priority, due date, subtasks, labels). Wire to existing task API. No SidePanel for tasks.

---

## 8. Reminders (list)

- [Reminders.svelte](src/components/views/Reminders.svelte): use prototype’s structure (`.notes-header` + “New Reminder”, `.notes-search-bar`, `.reminder-list-large`, `.reminder-row`, `.reminder-icon-large`, `.reminder-info`, `.reminder-source-badge`, etc.). If app has combined reminders (notes + tasks + standalone), match prototype’s combined list; otherwise style existing list with same classes. Navigate to full-page reminder-detail (see 9).

---

## 9. Reminder detail (add/edit)

- **Apply prototype exactly:** full-page reminder detail (`currentPage === 'reminder-detail'`).
- Add `reminder-detail` route. Use prototype’s reminder-detail markup. Wire to existing reminder API. No SidePanel for reminders.

---

## 10. Snippets (list)

- [Snippets.svelte](src/pages/Snippets.svelte): use prototype’s structure (`.notes-header` + “New Snippet”, `.notes-search-bar`, `.snippets-grid`, `.snippet-card`, `.trigger-code`, `.uses-count`, `.expansion-text`, `.snippet-tags`). Click card / New Snippet navigate to full-page snippet-detail (see 11).

---

## 11. Snippet detail (add/edit)

- **Apply prototype exactly:** full-page snippet detail (`currentPage === 'snippet-detail'`).
- Add `snippet-detail` route. Use prototype’s snippet-detail markup (trigger with “/”, expansion, labels). Wire to `get_snippets` / `add_snippet` / `remove_snippet`. No SidePanel for snippets.

---

## 12. Settings (all tabs)

- [Settings.svelte](src/pages/Settings.svelte): use prototype’s layout and classes (`.settings-page`, `.settings-tabs`, `.settings-section`, `.setting-row`, `.setting-label`, `.setting-control`, `.hotkey-capture-area`, `.hotkey-pills`, etc.). Keep all app tabs (General, Audio & Dictation, **Dictionary**, **Command**, Privacy, Advanced, About). **App-only tabs (Dictionary, Command):** not in prototype — build their UI using the same prototype language (same section/row/card patterns, same inputs and buttons). About: use prototype’s card layout (Community, Support, License). Preserve every `invoke` and save.

---

## 13. App-only features (adapt to prototype language)

- **Sidebar collapse:** Keep; style toggle and collapsed state with prototype’s button/icon and spacing (no new visual language).
- **Dictionary tab (Settings):** Use `.settings-section`, `.setting-row`, `.setting-label`, `.setting-control`; list and inputs styled like prototype form elements.
- **Command tab (Settings):** Same: prototype’s section/row/control patterns and form styling.
- Any other app-only UI: same rule — use prototype classes and patterns so it feels part of the same design system.

---

## 14. Shared CSS and routing

- All prototype CSS available to main app (single place: App.svelte global or shared stylesheet under `.kalam-sleek`).
- App.svelte handles `currentPage`: `home`, `history`, `notes`, `note-detail`, `tasks`, `task-detail`, `reminders`, `reminder-detail`, `snippets`, `snippet-detail`, `settings`. List views call `navigate('note-detail')` etc. with selected id/state (prop, store, or query).

---

## 15. Implementation order (suggested)

1. Design system + shell (section 1)  
2. Home (2)  
3. History (3)  
4. Settings (12)  
5. Notes list (4) → Note detail (5)  
6. Tasks list (6) → Task detail (7)  
7. Reminders list (8) → Reminder detail (9)  
8. Snippets list (10) → Snippet detail (11)
