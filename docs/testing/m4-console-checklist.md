# M4 — console cover checking checklist

Milestone M4 (PR #14, release v0.1.0). The owner checks it on their PS2. Tick items as you go; report issues in separate small PRs (ROADMAP, workflow). General test-data rules — in [TESTING.md](../TESTING.md).

Intended for stick E: (FAT32, volume serial `7E1CD2C5`), OPL 1.2.0 beta and OPL 1.1.0 from the FreeMcBoot card. Real dumps are not needed: for covers, generated images with real IDs are enough for OPL to show them in the list (they will not launch, which is expected). Every write to E: goes through a plan; confirm only those.

## Setup

- [ ] 1. Install the build from release `v0.1.0` (`YAOPLM_0.1.0_x64-setup.exe`) or run `pnpm tauri dev` from `apps/desktop`. In "Settings → About" the version is `0.1.0`, the mode is "Application".
- [ ] 2. "Settings → Library": profile `OPL 1.2.0 beta`, names `ID.Title.iso`. "Settings → Covers": both sources enabled, the "Mirror" fields empty (the internet is needed).
- [ ] 3. The files for import are on D::
   - `D:\yaoplm-testdata\out\m4-fixture\library\CD\SLUS_200.62.Retail Game.iso` (GTA III ID, a cover exists at xlenore);
   - `D:\yaoplm-testdata\out\m4-fixture\library\DVD\SLUS_212.18.Dual Layer Quest.iso` (Tak ID, a cover exists);
   - `D:\yaoplm-testdata\out\YAOP_000.01.wLaunchELF.iso` (bootable homebrew, no cover anywhere).
   - For manual replacement: `D:\yaoplm-testdata\out\m4-fixture\mirror\custom-cover.jpg` (a striped picture, easy to recognise on screen).
   - If the `m4-fixture` folder is missing: `cargo run -p yaoplm-api --example make_library -- D:\yaoplm-testdata\out\m4-fixture`.

## In YAOPLM on Windows

- [ ] 4. "Devices" → the E: card (serial `7E1CD2C5`) → "Open library". The header shows `E:\`.
- [ ] 5. "Add games" → "Add files…" → the three images from item 3 → "Preview plan". Expected: `CD/SLUS_200.62.Retail Game.iso`, `DVD/SLUS_212.18.Dual Layer Quest.iso`, `CD/YAOP_000.01.wLaunchELF.iso`. "Add to device" → in "Jobs" the job is finished, three operations `done`; in the library three games, the counter "3 without cover".
- [ ] 6. The "Retail Game" card: in "Details" the row "Title in the database: Grand Theft Auto III" with a "Rename to this" link (clicking is optional). At the bottom "No cover files".
- [ ] 7. Library → "Download covers". The dialog says "Only 3 games without a cover", sources `1. xlenore/ps2-covers`, `2. libretro-thumbnails`. "Start". In the log: `SLUS_200.62 Retail Game: found in xlenore`, `SLUS_212.18 Dual Layer Quest: found in xlenore`, `YAOP_000.01 wLaunchELF: not found`. Then the plan dialog "Write 2 covers" with `YAOP_000.01` skipped. "Confirm and execute".
- [ ] 8. In the grid two real covers (GTA III and Tak), wLaunchELF has a placeholder; the counter "1 without cover". In the file manager `E:\ART\SLUS_200.62_COV.png` and `E:\ART\SLUS_212.18_COV.png`, both 140×200 (file properties), about 50 KB.
- [ ] 9. "Jobs": for the "Find missing covers" job expand "Log" — the same three lines; the write job has two operations `done`.

## On the console, OPL 1.2.0 beta

- [ ] 10. Stick E: in USB, OPL 1.2.0 beta from the FreeMcBoot card, USB mode. The list has three games: `Retail Game`, `Dual Layer Quest`, `wLaunchELF`. We do not check launching, only the list (wLaunchELF launches, as in M2).
- [ ] 11. Select `Retail Game`: in the cover area the theme shows the GTA III box. `Dual Layer Quest` — the Tak cover. The disc icon (ICO) is empty: this is expected, ICO will appear in v0.2.
- [ ] 12. Select `wLaunchELF`: no cover, the area is empty or has the theme's placeholder, the list does not break, navigation works.

## On the console, OPL 1.1.0

- [ ] 13. The same stick in OPL 1.1.0: items 10–12 repeat (both versions read PNG).

## Replacement and deletion

- [ ] 14. In Windows: the "Retail Game" card → "Replace…" → "From file…" → `custom-cover.jpg`. The plan: `rename ART/SLUS_200.62_COV.png → .yaoplm/backup/<date>/ART/…` and `write ART/SLUS_200.62_COV.png`, with the note "the current cover is moved to the backup folder". Confirm. The card shows the striped cover; `E:\.yaoplm\backup\<date>\ART\` holds the previous GTA III file.
- [ ] 15. On the console: restart OPL (it keeps textures in memory, one rescan may not be enough). `Retail Game` has the striped cover.
- [ ] 16. In Windows: the `Retail Game` card → "Remove cover" → a plan with deletion; the button is unavailable until "I understand that files will be deleted" is ticked → confirm. The file is gone from `E:\ART`, a copy is in `.yaoplm\backup\…`. On the console after restarting OPL, `Retail Game` has the placeholder again.
- [ ] 17. Restore: the `Retail Game` card → "Download cover" → "Start". The log says `SLUS_200.62 Retail Game: from cache (xlenore)` — without a repeated network request; plan → confirm → GTA III is back.

## Optional

- [ ] 18. Paste from the clipboard: copy any picture in a browser ("Copy image"), in the card "Replace…" → "Paste from clipboard" → plan.
- [ ] 19. Drag a picture file onto the cover in the card → plan.
- [ ] 20. The `RiptOPL` profile in settings, "Download again" for one game: the file in `E:\ART` becomes 280×400.

## What to report

A screen photo for items 11, 12, 15 and 16 with the OPL version (the build line in its "About" menu), the output of `dir E:\ART` and `dir /s E:\.yaoplm\backup` after item 16, and the text of any error from the plan or job dialog. If OPL shows the cover in the wrong place or the wrong one: the file size and its properties (width, height, colour depth) in the file manager.
