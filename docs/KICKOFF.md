# Starting prompt for a development session

Copy the text below into a new Claude Code session opened in the `C:\Users\Admin\Projects\yaoplm` folder. The prompt is updated at the end of each milestone and when work is handed over to a new session in the middle of a milestone; previous versions are in the git history.

---

Hi! We are continuing YAOPLM development, milestone **M5 — Dump verification, game settings, VMC, loaders → v0.2.0**. The previous session overflowed and is handing the work over to you. We are starting the **milestone's second PR**.

## What has already been done in M5

- **PR #21** (merged): dump verification against Redump (DAT on demand at most once a week, DAT import from a file, verdicts, hash cache; ADR-0026), ISO ↔ ZSO ↔ `ul` conversions in either direction and "Compress to ZSO" when adding (ADR-0027), import from CHD, zip, 7z and multi-track BIN/CUE (ADR-0028). Console checklist: `docs/testing/m5-pr1-console-checklist.md`, test files — `D:\yaoplm-testdata\out\m5-packed`.
- **PR #22**: checkboxes in ROADMAP; the owner's decision — dump verification when adding games goes into the second PR.
- **PR #23 and the "phrases" PR (following it)**: all localization of core texts (ADR-0029). Any core text for the user is a phrase: a catalogue key and parameters. Details below, in "Rules that appeared recently".
- The owner has not yet run the `docs/testing/m4-console-checklist.md` and `m5-pr1-console-checklist.md` checklists on the console. Notes on them will come during the work; we fix them in separate small PRs, not mixed with the PR 2 branch.

**Step 0.** Check `gh pr list --state all --limit 5`: the phrases PR (branch `m5/phrases-everywhere`) must be merged into `main`. If it is still open, the previous session in this same folder is finishing it: until it is merged, only read and plan — do not switch branches, do not `git pull`, and do not run builds or tests (the working copy and `target/` are shared). After the merge: `git checkout main`, `git pull`, then a branch for PR 2a.

## What we are doing: the milestone's second PR

