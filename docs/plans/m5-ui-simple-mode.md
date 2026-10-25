# Plan: interface polish and simple mode (UI-4, UI-5)

Source — the product owner's notes of 2026-10-24 after first getting to know the interface from PR #25. Work separate from the main M5 line (PR 2b: CFG-2, VMC-1). Start after PR #25 is merged: the plan relies on its screens (picture slots, set import, toggles on the add screen).

## Prompt for a new session

Copy the block into a new Claude Code session opened in `C:\Users\Admin\Projects\yaoplm`:

```text
Hi! We are working on YAOPLM, milestone M5. This session's task is interface polish and "simple mode" (UI-4, UI-5) according to the plan docs/plans/m5-ui-simple-mode.md. Read it in full, then CLAUDE.md, docs/SPEC.md §5.11, docs/I18N.md and docs/TESTING.md.

Step 0: `gh pr list --state all --limit 5`. PR #25 (branch m5/add-art-and-checks) must be merged into main. If it is still open — only read and plan, do not build anything and do not switch branches (the working copy and target/ are shared with another session). After the merge: `git checkout main`, `git pull`, branch `m5/ui-simple-mode`.

Order — as in the plan's "Work order" section. First, build the inventory table (each screen and each control → is it visible in simple mode) and show it to me before hiding anything. The library grid and job noise can be fixed right away.

This is a proactive task: the notes listed in the plan are only the start. Poke every screen, every button and every state yourself (empty, loading, error, lots of data) at all window sizes, in the web and the desktop, and fix everything that looks or works badly. Use the skills and documentation from the plan's "Tools and sources" section: design by the rules of shadcn/ui, Radix, Tailwind 4 and WAI-ARIA, not by eye. Put the list of what was found and fixed in the PR description.

Communicate with me in English; code, comments, commits and PRs — in English. Every visible text — through i18n in English in the same commit. Before the PR — the full CI check set locally, screenshots (English, light/dark, 1280 and 800, plus desktop via WebView2 CDP) to the pr-assets branch under m5/, an independent review of the diff.
```

## Outcome (2026-10-24)

Done in the PR on branch `m5/ui-simple-mode`. The final visibility table — SPEC §5.11.1, decisions — ADR-0034. Tasks 1–8 completed. Picture sources for all types were moved into separate work (the `art-sources` session), and walkthrough findings on a real stick are in `docs/plans/m5-walkthrough-findings.md`.

## What the owner said (briefly)

1. The game grid when the window is resized: the title and metadata under the tile go under the neighbouring tile; at some widths even the covers overlap.
2. Poke the whole interface — every button against the expected behaviour — at different window sizes (Playwright or computer control).
3. The interface is overloaded. A "Simple mode" toggle in settings is needed, **on by default**, can be turned off.
4. In simple mode in the library: search and the format filter stay; sorting, conversion information, the "without cover" counter, "Import pictures" and other extras — hidden. The main thing — a clear "Add games" button. The add screen with all its toggles stays as it is.
5. Screens must be consistent: if something is hidden on one screen, it must not surface on another.
6. Jobs: an ordinary user has no reason to watch them. In simple mode — simplify as much as possible or hide. And overall the log should not record every little thing: a stick scan shows up as a job, sometimes twice in a row.
7. A single button that downloads all pictures itself (in the game card, in the library or when adding) is needed.
8. A global "Titles from the database when adding games" toggle in settings is not needed: it is on the add screen (on by default, remembered).
9. The "Connected" indicator at the bottom of the menu is confusing (added 2026-10-24): it seems to refer to the PS2, the network or Samba. How to handle it — task 8.

## Tasks

### 1. Library grid without overlaps (UI-5)

Cause found: `web/src/features/library/LibraryPage.tsx` gives the grid row a fixed height (`GRID_ROW_HEIGHT = 300`, `estimateSize` in `useVirtualizer`, without `measureElement`), while the column width varies (columns — `floor((width + 16) / (168 + 16))`, `GRID_MIN_COLUMN = 168`). The card height = column width × 200/140 (cover) + text block (title, ID and size, warning line), and already at a column wider than ~180 px it is greater than 300 — the next row lands on top.

- Compute the row height from the actual column width (cover proportion + fixed text block, space for the warning line always reserved) or measure rows with `measureElement`. The first option is more predictable for virtualization on 3000+ games (LIB-5).
- Recompute when the width changes (there is already a `ResizeObserver` in `useColumns`) and reset the virtualizer's size cache.
- Test: Playwright runs window widths from 800 to 2560 in steps (for example 37 px, to hit different column counts) and checks that card rectangles do not intersect and the card text does not go beyond its borders. Plus 3000 games (`make_library --games 3000`): scrolling without jumps.

### 2. Checking the whole interface at different sizes (UI-5)

