# YAOPLM — work plan

> Version 0.1, 2026-10-22. Requirements (`IMP-5`, etc.) are in [SPEC.md](SPEC.md). Tick `[x]` when an item is merged into `main`.

## Workflow

**Roles.**
- **User** — product owner and tester on a real PS2. Accepts milestones.
- **Agent** (Claude Code) — developer: code, tests, CI, documentation.

**Cycle of one milestone:**
1. The agent reads the milestone in this file and breaks it into tasks (in the session plan).
2. Creates a branch `m<N>/<short>`, for example `m1/core-formats`.
3. Works in small commits in Conventional Commits style in English, with requirement ID references: `feat(formats): ul.cfg writer (IMP-5)`.
4. Writes tests together with the code. For the interface, checks the UI in the browser via server mode and Playwright.
5. Opens a PR. In the description: what was done, how it was tested, a **PS2 test checklist**.
6. CI must be green.
7. The user tests on the console and writes remarks. The agent fixes them.
8. Merge with a regular merge commit so that the branch commits stay in history (ADR-0017); tag `v0.0.N` (or `v0.N.0` on major milestones), update this file and [DECISIONS.md](DECISIONS.md).
9. Before a release tag `v0.N.0` — the UI consistency pass in [plans/release-ui-consistency.md](plans/release-ui-consistency.md): rules, medium state matrix, findings in separate PRs (product owner's decision 2026-10-25).

If a decision that changes the specification appears along the way, the agent records an ADR in DECISIONS.md and informs the user.

---

## M0 — Foundation
Goal: an empty but buildable skeleton on all OSes with three interfaces.
- [x] Install the toolchain: rustup (MSVC, stable), `cargo-tauri` / `@tauri-apps/cli`, pnpm via corepack. Check the C++ workload of VS Build Tools 2026.
- [x] Cargo workspace per [ARCHITECTURE.md §3](ARCHITECTURE.md), `rust-toolchain.toml`, lint settings (`clippy -D warnings`, `rustfmt.toml`).
- [x] `yaoplm-api`: stubs `Request::Ping`, `Request::Version`, `JobEvent`; TS type generation via ts-rs.
- [x] `apps/cli`: `yaoplm --version`, `yaoplm rpc`.
- [x] `apps/server`: axum, `/health`, `/api/rpc`, `/api/events`, SPA serving; `--bind`, `--token`, `--root`.
- [x] `apps/desktop`: Tauri 2 window with the SPA, commands `rpc` and `subscribe`.
- [x] `web/`: Vite + React + TS + Tailwind + shadcn/ui, transport switcher, English UI, light and dark theme, empty library screen.
- [x] `.claude/launch.json` for previewing the web interface (Vite dev server and `yaoplm-server`).
- [x] Playwright smoke test against `yaoplm-server`.
- [x] CI `ci.yml` on Windows, Linux and macOS. Check that the core does not depend on tauri/axum/tokio.

Summary: PR #1, tag `v0.0.1`. Decisions along the way — ADR-0012…0015.

**Done when:** CI is green, `pnpm tauri dev` opens a window on Windows, `yaoplm-server` serves the same UI in the browser, `yaoplm rpc '{"type":"Ping"}'` responds.

## M1 — OPL formats (core without UI)
Goal: byte-exact handling of OPL formats, covered by tests.
- [x] For each format, first read the code in official OPL and the forks from `D:\yaoplm-testdata\refs\` (ADR-0015) and record the edge cases found in tests.
- [x] Fixtures in `yaoplm-test-support`: a tiny ISO with `SYSTEM.CNF`, an image without `BOOT2`, CD and DVD variants.
- [x] A bootable test ISO from an open-source homebrew ELF for checking on the console, plus a variant > 4 GiB with a filler file for `ul`.
- [x] ISO9660 reading and `SYSTEM.CNF` parsing exactly as in OPL (IMP-2). Edge-case tests: case, `cdrom0:\`, `;1`, IDs longer than 11 characters.
- [x] OPL naming rules: the old format `XXXX_YYY.ZZ.` and the 160/159 limits (IMP-4).
- [x] `USBA_crc32` with test vectors. Obtain the vectors from the OPL source (compile the C function locally in a comparison test or compute them by hand; see research/02 §5.3).
- [x] `ul.cfg`: reading and writing 64-byte records, chunk names, a 1 GiB splitting plan (IMP-5).
- [x] CFG: parser and writer that preserve unknown keys, CRLF, limits, round-trip tests (CFG-1).
- [x] ZSO: reading and writing (`ZISO` header, index, LZ4 blocks). Cross-check against `ziso.py` from the OPL repository.
- [x] ART file names and `CHT/`/`VMC/`/`APPS/title.cfg`, model and parsers only.
- [x] CLI: `yaoplm info <image>` (ID, medium type, size), `yaoplm scan <root>` (library output).
- [x] Fuzz targets for the ISO, CFG, `ul.cfg` and ZSO parsers.

Summary: PR #4, tag `v0.0.2`. Decisions along the way — ADR-0016…0018, findings — research/08. Cross-checking `yaoplm scan` against OPL on a flash drive prepared by OPL Manager has not been done yet: there is no such flash drive yet (test E: empty, `scan` on it matches OPL).

**Done when:** all tests are green. `yaoplm scan` on the user's real flash drive prepared by OPL Manager shows the same as OPL.
**PS2 check:** not needed, only `yaoplm scan` on an existing flash drive.

## M2 — Import to the device
Goal: a working flash drive can be prepared from the CLI.
- [x] Device and FS detection (DEV-1, DEV-2).
- [x] Folder structure creation (LIB-3).
- [x] Import pipeline: plan, then execution; copying with preallocation, progress and cancellation (IMP-7, IMP-9, IMP-10, IMP-11).
- [x] CD or DVD selection (IMP-3), names (IMP-4), deletion of `games.bin` (IMP-8).
- [x] FAT32 and a game >4 GiB → `ul` splitting with a journal (IMP-5); exFAT → a whole ISO (IMP-6).
- [x] BIN/CUE import with a single track → ISO (IMP-1).
- [x] Renaming, deletion, CD↔DVD move (GAME-1..3).
- [x] CFG backups (CFG-5).
- [x] CLI: `yaoplm add`, `yaoplm rename`, `yaoplm rm`, plan output and `--yes`.

Summary: PR #7, tag `v0.0.3`. Decisions along the way — ADR-0019…0021, findings — research/09. Test images for the console are in `D:\yaoplm-testdata\out\`. PS2 testing has not been done yet: we fix remarks about it in separate PRs.

**PS2 check:**
1. FAT32, CD game — visible and runs.
2. FAT32, DVD game up to 4 GiB — visible and runs.
3. FAT32, game larger than 4 GiB (`ul`) — visible and runs.
4. exFAT with OPL 1.2 beta, game larger than 4 GiB — visible and runs.
5. The same games on an SD card in MX4SIO — visible and run.
6. OPL launched from the memory card emulator sees games on the flash drive and MX4SIO the same as when launched from a Sony card.
7. A renamed game did not lose its settings.
8. The test homebrew ISO (CD and the `ul` variant > 4 GiB) is visible in the list.

## M3 — Desktop interface (MVP)
- [x] Library screen: grid and list, search, filters, virtualization (LIB-5).
- [x] Game card: ID, medium, size, paths, warnings (LIB-4).
- [x] Adding via a dialog and drag-and-drop, plan screen with confirmation, jobs panel.
- [x] Renaming, deletion and CD↔DVD move from the game card; recovery of an unfinished operation from the journal (GAME-1..3, ADR-0021). M2 has only the CLI.
- [x] Device selection, settings (profile, name format, theme).
- [x] Playwright e2e tests of the main scenarios against server mode, screenshots in the PR.

Summary: PR #9, tag `v0.0.4`. Findings on neighbouring interfaces — research/10. Added to the API: `ListDir` (browsing folders inside the server's roots) and the job step journal (`JobEvent::Log`, `JobInfo.log`). Interface settings are still in `localStorage` (ARCHITECTURE §6). The fixture library for e2e and previews — `make_library`. Scenario U1 has not yet been tested in the GUI on Windows or on the console: we fix remarks about it in separate PRs.

**Check:** the user goes through scenario U1 in the GUI on Windows. If possible, also on Linux or Steam Deck.

## M4 — Covers and metadata → **v0.1.0**
- [x] Built-in title database (META-1): a generation script from PCSX2 GameIndex and the libretro DAT, `data/ATTRIBUTION.md`.
- [x] Covers by ID: xlenore → libretro, re-encoding to PNG, profile sizes, cache (ART-1, ART-2 COV).
- [x] Manual replacement and bulk download (ART-3, ART-4).
- [x] "About" screen with source attribution.
- [x] Release build: `release-desktop.yml` (Windows, Linux, macOS ad-hoc), first pre-release.

Summary: PR #14, tag `v0.1.0`. Decisions along the way — ADR-0024 (the title database is embedded in the binary), ADR-0025 (cover search as a job, writing as a plan, backup by moving); findings on neighbours — research/11. New crate `yaoplm-net` (HTTP client with rate limit and ETag, `art-cache/` + `cache.sqlite` cache, source mirrors), requests `FetchArt`, `SetArt`, `DeleteArt`, `Attribution`, `ArtCache`, `ClearArtCache`, commands `yaoplm art …` and `yaoplm about`, the `/media/art` route on the server and the asset protocol on the desktop. A M3 bug was fixed along the way: the server returned 404 for direct links with a dot in the path. The updater is not connected: there is no signing key. Release v0.1.0 was published as a pre-release on 2026-10-23. Cover testing on the console is still ahead: the step-by-step list — [testing/m4-console-checklist.md](testing/m4-console-checklist.md); we fix remarks about it in separate PRs.

**PS2 check:** covers written by YAOPLM to a flash drive are visible in OPL 1.2 beta and in 1.1.0; a game without a cover shows the OPL placeholder; after replacement OPL shows the new cover.

## M5 — Dump verification, game settings, VMC, loaders → v0.2.0
- [x] Hashes in one pass, hash cache, Redump DAT (META-2, GAME-5; PR #21).
- [x] ISO↔ZSO↔`ul` conversions in either direction (GAME-4; PR #21).
- [x] CHD, zip, 7z and multi-track BIN/CUE import (IMP-1b; PR #21).
- [x] Core texts — phrases, key and parameters (ADR-0029): plan, transaction and job titles and API errors (PR #23); notifications, game warnings, medium tips, skip reasons, log lines and numbers (next PR).
- [x] CFG editor (CFG-2; PR #31, ADR-0036, research/15): editing by OPL menu rules, only what changed, with "before → after" keys in the plan; in simple mode modes 1–6 and VMC slot 1, in advanced mode everything (product owner's decision 2026-10-24); `yaoplm cfg`.
- [x] Creating formatted VMCs (VMC-1; PR #31, ADR-0036): byte-exact as OPL's genvmc, as a single file with preallocation, checking the fragment count after writing (Windows), assigning to a slot in the same plan, replacement only on explicit request with a backup; `yaoplm vmc ls`, `yaoplm vmc create`. Console check — `docs/testing/m5-pr2b-console-checklist.md`.
- [x] Remaining cover types, importing image sets (ART-2, ART-5; PR #25): eight slots in the game card with variants to choose from, sources by type, OPL Manager sets, folders and `art.tar` only for games on the medium (ADR-0032, ADR-0033).
- [x] Covers on import (ART-8, SPEC §5.4.1; PR #25, ADR-0031): a "Download covers" checkbox on the add screen, search by ID before confirmation, cover-writing transactions inside the import plan, `yaoplm add --covers`; cover-type slots in the game card with "Download", "Replace", "Delete" (ART-2, ART-3).
- [x] Dump verification on add (GAME-5; PR #25, ADR-0030; product owner's decision 2026-10-23, the milestone's second PR together with covers on import ART-8). Hashes are computed in the same pass as writing to the medium, without a separate read of the flash drive; after writing, the game immediately gets a Redump DAT verdict; without a DAT the hashes are saved to the cache for the future. The "Verify dumps on add" toggle in settings and on the add screen, on by default; in the CLI — a flag on `yaoplm add`. For CD from BIN/CUE and CHD, verify the raw data track as Redump hashes it: only then can a CD image get "verified" (an ISO never matches a CD, ADR-0026).
- [x] Titles from the database when adding games (META-1; at the product owner's request 2026-10-23; PR #25, ADR-0031): a toggle on the add screen, on by default; `yaoplm add --db-titles`.
- [x] A source for each image type (ART-1, ART-2, META-4; PR #29, ADR-0035, research/14; product owner's decision 2026-10-24): the OPL Manager copy on archive.org via Range requests (all eight types), ScreenScraper with a developer key, its own source order per type, packaging only its own region, other editions of the game for screenshots and logos, `YAOPLM_OFFLINE` in CI. Coverage has been measured; without a ScreenScraper key the 98% goal is not reached.
- [x] Interface polish and simple mode (UI-4, UI-5; PR #28, ADR-0034; product owner's remarks 2026-10-24, plan and prompt — `docs/plans/m5-ui-simple-mode.md`): a library grid without overlaps, walking all screens at different window sizes, simple mode on by default, quiet jobs, one "Download all images" button, add toggles only on the add screen.
- [x] Fragmentation check (IMP-7b, LIB-4; PR #34, ADR-0037): fragment counting on Windows, Linux (FIBMAP under root, otherwise "unknown") and macOS (for now "unknown" on FAT32/exFAT), the profile rule (64 in total on 1.2, each file in one piece on 1.1.0, VMC in one piece), warnings in the library and after writing; console check — `docs/testing/m5-fragmentation-console-checklist.md`.
- [ ] `yaoplm-server` and the Docker image (SRV-1).
- [x] Automatic loader profile detection by medium (LIB-1, SPEC §5.1.1, ADR-0023; PR #37, ADR-0039, research/16 — PR #36; signs by strength: the checksum of the OPL 1.0.0/1.1.0 ELF, family files, the `games.bin` generation, `conf_opl.cfg` keys, ELF names; exFAT/ZSO exclude 1.1.0; check — `docs/testing/m5-auto-profile-console-checklist.md`): installed loader → settings files → exFAT/ZSO → default; "Auto (by medium)" in settings by default, override per medium by volume serial number; the active profile and the reason on the device card and in the library header.
- [x] Loader catalogue as data (LDR-1, ADR-0022; PR #39, ADR-0042, research/17): OPL 1.1.0, OPL 1.2.0 beta, YAOPL — channel, assets, hash sources for verifying downloads, path presets (FreeMcBoot `UPDT/OPL.ELF`, PS2BBL `APPS/OPNPS2LD.ELF`, the application folder), descriptions in English, "Recommended" on OPL 1.2.0 beta; `yaoplm loader ls`. There is no hash history for new builds (product owner's decision 2026-10-25): the build and channel are named by the version inside the ELF (ADR-0041), `opl-stable.txt` — frozen releases only; the scheduled workflow and `HASHES.txt` in the fork's CI are not needed.
- [x] Build download with SHA-256 verification, cache, licence and changelog before download, installation from one's own archive (LDR-2; PR #41, ADR-0043): release details via `ETag`, the download is held to the hash the user saw, files only from the project's releases, checking size, SHA-256 and the version inside the ELF; ELF storage in the application data; one's own ELF, zip or 7z without network; the catalogue from `main` for the same program version; `yaoplm loader release|download|import|files|update-catalog`; mirrors in tests.
- [x] Installation on USB and MX4SIO: folders by profile, ELF by boot-method preset, backup, manifest, lines for FreeMcBoot and PS2BBL (LDR-3, BOOT-1; PR #43, ADR-0044): one plan (folders → ELF via a temporary file with a copy of the previous one, three copies per path → language pack → manifest → startup settings); the same ELF is not touched, another OPL is replaced, a foreign program — only with "Replace this file"; lines for FreeMcBoot, PS2BBL, PS2BBL-MX4SIO and OSDMenu; `yaoplm loader install`; the "Install loader…" wizard on the medium card.
- [x] Detecting the installed loader and version, "Update" only within the channel, rollback (LDR-4; PR #44, ADR-0045); device card states per SPEC §5.10.1; the automatic "Update available" window when a medium appears, from the catalogue cache without network, "Not now" is remembered, the button on the card opens the window again; the "Offer loader updates automatically" setting (on by default) turns the window off entirely, at startup and when a medium appears. The build is known by the manifest (while its hash matches the file), the hash of a frozen release or the version inside the ELF; the latest channel version — from the saved GitHub response, network only via "Check for updates"; an update changes only the ELF, language pack and manifest; rollback swaps the ELF and the latest copy; `yaoplm loader update|rollback`. "Replace…" as an explicit action — with LDR-7 (item below); until then "Reinstall…" opens the installation wizard.
- [ ] Helper process with elevated privileges and protected formatting of removable media (DEV-3, ADR-0013, research/06).
- [ ] Medium preparation wizard: "Install…" on an occupied or foreign flash drive offers to format (DEV-3, ADR-0013 protections) or keep the files and add OPL alongside; "Reinstall from scratch"; then folders and loader (DEV-4, after LDR-3).
- [ ] Beginner path in simple mode (UI-6, SPEC §5.11.2, ADR-0038; product owner's decision 2026-10-24, after DEV-4 and LDR-3): the "what next" step in the `InspectDevice` response and in `yaoplm device`; a hint with a main button on the device card, in the empty library and at the end of every write; "Prepare medium…" for media that formatting will fix; the preparation wizard's two questions and FS choice by OPL rules ("Don't know" → FAT32); the "OPL on the memory card" mark per medium; "How to run on PS2"; the hints toggle in settings.
- [ ] Build and version selection (LDR-6, SPEC §5.10.2, ADR-0038; product owner's decision 2026-10-24): cards for OPL 1.1.0, OPL 1.2.0 beta and YAOPL with a description and "Recommended" (`summary`, `recommended` in the catalogue); version, release date, size, short changelog, links to the release and commit — in the installation wizard and in the "Update available" window, along with "What changed since your version"; "Other version…" in advanced mode by repository releases (upstream — `latest` and `v1.1.0`).
- [ ] Versioned YAOPL releases (ADR-0038; task in `belomaxorka/yaopl`, product owner's decision 2026-10-24): CI publishes every build as an immutable release with a version tag next to `latest`, `FORK.md` describes this; after that YAOPL appears in "Other version…".
- [ ] Build replacement (LDR-7, SPEC §5.10.3, ADR-0038; product owner's decision 2026-10-24): OPL 1.1.0 ↔ 1.2.0 beta ↔ YAOPL in one plan preserving games and settings; folders of the new build are created; the "What will change for games" section: when moving to 1.1.0 — ZSO with conversion to ISO, games not in one piece, exFAT (replacement unavailable, the path goes through preparation anew); when leaving 1.1.0 — JPG and BMP covers with re-encoding to PNG; the loader changes last; "Restore previous build".
- [ ] Before the v0.2.0 tag: the UI consistency pass ([plans/release-ui-consistency.md](plans/release-ui-consistency.md); product owner's decision 2026-10-25): device card, library header, "What next" hints, profile questions, notifications and buttons per the state matrix, both modes, both themes, English, axe; beforehand — a "removable medium" fixture for E2E and a decision on the "medium for PS2" and "folder for SMB" modes.
- [ ] Findings from walking the interface on a real flash drive (2026-10-24, plan — `docs/plans/m5-walkthrough-findings.md`): disc type from the Redump DAT and title database, not from the image size; an honest "remaining" estimate at the start of a write; paths without `\\?\` and clear skip reasons; the result of the image search per plan game. Desktop app walkthrough (same place, items 6–11): disc type by the game with the same ID, `Title` in CFG on rename (META-3), contrast of checkboxes and fields, ID from `SYSTEM.CNF` in the rename window, dump verdict after conversion.

**PS2 check:**
1. A ZSO game runs.
2. Compatibility modes are applied.
3. VMC works: a save is created and read.
4. A game from an SMB share on a home server is visible and runs.
5. A flash drive and an SD card formatted and prepared by YAOPLM from scratch work in OPL.
6. OPL installed by YAOPLM on a flash drive (`UPDT/OPL.ELF`) launches from the standard FreeMcBoot menu item without editing the memory card.
7. After "Update" the flash drive has the new YAOPL build (the version is visible on the OPL screen), settings and games are in place; "Roll back" restores the previous one.
8. A flash drive with a foreign OPL of an unknown version is recognized as "OPL, version unknown", and an update is not forced on it.
9. A flash drive prepared in simple mode using only the "What next" hints (boot method "Don't know", i.e. FAT32) launches OPL from FreeMcBoot and shows the added games with covers.
10. OPL v1.1.0 installed via "Other version…" shows version 1.1.0 on the OPL screen.
11. After "Replace…" from YAOPL to OPL 1.1.0 with ZSO → ISO conversion, all games are visible and run; "Restore previous build" brings YAOPL back, games are in place.
12. After "Replace…" from OPL 1.1.0 to 1.2.0 beta, the covers that were JPG are visible.

## M6 — PS1, APPS, RiptOPL, cheats → v0.3.0
- [ ] PS1: VCD and POPStarter (PS1-1); APPS (APP-1).
- [ ] RiptOPL profile: 8-bit PNG, sizes, `art.tar`, extra CFG keys and bits (ART-6).
- [ ] CHT editor (CHT-1), compatibility presets (CFG-3), metadata in CFG (META-3).
- [ ] Game information (META-3, at the product owner's idea 2026-10-23): genre, developer, publisher, release date, number of players and a short description on the game page and in the add plan; writing the same fields to CFG, from where the OPL theme's information page shows them (`Title`, `Genre`, `Release`, `Developer`, `Description`, `src/themes.c`). Sources: the Tom-Bruise CFG database (GPL-3) and Wikidata (CC0), descriptions from Wikipedia (CC BY-SA) with attribution; download on request and with a cache, like covers (ADR-0007). Before starting — research into the sources and an ADR.
- [ ] Installing and updating forks (RiptOPL as the `APPS/APP_RIPTOPL/` package, wOPL, the MMCE fork) and games on the memory card emulator SD (LDR-1, LDR-3, LDR-4 for forks).
- [ ] RiptOPL in build selection and replacement (LDR-6, LDR-7, SPEC §5.10.2–§5.10.3; product owner's decision 2026-10-24): a description and the variants `rolling`, `rolling-korium`, `rolling-ra`, `current-fan-favorite` instead of version selection; transitions to RiptOPL and back with the "What will change for games" section (PS1 games in `POPS/` and `EMBER/`, bits 0x40/0x80 and RiptOPL keys, `art.tar`, covers deeper than 8 bits); version selection for wOPL and the MMCE fork.

**PS2 check:** RiptOPL shows covers prepared by YAOPLM; games from the memory card emulator SD run via MMCE.

## M7 — Public release
- [ ] Make the repository public. First re-check that there are no secrets in the history.
- [ ] Signing: SignPath (Windows); Apple Developer if needed.
- [ ] Flatpak (Steam Deck), Homebrew cask, winget.
- [ ] User documentation in English, announcement on psx-place.

## Later
Agent's proposal (2026-10-23, not agreed as a requirement): import from a DVD drive, i.e. ripping the owner's own disc directly to the medium with a title from the database and a cover in one plan (IMP-1d); OrbitPS2 Manager did this in October 2026 (`optical-drive.service`). An ADR and a drive-reading prototype on three OSes are needed.

Network library (discussion with the product owner 2026-10-24, a separate milestone after M5; research and an ADR before starting):
- Scheme: the folder is shared over SMB by the operating system or a NAS (OPL understands only SMBv1, dialect `NT LM 0.12`), and YAOPLM — the desktop or `yaoplm-server` — opens this folder as a library and fills it. The layout is the same as on a flash drive (`CD/`, `DVD/`, `ART/`, `CFG/`, `VMC/`; ISO and ZSO, `ul` not needed); the file name limit of 159 bytes is already respected. We will not embed Samba into the program.
- Sharing assistant (NET-2) per OS: Windows (sharing and the "SMB 1.0/CIFS Server" component — the program only checks whether it is enabled and honestly warns about security), Linux and Steam Deck (a Samba settings fragment with SMBv1), macOS (Samba from Homebrew or Docker), NAS (minimal SMB1 protocol).
- Writing OPL network settings (`conf_network.cfg`: `smb_ip`, `smb_share`, `smb_user`, `smb_port`) to a flash drive or memory card so that nothing has to be entered on the console; checking the share — "will OPL see it".
- `yaoplm-server`: open a subfolder of the root as a library directly from the browser (without leaving the root), convenient launch (shortcut or service), Docker image (SRV-1), instructions for running with a token and a share.
- Desktop: the "Control from other devices" toggle — the program's web interface on the local network while it is running; off by default, address and access code on the screen (ADR-0009, ADR-0012). It does not affect PS2 and Samba.
- Fallback if SMBv1 setup proves too heavy: a built-in minimal SMB1 server on a non-standard port (the SMB port in OPL is configurable, `smb_port`), so as not to conflict with the system SMB on 445.

SMB assistant (NET-2), internal HDD via hdl-dump/pfsshell, then natively (HDD-1), NBD (HDD-2), game information from ScreenScraper (META-4; images — ADR-0035), pnach → cht (CHT-2), VMC save manager (VMC-2), installing the loader inside the memory card image on the emulator SD (LDR-5), PSBBN covers and HDD-OSD icons (ART-7), CSV/HTML export (LIB-6), NHDDL/Neutrino and PSBBN profiles, global CFG (CFG-4).