According to the milestone plan (ROADMAP, the previous version's KICKOFF), the second PR is covers on import and picture type slots (ART-8, ART-2, ART-5), the CFG editor (CFG-2) and VMC (VMC-1), plus the owner's decision of 2026-10-23: dump verification when adding. That is a lot. **Suggest to the owner splitting it into two PRs** and do them in order:

**PR 2a — adding games and pictures:**
1. **Covers on import (ART-8, SPEC §5.4.1, ADR-0025 with a clarification).** A "Download covers" checkbox on the add screen, on by default, remembered. After "Show plan" the same `FetchArt` job starts with the "missing only" scope for the image IDs from the plan (a per-game log right under the plan, cancel). When the search finishes, the import plan is rebuilt: after each game's copy transaction comes a transaction writing `ART/<ID>_COV.png` from the cache. One confirmation, one job. API: `PlanImport` gets a reference to the ready search plan (`covers: <plan id>`), the core accepts a list of `ArtItem` and adds the transactions itself; CLI — `yaoplm add --covers`. Duplicate IDs get one cover. A cover not found goes into "skipped" and never blocks the import; without a network the import works as it does now.
2. **Dump verification when adding (GAME-5, ROADMAP M5 item).** Hashes are computed in the same pass as the write to the device (the copy stream also goes to the `yaoplm-hash` hasher), without a separate read of the stick. After the write, the Redump DAT verdict is immediate, and the hashes go into the cache under the key of the new file on the device (volume + path + size + mtime, as in `crates/yaoplm-api/src/verify.rs`: `keys`, `attach_checks`, and `put_hashes` in `crates/yaoplm-net/src/cache.rs`). Without a DAT the hashes are still saved. The "Verify dumps when adding" toggle in settings and on the add screen, on by default; a flag on `yaoplm add`. **For a CD from BIN/CUE and CHD, check the raw data track** (2352 bytes per sector), as Redump hashes it: only this way can a CD game get "verified" (an ISO never matches a CD, ADR-0026). Compression to ZSO when adding — hash the ISO data before compression.
3. **Picture type slots (ART-2, ART-3).** In the game card, instead of one cover — a row of slots: `COV`, `ICO`, `COV2`, `BG`, `SCR`, `SCR2`, `LGO`, `LAB`; each with a preview, size and "Download", "Replace" (file, clipboard, drag-and-drop), "Delete"; each action — the same plan with a backup. Type sizes — research/02 §7 (COV 140×200, COV2 242×344, LAB 18×240, ICO 64×64, LGO 300×125, SCR/SCR2 250×188, BG 640×480; RiptOPL — doubled COV/ICO). Sources by type: xlenore — `COV`; libretro-thumbnails — `Named_Snaps` → `SCR`, `Named_Titles` → `SCR2`, `Named_Logos` → `LGO`. New types — new `ArtKind` in the same pipeline (ADR-0025). Bulk download gets a type selection.
4. **Art pack import (ART-5).** OPL Manager zip backups from archive.org (the user downloads them themselves), folders, `art.tar` → `ART/<ID>_<SUFFIX>.png` with transcoding, a plan with confirmation and a backup of replaced files. Never contact OPL Manager's private server (rule 3).

**PR 2b — game settings:**
5. **CFG editor (CFG-2).** Compatibility modes (bits 0x01–0x20; 0x40/0x80 only in the RiptOPL profile), DMA (HDD only), GSM, VMC slots, cheats on/off, PADEMU, AltStartup. Keys and values — research/02 §6.3; OPL deletes a key when the value returns to default. Preserve unknown keys and their order (rule 2), a backup before every change (CFG-5, `.yaoplm/backup/`), writing through a plan. A parser that preserves unknown keys already exists: `crates/yaoplm-formats/src/cfg.rs`.
6. **VMC creation (VMC-1).** A correctly formatted 8/16/32/64 MiB card (not an empty file) and assignment to a slot (`$VMC_0`/`$VMC_1`). Format — research/02 §9 (like `genvmc`: no ECC, 512-byte pages, 1024-byte cluster, IFC from cluster 8, FAT, root with `.` and `..`, 0xFF fill, exact size). The file on BDM must be contiguous: preallocation (ADR-0019). Check — `vmc::opl_check` in `crates/yaoplm-formats/src/vmc.rs`. Sources: `Adubbz/mymcplusplus` (`ps2mc.py`, function `format()`, GPL-3, may be ported with attribution), `bucanero/ps2vmc-tool` (GPL-3); `modules/vmc/genvmc/genvmc.c` in OPL — AFL, logic only, do not copy the code. **Finding:** Orbit (`src/services/vmc.service.ts`) creates a file from zeros and writes that OPL will format it on first launch. According to `sysCheckVMC` (`src/system.c` in OPL) this is not so: without a signature, `mc_type == 2` and a size match, OPL sets the size to 0 and does not use the card.

Each PR — with its own console checklist in `docs/testing/` and screenshots.

## Rules that appeared recently (important)

- **Core texts are phrases (ADR-0029, docs/I18N.md, rules 6–8).** The `Phrase` type from the `yaoplm-i18n` crate (re-exported in `yaoplm-core`): `Phrase::title`, `Phrase::error`, `Phrase::notice`, `Phrase::reason` and `.with("name", value)`, `.count(n)`. A notice: `Notice::warning(Phrase::notice("noSpace").with("needed", n).with("free", f))`; a plan and a transaction: `Plan::titled`, `Transaction::titled`. The core renders English from the `en` catalogue, so a new key goes into `en` in the same commit, otherwise the tests fail. Catalogue sections: `titles`, `errors` (whole sentences), `notices` (notices, warnings, advice, skip reasons; the code is the last part of the key), `reasons` (fragments after a colon, lowercase). Numbers are passed as numbers (`count` selects the plural form, sizes — `{{size, bytes}}`), technical details (OS messages, parse error positions) — as text inside the English sentence.
- **Localization checks** (`web/tests/unit/i18n.test.ts`): keys and plural forms in the `en` catalogue, placeholders, typography, a phrase for every phrase from the Rust sources, **matching parameters of every phrase to its template placeholders** (with the file and line on a mismatch), no unused keys. In Rust a placeholder without a value is an error in a debug build.
- **Interface**: `phraseText`, `errorText` (`web/src/lib/phrase.ts`), `useNoticeText` (`web/src/lib/notices.ts`); API types have `phrase`, `titlePhrase`, `errorPhrase`; numbers are formatted per the English locale ("5.0 GiB"). Strings are English-only in the keys, glossary and typography — docs/I18N.md.
- The owner wants no duplicate text in the interface and, after large changes, a **deep review**: static checks, exhaustive tests, a manual browser review in English, and an independent review of the diff by a separate agent.

## Where things live

- Covers: `crates/yaoplm-core/src/art.rs` (`plan_set_art`, `plan_delete_art`, `ArtItem`), `crates/yaoplm-api/src/art.rs` (the `FetchArt` job, `targets`), `crates/yaoplm-net` (`ArtService`, sources, `transcode::to_png`, cache), interface `web/src/features/art/` and `web/src/features/game/GamePage.tsx`.
- Import and writing: `crates/yaoplm-core/src/import.rs` (`plan_import`, `Planner`), `crates/yaoplm-core/src/exec.rs` (`Runner`: `write_file`, `write_zso` — embed the hasher here), `crates/yaoplm-hash`, `crates/yaoplm-core/src/source.rs` (`probe`, BIN/CHD), `web/src/features/add/AddGamesPage.tsx`.
- Dump verification: `crates/yaoplm-api/src/verify.rs`, `crates/yaoplm-core/src/verify.rs`, the hash cache in `cache.sqlite` (`yaoplm-net`).
- CFG and VMC: `crates/yaoplm-formats/src/cfg.rs`, `crates/yaoplm-formats/src/vmc.rs`, CFG backups — `crates/yaoplm-core/src/ops.rs`.
- Neighbours: `D:\yaoplm-testdata\refs\<owner>__<repo>` (`git pull` before reading): Orbit (`artwork*`, `cfg.service`, `vmc.service`), OPL-PC-Tools (`GameArtManager`, `Vmc*`, `GameConfiguration`), PyOPLM, mymcplusplus, ps2vmc-tool, upstream OPL. What was taken — briefly in `docs/research/13-*.md`.

## Process

1. Read `CLAUDE.md`, `docs/SPEC.md` (ART-2, ART-3, ART-5, ART-8, §5.4.1, GAME-5, CFG-2, CFG-5, VMC-1), `docs/ARCHITECTURE.md` §4–§8, `docs/ROADMAP.md` (M5), `docs/DECISIONS.md` (ADR-0021, 0025, 0026, 0029), `docs/I18N.md`, `docs/TESTING.md`, research/02 §6, §7, §9 and research/11, 12.
2. Branches `m5/<slug>`, small commits in Conventional Commits style with requirement IDs. Before a PR — the full CI set locally (commands in CLAUDE.md), screenshots in English and light/dark theme to the `pr-assets` branch, folder `m5/`, links like `https://github.com/belomaxorka/yaoplm/blob/pr-assets/m5/<name>.png?raw=true`.
3. PR in `belomaxorka/yaoplm`: description (what, requirement IDs, how it was tested, console checklist). `main` has no protection: **do not use `gh pr merge --auto`**. The owner has allowed merging with a regular merge commit when an independent review finds no problems and CI is green. After the merge — checkboxes in ROADMAP; a decision that changes the specification — in an ADR, and tell the owner.
4. Check the interface yourself: previews `yaoplm-server-fixture` and `web-dev` from `.claude/launch.json`, Playwright (`pnpm -C web e2e`), desktop `pnpm tauri dev` when needed.

## Environment and pitfalls

- **Disk C:** on 2026-10-23 it filled up to zero because of `target/` (105 GB). Build with `CARGO_INCREMENTAL=0`, watch `df -h /c` (not below ~15 GB), `target/debug/incremental` can be deleted, `cargo clean` — build cache only. Do not build anything on D:.
- Before `cargo test`, stop the server preview: Windows locks a running `yaoplm-server.exe`.
- In the Bash tool's heredoc, double backslashes collapse; for text with `\` and quotes use Write/Edit or a Python script in a file.
- D: — the owner's data drive: write only to `D:\yaoplm-testdata`. Stick E: (FAT32, volume serial `7E1CD2C5`), OPL 1.2.0 beta and 1.1.0 from the FreeMcBoot card. Do not write to real devices without the owner's confirmation.
- Do not download commercial images, BIOS or POPS; Redump DAT — at most once a week; in tests and CI — only `--import` and mirrors.

## Deferred for later

- Game information (genre, developer, description) on the game page and in CFG — META-3, ROADMAP M6.
- The milestone's third and fourth PRs: automatic profile detection and loaders (LIB-1, LDR-1…4, BOOT-1); the privileged helper, formatting, the setup wizard, `yaoplm-server` in Docker, fragmentation checking (DEV-3, DEV-4, SRV-1, IMP-7b). Closing the milestone: checkboxes, tag `v0.2.0`, a new starting prompt, a checklist and the M6 plan.

Communicate with me in English.