- A new e2e set `web/tests/e2e/walkthrough.spec.ts`: every screen (library, add, game card, devices, jobs, settings, all dialogs) and every control on it — click and check the expected result (a dialog opened, a plan appeared, cancel wrote nothing, etc.). Sizes: 800×600, 1024×768, 1280×800, 1366×768, 1920×1080, 2560×1440; both themes; English.
- Common checks on every screen: no horizontal scrolling (`scrollWidth <= clientWidth` on `main` and on the document), interactive elements do not overlap, no console errors, every button has an accessible name.
- Desktop: the same walkthrough via WebView2 CDP (based on `web/tests/e2e/desktop-shots.mjs`), with window resizing. Native file selection dialogs are not automated — the owner checks them.
- The result — a report (what was checked, what is broken, what was fixed) in the PR description; fix what is found in the same PR if it is interface.

### 3. Simple mode (UI-4)

- The `simpleMode` setting in `web/src/settings/settings.ts`, default `true`; the toggle as the first item in settings: "Simple mode" with a hint about what it hides and how to bring everything back. One hook (`useSimpleMode()` or a settings field) — without scattered checks across the code.
- **First the inventory table** (in this file or in SPEC §5.11): each screen → each element → "simple / advanced". Show it to the owner before implementation. Below is the starting proposal; the owner's decisions are marked "(owner)".

| Screen | Visible in simple mode | Hidden in simple mode |
|---|---|---|
| Library | search (owner); format filter (owner); "Add games" (owner); one "Download pictures" button (task 5); warning icon on the card | sorting and its direction (owner); grid/list toggle; "Convert…" (owner); "without cover" counter (owner); "Import pictures" (owner); "with warnings" filter; "Verify dumps"; "Rescan"; the "N files OPL does not see" line |
| Add games | everything as now (owner) | — |
| Game card | title and rename, disc type, pictures (slots + "Download all pictures"), delete, dump verification verdict as an icon | hashes and verification details, file path, CFG and VMC information (until the CFG editor from PR 2b), "Convert…" (consistent with the library) |
| Jobs | menu item hidden; write progress — a compact panel with "Cancel", the result — a notification ("Added 2 games", errors — explicit) | the jobs page, logs, scan and search jobs |
| Devices | device list, "Open library" | technical volume details (FS, clusters, serial) — collapsed |
| Settings | theme, loader profile, "Simple mode" | picture sources, cache, Redump DAT — under "Advanced" |

- Consistency (owner): what is hidden on one screen does not surface on another. For example, "Convert…" is hidden both in the library and in the card; the dump verdict is the same icon everywhere. Each rule — one table row and one check in the tests.
- Nothing is lost: everything hidden is available in advanced mode. Switching modes does not reload the page and does not reset filters.
- Tests: vitest for element visibility in both modes; the walkthrough e2e (task 2) — in both modes.
- ADR: what is hidden and why, and the consistency rule.

### 4. Jobs without noise (in both modes)

- Library scanning is an internal action: do not show `scanLibrary` jobs on the jobs page (scan progress is already visible in the library header, `ScanProgress`).
- Find out why the scan runs twice in a row. Where to look: `rescanIfIdle` in `web/src/jobs/JobsProvider.tsx` resets the `["library"]` query after `executePlan`, `verifyGames` and `updateRedump`; `useLibrary` (`web/src/features/library/useLibrary.ts`) uses the key `["library", root, profile, verify]` in the library and in the game card; adding games and importing pictures may reset it again; in development mode — `StrictMode`. Reproduce: add a game and look at the jobs. Fix and close with a test (one scan per one write queue completion).
- Logs are collapsed by default; line-by-line transaction steps (started, completed) — only in advanced mode, in simple — the summary.

### 5. One "Download all pictures" button (ART-4, ART-8)

- Library: in simple mode "Download pictures" — one click without a dialog: all types that the enabled sources have (currently COV, SCR, SCR2, LGO), missing only → a plan with confirmation. In advanced mode the dialog with type and scope selection remains.
- Game card: "Download all pictures" above the slots — the same for one game (the first found variant of each type), without a variant window.
- Add games: the "Download covers" toggle → "Download pictures for added games" with all available types. The core is already half ready: `PlanImport.covers` accepts an `ArtItem` of any type, and `plan_covers` in `crates/yaoplm-core/src/import.rs` takes the type into account; but the search for import takes only COV (`art::import_targets(…, ArtKind::Cov)` in `crates/yaoplm-api/src/lib.rs`, handling `FetchArt.import`). Need to pass `kinds` into the search for import and update ADR-0031.

### 6. Add toggles — only on the add screen

- Remove "Titles from the database when adding games" from settings (`web/src/features/settings/SettingsPage.tsx`) (owner). For consistency also remove "Download covers for added games" and "Verify dumps when adding games": all three live on the add screen and are remembered there.
- Update the places where they are described as settings (SPEC GAME-5, ADR-0030); delete i18n keys that become unused.

### 7. Proactive interface audit (UI-4, UI-5)

The owner asks not to limit ourselves to the list above but to go through the interface entirely and bring it to good design. For each screen:

