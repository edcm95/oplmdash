# Plan: interface walkthrough findings on a real stick (M5)

Source — the interface walkthrough for the PR "interface polish and simple mode" (UI-4, UI-5, `docs/plans/m5-ui-simple-mode.md`), 2026-10-24. Interface findings were fixed in that PR; this file covers what relates to the core or needs separate work. The product owner's decision: handle it in a separate session at the end of M5.

## How we looked

- Test collection `D:\yaoplm-testdata\out\ui-collection`: 20 files, images with wLaunchELF inside (homebrew, AFL-2.0) and real game IDs, built with `make_boot_iso --id` and `pack`. There is an ISO (one at 700 MiB), BIN/CUE, CHD, zip, 7z, a zip with two games, a broken zip, random bytes with an `.iso` extension and a text file. The program sees them as real games: titles from the database, pictures from the network, Redump verification (the verdict — "bad dump", hashes do not match).
- `yaoplm-server` with the roots `E:\` (the owner's test stick, FAT32, serial 7E1CD2C5) and the collection folder; browser — Playwright. Simple mode, English, pictures — from the network, without mirrors.
- 15 games added in one plan with picture search: 60 pictures found in 56 s, writing 711 MiB — 3 min 7 s.

## Findings

### 1. Disc type is guessed from the image size

- **What is visible.** The plan places the Gran Turismo 4 image (`SCUS_906.82`, 700 MiB) into `CD/`, although it is a DVD game. With a real GT4 dump (4 GiB) this will not happen, but small DVD games (there are 300–600 MB ones) will land in `CD/`, and OPL will read them in CD mode.
- **What to do.** First prior art (rule 6): how OPL Manager, OPL PC Tools, ulmake and hdl-dump solve it. Then take the disc type from the Redump DAT (CD games have `.cue` and `.bin`, DVD have `.iso`) or from the title database if the ID is there; size — only a fallback. Show in the plan where the type came from; the CD/DVD override in the plan already exists. Test: a small image with a known DVD game's ID lands in `DVD/`.

### 2. "25 min remaining" for a write that took 3 minutes

- **What is visible.** At the start of the write the floating panel showed 478 KiB/s and "25 min 18 s remaining": the first operations are small pictures, where FAT32 has a large per-file overhead. The job finished 3 minutes later.
- **What to do.** Do not show an estimate until enough has been written (by time or bytes), smooth the speed, account for the file count separately from bytes. Check on a real stick with a 1–4 GiB image.

### 3. A technical path in a skip reason

- **What is visible.** A broken zip in the plan: "Cannot read: `\\?\D:\yaoplm-testdata\...\Broken download.zip`: zip: invalid Zip archive: Could not find EOCD". The user-facing text contains the `\\?\` prefix and the library's message in English.
- **What to do.** Remove the `\\?\` prefix everywhere a path reaches a phrase (`display_path` in `yaoplm-api`); for common reasons (archive not fully downloaded, corrupted, encrypted) — a dedicated phrase in `reasons.*`, with the technical detail after it (I18N, rule 8).

### 4. The picture search result is not recomputed when a game is removed

- **What is visible.** After removing one game from the source list, the "Pictures" panel under the plan shows the same "Pictures found: 60, not found: 4": the numbers refer to the search, not to the plan. What is written, however, is correct — only the pictures of the remaining games.
- **What to do.** Compute the result for the games of the current plan (found and not-found pictures by their IDs) or label it as the search result. A small interface fix, but it changes `CoverSearchPanel` and the `FetchArt` response (counting by ID).

### 5. The grid was scrolled once

- **What is visible.** Right after writing 15 games the library grid showed the second row at the top and the first was cut off. After a reload and rescan — no; could not reproduce.
- **What to do.** Watch during the next walkthroughs; if it repeats — record the steps and fix position saving in `GameGrid` (`firstShown`, `virtualizer.measure()`).

## Non-findings

- The "bad dump" verdict for all collection images is expected: they contain homebrew, not games.
- A separate task — picture sources for all OPL types (the `art-sources` session, its own PR and ADR).

## Desktop app walkthrough (2026-10-24, second part)

Debug build of `yaoplm-desktop.exe` with its own WebView2 profile, the test library and mirrors in `D:\yaoplm-testdata\out\desktop-walkthrough` (`make_library`, homebrew images from `D:\yaoplm-testdata\out\m5-pr2b`), `YAOPLM_OFFLINE=1`. Both modes, both themes and English were walked: library, game card, rename from the database, cover from a file, CD↔DVD move, game settings in simple and advanced mode (RiptOPL modes, VMC slots, GSM), adding a folder with ISO, BIN/CUE and duplicates, Redump DAT import, source mirrors, downloading and importing pictures, dump verification, conversion to ZSO, jobs, devices, deletion.

Fixed in the PR `m5/desktop-walkthrough-fixes`:

- library search lost letters during fast typing (`BrowserRouter` updated `?q=` in a React transition);
- GSM offsets accepted any number, although the OPL menu offers −100 to 100 (`src/dialogs.c`), and "−" could not be typed into the field;
- in the add plan without a network every game had a paragraph of addresses and English messages; now — source names, with the full text in the job log;
- the "More" menu and the picture slot menu wrapped items onto two lines; the VMC card list cut off the label;
- an invalid name for a new VMC card blocked the button without explanation.

Not fixed:

6. **Disc type from another game with the same ID.** Addition to item 1: the plan places "Tak - The Great Juju Challenge" (`SLUS_212.18`, an image without UDF and CD-XA) into `CD/`, although the device already has a game with this ID in `DVD/`. Such an ID is another source of the type, before size.
7. **Renaming does not touch `Title` in CFG.** After renaming "Retail Game" → "Grand Theft Auto III", `Title=Retail Game` remained in `CFG/SLUS_200.62.cfg`, and the OPL theme's information page shows the old title. This is META-3 (v0.3): write `Title` together with the other information.
8. **Barely visible checkboxes and fields in the light theme.** The border of an unchecked checkbox, toggle and input — `--input` (`oklch(0.922 0 0)`) on white, contrast about 1.3:1 against a 3:1 requirement (WCAG 1.4.11). Worst in the delete dialog: the "I understand that files will be deleted" checkbox is almost invisible. The owner's decision is needed: strengthen `--input` for all fields or only for checkbox and toggle borders.
9. **ID from SYSTEM.CNF in the rename dialog.** The warning "the file name has SLES_999.99, but SYSTEM.CNF has SLUS_200.62" exists, but the rename dialog does not offer to take the ID from the image: it must be typed by hand.
10. **Dump verdict after conversion.** After conversion to ZSO, games lose the verification icons ("not checked yet"), although the image data is the same. The verdict and hashes could be carried over during conversion or recomputed during compression.
11. **Interface small things.** Column headers as a list do not sort (sorting — only through the "Title" list); after deleting a game the library opens with the default sort; the add plan appears below the window edge and must be scrolled to by hand.