- **Hierarchy and density.** One main action per screen, secondary ones quieter; grouping and spacing on the Tailwind grid (4 px), without random values; one text size scale.
- **States.** Empty (first launch, no device, no games), loading (skeletons instead of layout jumps), error (what happened and what to do), a lot of data (3000 games, long titles in English, long paths).
- **Texts.** Short and to the point, the same words for the same thing (glossary docs/I18N.md), hints only where it is unclear without them; in simple mode fewer hints.
- **Accessibility.** Keyboard (focus order, visible focus, Esc closes dialogs, focus returns), accessible names, contrast in both themes (WCAG 2.2 AA), WAI-ARIA patterns for dialogs, menus, radio groups, tabs.
- **Responsiveness.** From 800×600 to 2560×1440 without overlaps and horizontal scrolling; toolbars wrap instead of being cut off; dialogs fit in height.
- **Consistency.** The same components for the same tasks (buttons, plan dialogs, badges, empty states); button behaviour is the same on all screens.

The audit result — a "screen → what is wrong → what was done" table in the PR description, before and after screenshots.

### 8. Connection indicator: only where the connection can be lost

What it actually shows: the interface's connection to the YAOPLM core — the part of the program that does all the work (reads the library, writes games, converts, computes hashes, downloads pictures). It has nothing to do with the PS2, Samba or the console's network: Samba only shares a ready folder, and the core fills it.

- **Desktop:** the interface and the core are one program (Tauri IPC inside the process), the connection is not lost, the dot is always green and reports nothing. Do not show the indicator.
- **Browser** (`transport.kind === "http"`): the core runs on another machine (`yaoplm-server` on a NAS or PC), and if it is off or the network is gone, the buttons will do nothing. Remove the permanent "Connected" label; when the connection is lost, show a clear banner: "No connection to YAOPLM at <address> — actions are unavailable until the connection returns" (English), and while there is no connection — do not allow starting actions.
- Code: `web/src/components/ConnectionStatus.tsx`, transport state — `useApi().status`.
- In the simple mode inventory table: the indicator — in both modes per the rule above.
- Do not confuse it with the future "Control from other devices" toggle in the desktop (the program's web interface on the local network, off by default): that is a network milestone, not this PR (ROADMAP, "Later").

## Tools and sources

- **Claude Code skills** (if the owner has connected them; check `ListPlugins`, suggest an install card if not): `frontend-design` (Anthropic) — interface design and implementation; `Design` (Anthropic) — `design-critique` for analysing screens, `accessibility-review`, `ux-copy` for texts, `design-system` for component rules; `Axe Accessibility` (Deque) — accessibility checking with a fix loop.
- **Browser check:** Playwright (e2e in `web/tests/e2e`, plus Playwright MCP for a manual walkthrough), Claude's built-in browser for the web version via `.claude/launch.json` (`yaoplm-server-fixture`).
- **Desktop:** `pnpm tauri dev` with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and Playwright via CDP (`web/tests/e2e/desktop-shots.mjs`); window resizing and everything CDP cannot see (native dialogs, dragging files from Explorer) — via computer control (the `anthropic-skills:computer-use` skill).
- **Automatic accessibility in e2e:** `@axe-core/playwright` (MPL-2.0, GPL-3 compatible). Before adding, check maintenance and licence, pin an exact version at least a day old (CLAUDE.md, `minimumReleaseAge`).
- **Documentation** (read the current version, not from memory): shadcn/ui (`radix-nova` style, components in `web/src/components/ui`), Radix Primitives (accessibility of each component), Tailwind CSS 4 (theme, `@theme`, container queries for responsive cards), TanStack Virtual (dynamic row height, `measureElement`), WAI-ARIA Authoring Practices, WCAG 2.2.
- **Neighbours:** docs/research/10-ui-prior-art.md (OrbitPS2 Manager, OPL PC Tools, POUM) — what has already been taken; add new findings there too.

## Work order

1. Step 0 from the prompt, branch `m5/ui-simple-mode`.
2. Inventory table (task 3) → agree with the owner.
3. The grid (task 1) with a test.
4. Job noise (task 4).
5. Add toggles (task 6).
6. The "Download all pictures" button (task 5), with the core and CLI if needed.
7. Simple mode (task 3).
8. Connection indicator (task 8).
9. Interface walkthrough at all sizes, web and desktop (task 2), and the proactive audit (task 7); fix what is found.
10. Documentation: SPEC (UI-4, UI-5, §5.4.1), ADR, TESTING (the new e2e set), ROADMAP after the merge.
11. Full CI locally, screenshots of both modes, independent review, PR.

A console check for this PR is not needed: only the interface changes. If the picture search when adding changes (task 5) — one item: the added game shows its cover and screenshots in OPL.

## Open questions

- Exporting pictures from a device (a backup of `ART/` in a zip in the OPL Manager layout or in `art.tar` for RiptOPL, ART-6) — only an idea so far, not part of this PR.
