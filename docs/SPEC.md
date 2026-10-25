# YAOPLM — product specification

> Yet Another OPL Manager. Document version 0.2, 2026-10-22: added medium preparation, formatting, loader installation and the RiptOPL cover specification.
> Based on the research in [`docs/research/`](research/README.md). Requirement IDs (`IMP-5`, `ART-1`, etc.) are used in tasks, commits and PRs.

## 1. What it is

YAOPLM is an open-source, cross-platform PlayStation 2 game library manager for Open PS2 Loader (OPL) and its forks. The program prepares storage so that games appear in OPL right away: correct IDs and names, covers, compatibility settings, splitting large games for FAT32. Storage means a USB flash drive or disk (FAT32/exFAT), an SMB share, and later the PS2 internal HDD.

One Rust core and three interfaces:
- **desktop** (Tauri): Windows, Linux (including Steam Deck), macOS;
- **CLI** `yaoplm` for scripts and automation;
- **server** `yaoplm-server`: the same interface in a browser, running in Docker on a home server.

## 2. Positioning

| Program | What is missing (details in research/01 and research/05) |
|---|---|
| OPL Manager V24 | Closed source, Windows only, frozen since 2024-09-15 on .NET 6 (support for .NET 6 has ended). The cover server is private. It has optional (opt-out) telemetry |
| OrbitPS2 Manager 1.3.1 (the best of the open ones) | Does not write `ul.cfg`, so games >4 GB cannot be installed on FAT32. No internal HDD, cheats, translations, CLI or web mode (#24 closed as wontfix). VMC — an empty file. Covers come from one GitHub repository via the API with a request limit |
| OPL PC Tools 4.0 | No macOS, PS1/APPS or internal HDD |
| OPLNova 2.0.4 | Minimal features and critical bugs (clicking a game erases its CFG, Delete removes an ISO without confirmation). A fresh install has neither covers nor titles |

**Key differences of YAOPLM:**
1. Native operation on Windows, Linux and macOS, without .NET or Wine.
2. One core and three interfaces: desktop, CLI and web server.
3. A complete game preparation cycle: ISO, ZSO, `ul` splitting for FAT32, BIN/CUE, CHD, archives.
4. **Loader profiles**: OPL 1.2.0 beta, OPL 1.1.0, RiptOPL; later NHDDL/Neutrino, POPStarter, PSBBN.
5. Dump verification against Redump, correct titles, duplicate search.
6. Several legal cover and metadata sources with a cache and offline import. No dependence on someone else's private server.
7. Data safety: first a change plan, then confirmation, then execution. Unknown CFG keys are preserved; there is no disk formatting in v1.
8. No telemetry.
9. English-only interface from the first release.
10. Preparing a medium from scratch: protected formatting, folder structure and installation of the selected loader (official OPL or a fork) for the user's boot method: flash drive, MX4SIO, memory card emulator SD card.

## 3. Users and scenarios

- **U1. Flash drive.** The user drops in a folder of images and gets a ready flash drive with covers.
- **U2. Large library** on an exFAT disk or internal HDD via a USB adapter: bulk operations, dump verification, additional cover downloads.
- **U3. Network.** The library lives on a NAS or home server, OPL loads it over SMB, and management goes through the browser via `yaoplm-server`.
- **U4. Linux, Steam Deck, macOS**: everything the same as on Windows.
- **U5. Automation**: CLI and JSON-RPC.
- **U6. PS2 internal HDD** (APA/PFS). Later.

## 4. Loader profiles and storage types

| Profile | Covers | ZSO | exFAT | Features |
|---|---|---|---|---|
| **OPL 1.2.0 beta** (default) | PNG only, ≤1024 px | yes | yes | up to 64 fragments per game, `games.bin` cache |
| **OPL 1.1.0** (stable, 2021) | PNG/JPG/BMP | no | no | on FAT32, games >4 GiB only in `ul` format |
| **YAOPL** (the owner's fork, `belomaxorka/yaopl`) | same as 1.2.0 beta | yes | yes | the format rules are the same as 1.2.0 beta: there is no separate profile; in the loader catalogue it is a separate build with its own update channel (ADR-0018, ADR-0022) |
| **RiptOPL** (fork) | 8-bit indexed PNG (requirement unconfirmed), its own sizes, optional `ART/art.tar` | yes | yes | extra CFG keys `$CoreLoader`, `$Neutrino*`, `$Ember*`, `$VMCDisable_N`; compatibility bits 0x40/0x80; PS1 in `POPS/` and `EMBER/games/<name>/` |
| wOPL (later) | same as 1.2.0 beta, `art.tar` | yes | yes | MMCE since 2026-02-28; its own CFG format on libconfig (see CFG-1) |
| OPL fork with MMCE (later) | same as 1.2.0 beta | yes | yes | branch `mmce_support` of the `ps2-mmce` fork, latest builds — 2025 betas (research/07) |
| NHDDL/Neutrino (later) | — | no | yes | YAML configs, ISO only; supports MMCE |

Covers are always written as PNG: this works on both 1.1.0 and 1.2.0 (ADR-0005).

**Media** (as the loader sees them, research/02 §2):
- USB flash drive or disk: BDM `usb`, FAT32 or exFAT (exFAT only with 1.2.0 beta).
- SD card in an **MX4SIO** adapter (memory card slot): BDM `sdc`, enabled in OPL by the `enable_mx4sio` setting. Layout as on a flash drive.
- **Memory card emulator** SD card (SD2PSX, MemCard PRO 2 and similar, with a screen and buttons). It holds memory card images, and with MMCE firmware also games. Only loaders with MMCE (OPL fork, NHDDL/Neutrino) see games on it; official OPL does not.
- iLink and an internal HDD formatted as exFAT (BDM `sd` and `ata`).
- Any local folder, including an SMB share mounted by the OS, or a folder on the server in `yaoplm-server` mode.
- Later: a direct SMB client, internal HDD (APA), a disk image.

## 5. Functional requirements

Stages: **MVP** — by v0.1, **v0.2**, **v0.3**, **later** — after v0.3. Milestone mapping — in [ROADMAP.md](ROADMAP.md).

### 5.1 Library and scanning
| ID | Requirement | Stage |
|---|---|---|
| LIB-1 | Open a device root or folder. Determine the FS, free space and loader profile from the medium's files (§5.1.1) | MVP (FS and space), v0.2 (profile) |
| LIB-2 | Read an existing library: `CD/`, `DVD/` (`.iso`/`.zso`), `ul.cfg` with `ul.*` chunks, `ART`, `CFG`, `VMC`, `CHT`, `APPS` | MVP |
| LIB-3 | Create the folder structure `CD DVD CFG CHT VMC ART THM LNG APPS` taking `usb_prefix` into account | MVP |
| LIB-4 | Library check with warnings: no `BOOT2`, name longer than the limit, non-ASCII name for `ul`, JPG covers on 1.2, duplicates, stale `games.bin`, `._*` files from macOS (they slow down OPL scanning), fragmentation | MVP (basic), v0.2 (fragmentation) |
| LIB-5 | Search, filters, sorting; cover grid and list; virtualization for 3000+ games | MVP |
| LIB-6 | Export the list to CSV/HTML, statistics | later |

#### 5.1.1 Automatic loader profile detection

The profile (§4) is determined for each medium from its files, without network; the first level where the signs agree decides (ADR-0023, ADR-0039, ADR-0040, ADR-0041, research/16):
1. Installed loader → "exact":
   - an ELF with the SHA-256 of an official release: currently the table of OPL 0.5–0.9.3, 1.0.0 and 1.1.0 in `data/loader-hashes/opl-stable.txt`; there is no hash history for new builds: the version string identifies them (ADR-0042);
   - or the version string inside the ELF, unpacked from the ps2-packer LZMA stream. 1.0, 1.1 and upstream betas before build 1871 get the 1.1.0 rules. The ELF family is given by the program's own strings (`settings_riptopl.cfg` for RiptOPL, `wopl_settings.cfg` for wOPL), not by the file name;
   - two ELFs that disagree at this level decide nothing;
   - the YAOPLM installation manifest (from LDR-3) decides nothing by itself: it only confirms a build if the SHA-256 of the file at the recorded path matches the recorded one. If it does not match (the user replaced the ELF themselves), the manifest is stale: the actual files decide, and the medium card says so.
2. Files written by only one family: RiptOPL — `title.cfg` with `Version=`, `settings_riptopl.cfg`, `conf_riptopl.cfg`, `config.path`, `favourites.bin`, `ART/art_cache.bin`; wOPL and uOPL — `wopl_*.cfg`, `conf_wopl.cfg`, `conf_uopl.cfg`; NHDDL — `nhddl/cache.bin`, `nhddl/lastTitle.bin` (the `nhddl/` folder alone is not enough: the PSBBN installer creates it too).
3. `CD/games.bin`, `DVD/games.bin`: 92-byte records were left by OPL 1.1.0 or a beta before build 2016, 188-byte records by a beta from build 2017 or a fork.
4. `conf_opl.cfg`: keys that 1.2 always writes (`bdm_cache`, `enable_bgm`, `x_sensitivity`, `enable_bdm_hdd`, etc., as well as `mmce_*` of YAOPL and the MMCE fork) → 1.2.0 beta; OPL keys without them → 1.1.0 rules. The absence of `conf_opl.cfg` means nothing: official OPL stores it primarily on the memory card.
5. ELF names: `RIPTOPL*` → RiptOPL, `WOPNPS2LD*` → wOPL, `nhddl*` → NHDDL, `OPNPS2LD*` and `OPL.ELF` → the OPL family.
6. exFAT or `.zso` images exclude OPL 1.1.0 for levels 3–5, and without other signs give 1.2.0 beta. JPG covers in `ART/` are only a hint when the profile is taken by default.
7. Without signs — OPL 1.2.0 beta.

wOPL, uOPL, NHDDL, the MMCE fork and YAOPL get the 1.2.0 beta rules until they have their own profiles. Conflicting signs at the same level decide nothing but produce a notification on the medium card. Three more notifications:
- a `games.bin` of a different generation than the detected loader: "the console may be running another copy";
- files of another family next to the loader whose ELF decided: "probably leftovers from a previous installation";
- a file with the loader's name that is not a PS2 program: "possibly damaged or not fully copied".

**The question when there are no signs** (ADR-0041). If the profile is taken by default, the medium root has OPL folders (except a single `APPS/`) or `ul.cfg`, and the medium is not marked "Not suitable for OPL" (DEV-2), the medium card and the library ask in one line, without a window: "Which OPL do you run from this medium?".
- Answers: OPL 1.2 or YAOPL, OPL 1.1.0, RiptOPL, "Don't know (rules for any OPL)".
- The answer is remembered as the choice for the medium; "Don't know" gives the OPL 1.1.0 rules, which every version reads.
- An empty or foreign medium does not get the question: it first gets a loader installed (§5.11.2), and that decides the profile.
- If the medium's files later say something different from the choice, the line "From the medium's files this is …" offers to return to "Auto".

The result is the `loader` field in the `ListDevices` and `InspectDevice` responses: the active profile, the source (override for the medium, explicit request profile, detection), the medium key and what was found (profile, confidence level "exact", "by indirect signs", "by default", reason, loaders, notifications). There is no global profile setting (ADR-0040): each medium gets a profile from its own files, and if they are silent or wrong, the profile is chosen on the medium card ("Change"), and the choice is remembered for this medium. An explicit request profile exists only in the CLI (`--profile`). The medium key is the volume serial number for USB and SD, the path for folders and network folders; overrides are stored in the interface settings. CLI: `--profile auto` by default. The device card and the library header show the active profile and the reason: "OPL 1.2.0 beta: by installed YAOPL", "RiptOPL: by settings_riptopl.cfg", "OPL 1.2.0 beta: by default". Scanning and plans receive the already active profile, as now. The signs are the same as for loader detection, so it is done together with LDR-4 (M5).

### 5.2 Adding games (import pipeline)
| ID | Requirement | Stage |
|---|---|---|
| IMP-1 | Sources: `.iso`, `.zso`, `.bin/.cue` with one data track, folders recursively, drag-and-drop | MVP |
| IMP-1b | `.chd`, `.zip`, `.7z`, multi-track BIN/CUE (data — track 1, audio tracks discarded); archives are read without unpacking to disk and are not deleted (ADR-0028) | v0.2 |
| IMP-1c | `.rar` (optional, due to the UnRAR licence), `.nrg`, `.mdf` | later |
| IMP-2 | The game ID is determined **exactly as in OPL**: `SYSTEM.CNF` → the `BOOT2` line (case-sensitive), take the text after `:`, strip `\`, read up to `;`, keep ≤11 characters. OPL silently does not show an image without `BOOT2`, so we warn | MVP |
| IMP-3 | CD or DVD selection by the medium type in the image (size heuristic plus manual correction) | MVP |
| IMP-4 | File names: by default `SLUS_200.62.Title.iso` (OPL does not mount the image and scans faster, works on 1.1.0 too), optionally `Title.iso`. Cleaning of invalid characters. Limits: title ≤160, file name ≤159 bytes for SMB | MVP |
| IMP-5 | FAT32 and a game >4 GiB → `ul` format: chunks of exactly 1 GiB (except the last), names `ul.%08X.%s.%02x`, `USBA_crc32` hash, a record in `ul.cfg` (64 bytes). Title ≤32 ASCII, ≤10 parts on SMB | MVP |
| IMP-6 | exFAT and other FS → a regular ISO or ZSO in `CD/` or `DVD/` | MVP |
| IMP-7 | Sequential writing with space preallocation, speed display | MVP |
| IMP-7b | Fragmentation check per OS: ≤64 fragments **per game in total across all parts** on BDM. OPL checks the limit per part, so we control the total ourselves; for OPL 1.1.0 each game file must be one piece, VMC — one piece on any build. Where the OS does not report the layout (Linux without root), the fragment count is "unknown" (research/06 §1.11, ADR-0037) | v0.2 |
| IMP-8 | Delete `games.bin` in changed `CD/` and `DVD/` folders | MVP |
| IMP-9 | Job queue: progress, cancellation, batch import | MVP |
| IMP-10 | Dry-run: show the plan before writing (what, where, how much space) | MVP |
| IMP-11 | Copy or move. The source is deleted only on explicit choice | MVP |

### 5.3 Game management
| ID | Requirement | Stage |
|---|---|---|
| GAME-1 | Rename: the image name or `ul` chunks and the `ul.cfg` record. CFG, ART and CHT are moved together with the game ID change; VMC references are preserved, the cards themselves are not renamed (ADR-0020) | MVP |
| GAME-2 | Deletion only after confirmation (the game and, optionally, CFG, ART and CHT if the ID is not used by another game; VMC — never, ADR-0020) | MVP |
| GAME-3 | Move a game between `CD` and `DVD` | MVP |
| GAME-4 | ISO ↔ ZSO ↔ `ul` conversions in either direction; the old form is deleted after the new one is written and with confirmation (ADR-0027) | v0.2 |
| GAME-5 | Integrity check: CRC32 and SHA-1 (MD5 optionally) in one pass over the file, comparison with Redump. Statuses: verified / unknown (with a reason: no DAT, not in Redump, CD image) / bad dump / failed (the image cannot be read); an ISO never matches a CD in Redump (ADR-0026). When adding games, the check runs automatically, in the same pass as the write (toggle on the add screen, on by default and remembered; product owner's decision 2026-10-23, ADR-0034) | v0.2 |

### 5.4 Covers and metadata
| ID | Requirement | Stage |
|---|---|---|
| ART-1 | Download images by ID, the source is chosen silently, each type has its own order (ADR-0035): `xlenore/ps2-covers` via raw URL (not via the GitHub API), a backup of the OPL Manager database on archive.org (`OPLM_ART_2024_09`, table of contents once, then a Range request per image), `libretro-thumbnails` by the Redump name, ScreenScraper with a developer key (META-4). Packaging (COV, COV2, LAB, ICO) — only its own region | MVP / v0.2 |
| ART-2 | Re-encoding to PNG and fitting to profile sizes, ≤1024 px. Types: `COV` (MVP), `ICO`, `COV2`, `LAB`, `LGO`, `SCR`, `SCR2`, `BG` (v0.2) | MVP / v0.2 |
| ART-3 | Manual replacement: from a file, drag-and-drop, paste from the clipboard | MVP |
| ART-4 | Bulk download: "missing only" or "overwrite" | MVP |
| ART-5 | Import image sets: OPL Manager backups from archive.org in zip or 7z (the user downloads them themselves), folders, `art.tar`. Only images of games on the medium are taken (product owner's decision 2026-10-23), with OPL Manager numbering (`SCR_00`, `SCR_01`, `BG_00`), through a plan with a backup (ADR-0033) | v0.2 |
| ART-6 | RiptOPL profile per the fork author's specification (research/05 §3.4, checked against the code in research/07): PNG not deeper than 8 bits per channel (RiptOPL does not load 16-bit; an indexed palette is optional and saves memory); name keys PS2 `<ID>`, VCD `<name without .VCD>`, Ember `<folder in EMBER/games>`, APP `<ELF name with .ELF>`; COV and ICO twice the standard size (280×400, PS1/APP 400×400, 128×128), other types standard; `art.tar` packaging; placement in `ART/` on BDM and in `+OPL/ART/` or `__common/OPL/ART/` on APA | v0.3 |
| ART-7 | Covers for PSBBN (`PP.<entry>/res/jkt_001.png`) and HDD-OSD icons (`list.ico`, `del.ico`, `icon.sys` in the `PP.<entry>` partition) | later (together with HDD-1) |
| ART-8 | Images on import: the "Download images" toggle on the add screen (on by default, remembered) searches all types available from the enabled sources (ADR-0034). The search runs by image IDs before confirmation, and the import plan gets a cover-writing transaction per game: one confirmation, one job, the image and the cover land together. Covers not found are listed in the plan and remain for "Download images" in the library later; import is never blocked by a missing cover or network (§5.4.1) | v0.2 |
| META-1 | Built-in title database from PCSX2 GameIndex (GPL-3.0) and the PS2 DAT from libretro-database (CC BY-SA 4.0), with attribution. When adding games, the "Titles from database" toggle (on by default) gives the game a title from the database if the ID is there, otherwise from the file name (ADR-0031) | MVP |
| META-2 | Redump DAT with serial numbers (`redump.info`). Download only by button and at most once a week (counting any responses), cache, import of a manually downloaded DAT; Redump does not support conditional requests (ADR-0026) | v0.2 |
| META-3 | Game information (`Title`, `Genre`, `Release`, `Developer`, `Description`, publisher, players): display on the game page and in the add plan, write to CFG (the OPL theme's information page shows them); sources — open CFG databases (Tom-Bruise, GPL-3), Wikidata (CC0), descriptions from Wikipedia (CC BY-SA) | v0.3 |
| META-4 | ScreenScraper as a source of all image types with a region (ADR-0035): works in a build or environment with the project's developer key, a user account is optional; without a key the source is off. Game information from ScreenScraper — later | v0.2 / later |

#### 5.4.1 Covers on import and per-type editing

Product owner's decision (2026-10-23): covers should be fetched automatically and edited manually as desired. Refines ADR-0025.

**Automatically on import (ART-8, v0.2).**
- On the "Add games" screen the "Download images" toggle is on by default and remembered (only on this screen, ADR-0034). It searches all types available from the enabled sources (currently COV, SCR, SCR2, LGO); the plan lists only covers not found. The network is enabled only because the checkbox is on (ADR-0007); without the checkbox or without enabled sources, import works as now.
- Order: the import plan already knows each image's ID (the transaction stores `startup`). After "Preview plan" the same search job runs as for "Download covers" (`FetchArt` with `import: <plan id>`, missing only, per-game log right under the plan, cancellation). When the search finishes, the import plan is rebuilt with its result: after each game's copy transaction comes a transaction writing `ART/<ID>_COV.png` from the cache (as in ADR-0025). While the plan is being rebuilt, "Confirm" is unavailable. One confirmation, one job; the job log shows both the image and the cover for each game.
- API: the search ends not with a plan but with a set of covers (`JobResult::Covers`); `PlanImport` receives its id (`covers`), the core accepts a list of ready cover files (`ArtItem`) and adds the transactions itself; CLI — `yaoplm add --covers`. Duplicate IDs within a batch get one cover (ADR-0031).
- If a cover is not found or a source is unavailable, the game is still imported, and the reason goes into the plan's "skipped" list; later it can be downloaded via "Download images" in the library (ART-4) or set manually (ART-3). The search can be skipped ("Skip covers") or retried after a failure ("Retry search").
- Next to the covers checkbox — the "Titles from database" (META-1) and "Verify dumps on add" (GAME-5, ADR-0030) toggles; both are on by default and remembered.

**Manually by type (part of ART-2 and ART-3, v0.2).**
- In the game card, instead of one cover — a row of slots: COV (box), ICO (disc), COV2 (back), BG (background), SCR and SCR2 (screenshots), LGO (logo), LAB (spine). Each slot has a preview, size and the buttons "Download", "Replace" (file, clipboard, drag-and-drop), "Delete"; each action is the same plan with a backup (ADR-0025).
- Sources by type (ADR-0035, research/14), in this order:

  | Type | Sources |
  |---|---|
  | COV | xlenore (and 3D covers `covers/3d/` as a second variant), the OPL Manager copy, libretro (`Named_Boxarts`), ScreenScraper |
  | ICO, COV2, LAB | the OPL Manager copy, ScreenScraper |
  | BG | the OPL Manager copy (`BG_00`, then unused screenshots), ScreenScraper, the libretro screenshot |
  | SCR, SCR2, LGO | libretro (`Named_Snaps`, `Named_Titles`, `Named_Logos`), the OPL Manager copy (`SCR_00`, `SCR_01`, `LGO`), ScreenScraper |

  A source whose images are smaller than the slot goes last (the OPL Manager copy for COV and ICO RiptOPL). Packaging (COV, COV2, LAB, ICO) is taken only by the game's ID or serial number, by the Redump name with its region, or by a ScreenScraper image with its region; an OPL Manager copy image that byte-matches an image of a game from another region is skipped in the silent download with a reason, and is offered last in the game card. SCR, SCR2, BG and LGO with an empty result are taken from other editions of the same game. Coverage against the title database — research/14 §1; without ScreenScraper, packaging is found for Japanese games in a quarter to a third of cases. Importing image sets (ART-5) and manual replacement remain for the rest. The logo is fitted into a frame on a transparent background; other types are stretched as in OPL (ADR-0032).
- When several sources offer a cover, the card shows variants to choose from; by default the first in source order is taken. Bulk download (ART-4) gets a type selection, like OrbitPS2 Manager (research/11 §1).

**Not doing.** Downloading game images into the program is not embedded in any form (§7): the legal analogue of "downloaded and straight to the flash drive with covers" is importing from a DVD drive of one's own disc (proposal IMP-1d in ROADMAP "Later").

### 5.5 Game settings (CFG)
| ID | Requirement | Stage |
|---|---|---|
| CFG-1 | Parser and writer: `key=value`, CRLF, key ≤31, value ≤255. We do not write keys starting with `#`. **Unknown keys and their order are preserved**. A CFG in another format (wOPL since 2026-06-20 writes `CFG/<ID>.cfg` in libconfig syntax plus `.info` files) is recognized and not overwritten | MVP |
| CFG-2 | Editor: compatibility modes (bits 0x01–0x20; 0x40/0x80 only in the RiptOPL profile), GSM, VMC slots, cheats on/off, PADEMU, AltStartup. Only what changed is written and according to OPL menu rules: the default value deletes the key, "global settings" — the absence of the `...Source` key, `$ConfigSource=1`; bits and keys the editor does not know are kept. `$DMA` applies only to HDD and is not edited until HDD-1 (ADR-0036) | v0.2 |
| CFG-3 | Compatibility presets from community lists. The source is shown; we apply only after confirmation | v0.3 |
| CFG-4 | Global settings (`conf_game.cfg`, etc.) | later |
| CFG-5 | CFG backup before every change | MVP |

### 5.6 Virtual memory cards (VMC)
| ID | Requirement | Stage |
|---|---|---|
| VMC-1 | Creating a **properly formatted** 8/16/32/64 MiB card (like OPL's genvmc, byte-exact; not an empty file) and assigning it to a slot. As a single file with preallocation, fragment count checked after writing; an existing card is replaced only on an explicit request and goes to a backup (ADR-0036) | v0.2 |
| VMC-2 | View, import and export saves (`.psu`/`.max`), exchange with a physical card dump | later |

### 5.7 Cheats
| ID | Requirement | Stage |
|---|---|---|
| CHT-1 | `.cht` editor: raw codes, mastercode, no more than 250 cheats | v0.3 |
| CHT-2 | pnach → cht conversion by ELF CRC (experimental) | later |

### 5.8 PS1 and applications
| ID | Requirement | Stage |
|---|---|---|
| PS1-1 | BIN/CUE → VCD (cue2pops logic), `POPS/` layout, launchers `XX.<VCD>.ELF`. The user places the proprietary POPStarter/POPS files themselves; we do not distribute them | v0.3 |
| APP-1 | `APPS/<folder>/title.cfg` (`title=`, `boot=`, `argv1=`), covers by ELF name | v0.3 |

### 5.9 Devices and network
| ID | Requirement | Stage |
|---|---|---|
| DEV-1 | List of removable disks: FS, free space | MVP |
| DEV-2 | "Media diagnostics" read-only: FS, partition table and partition number, sector size (512 bytes only), cluster, alignment, label, junk files; verdict for the selected medium and loader (exFAT for OPL 1.2; FAT32 for 1.1.0 and when OPL boots from this flash drive via FreeMcBoot). Parameters — research/06 §6.1 | MVP |
| DEV-3 | Formatting a removable medium (USB, SD for MX4SIO or the emulator) to the required FS with the partition layout and cluster size for the profile (ADR-0013). Removable disks only, never the system one; we show model, serial number, size and partitions; confirmation by typing; re-check of the disk right before writing; runs in a separate elevated process. **Memory card emulator SD**: formatting destroys card images (including ones with FreeMcBoot), so we first recognize the emulator's service files and require a backup | v0.2 |
| DEV-4 | Media preparation wizard in one plan: formatting (optional) → folder structure (LIB-3) → loader installation (LDR-3) → initial settings. In simple mode this is the first step of the beginner path: two questions (boot method and build), FS and partition layout are chosen automatically by OPL rules (UI-6, §5.11.2) | v0.2 |
| NET-1 | SMB share as a regular folder: mounted by the OS or local on the server | MVP |
| NET-2 | SMB share setup assistant. OPL understands only SMBv1 (dialect `NT LM 0.12`) | later |
| HDD-1 | PS2 internal HDD (APA/PFS, HDL 0x1337 partitions): first via separate helper programs hdl-dump/pfsshell, then natively | later |
| HDD-2 | OPL NBD server | later |

### 5.10 Loaders
A loader is an ELF (for example `OPNPS2LD.ELF`) launched by an exploit on the memory card (FreeMcBoot, PS2BBL). "Install" means putting the ELF where the user's boot method will find it, creating folders and, if needed, initial settings. Details — research/07.

| ID | Requirement | Stage |
|---|---|---|
| LDR-1 | Loader and build catalogue as data (`data/loaders/`, ADR-0022): OPL 1.1.0, OPL 1.2.0 beta (tag `latest`), YAOPL (`belomaxorka/yaopl`, tag `latest`), RiptOPL, wOPL, OPL fork with MMCE (`ps2-mmce`, branch `mmce_support`); later NHDDL/Neutrino. For each build: family, channel, repository, licence, profile (§4), asset names, folder set, path presets for the boot method, fingerprints for detection, supported media (including MMCE) | v0.2 (OPL 1.1.0, 1.2.0 beta, YAOPL), v0.3 (forks, MMCE) |
| LDR-2 | Download only from official releases at the user's request (ADR-0014). Pinned to the asset's SHA-256, not the tag: "floating" tags (`latest`) are overwritten in place. The hash comes from the release's `SHA256SUMS` or the GitHub API `digest`; otherwise we compute our own and show it. The hash shown before confirmation is pinned in the download request (a rebuilt tag is rejected) and in the installation plan; for releases without a `digest`, hashes are pinned in the catalogue (§5.10.1). The downloaded ELF is also checked against the version inside the file (ADR-0043). Cache, licence, changelog. Offline installation from one's own archive. Binaries are not included in the build | v0.2 |
| LDR-3 | Install onto the medium where the standard boot method will find the loader, without editing configs on the memory card: for example, on a flash drive — `mass:/UPDT/OPL.ELF` (OPL menu in FreeMcBoot 1.966) or `mass:/APPS/OPNPS2LD.ELF` (the first path in the standard PS2BBL config). Plus folders by profile and build (LIB-3, `POPS/` and `EMBER/` for RiptOPL), an installation manifest and, where the loader reads settings from the medium, initial settings preserving foreign keys (optional only). On the emulator SD, the card's service files (`.sd2psx/`, `MemoryCards/`, `PS1/`, `PS2/`, `mcp2.bin`, `os/`) are never touched. Everything through a plan (ADR-0044) | v0.2 (official OPL and YAOPL on USB and MX4SIO), v0.3 (forks, MMCE) |
| LDR-4 | Detect the installed loader, its family, channel and version: the version string inside the ELF (ADR-0041), SHA-256 against the frozen releases table and our installation manifest (the manifest counts only if its hash matches the file; otherwise it is stale and the actual files decide, ADR-0040) → `Version=` in RiptOPL's `title.cfg` → file names → settings file fingerprints; channel — by version and catalogue templates (ADR-0042). The update offer is **only within the same family and channel**; changing fork or channel is a separate explicit action (§5.10.1, ADR-0022, ADR-0045) | v0.2 (OPL, YAOPL), v0.3 (forks) |
| LDR-6 | Build selection in the install and replace wizard: a card for each build with a short description of its advantages (from the catalogue, in English) and a "Recommended" mark; details of the version to be installed: version string, release date, size, a short changelog if present in the release, links to the release and commit. A specific version is chosen where the repository keeps releases. The same details in the update window (LDR-4). Details — §5.10.2 | v0.2 (OPL 1.1.0, 1.2.0 beta, YAOPL), v0.3 (RiptOPL and other forks) |
| LDR-7 | Replace one build with another (official OPL ↔ YAOPL ↔ RiptOPL) in one plan: games, covers, CFG, VMC and cheats remain; folders needed by the new build are created. The plan lists in advance what will change for games under the new build's profile: when moving to OPL 1.1.0 — games that will become invisible (ZSO, files not in one piece, an exFAT medium), with an offer to convert ZSO to ISO; when leaving 1.1.0 — JPG and BMP covers with an offer to re-encode them to PNG. Details — §5.10.3 | v0.2 (OPL 1.1.0 ↔ 1.2.0 beta ↔ YAOPL), v0.3 (RiptOPL) |
| LDR-5 | Install and update the loader inside the memory card image on the emulator SD (PS2 memory card filesystem, together with VMC-2) | later |
| BOOT-1 | Boot hint: for the chosen method (FreeMcBoot, PS2BBL, OSDMenu, uLaunchELF) we show the exact ELF path and the lines the user adds to the boot config themselves. YAOPLM does not write loader configs from the memory card: PS2BBL, for example, reads the config from USB before the card, and a foreign file can break boot (ADR-0044) | v0.2 |

Boot method limitations (research/07): FreeMcBoot does not read MMCE, MX4SIO or exFAT. Booting OPL from these media requires PS2BBL (builds with MMCE/MX4SIO), OSDMenu or OPL inside a boot card image. Official OPL stores `conf_opl.cfg` primarily on the memory card, i.e. when booting from the emulator — inside the card image, not as a file on the SD.

Out of scope: installing the exploits themselves (FreeMcBoot, PS2BBL) onto a physical memory card. This is done from the console.

#### 5.10.1 Installing and updating the loader: how it works

Details and sources — research/07 §1.9–§1.10, §2.9, §3, §6; decision — ADR-0022.

**Catalogue (LDR-1).** Files `data/loaders/<id>.toml` in the YAOPLM repository; a copy is included in the build, a fresh version is pulled from `main` at the user's action and cached. A record describes a build, not a loader in general:
- `id`, `family` (`opl`, `riptopl`, `wopl`, `mmce`, `nhddl`), name, repository, licence (SPDX and link to the file), `profile` (§4: for YAOPL — the same as 1.2.0 beta);
- `channel`: how to find the release (fixed tag `v1.1.0`, floating `latest` or `rolling`), which asset is the ELF (`OPNPS2LD.ELF`), which is the variants-and-languages archive, where the hashes come from (`SHA256SUMS`, GitHub `digest`, otherwise our own);
- `layout`: folder set (LIB-3 plus `POPS/`, `EMBER/` for RiptOPL, `nhddl/` and `neutrino/` for NHDDL), ELF path presets by boot method (research/07 §2.9), where settings live and which keys can be seeded;
- `fingerprints`: ELF names (`OPNPS2LD*.ELF`, `RIPTOPL*.ELF`, `WOPNPS2LD.ELF`), settings files (`conf_opl.cfg`, `settings_riptopl.cfg`, `wopl_settings.cfg`, `nhddl/`), the package's `title.cfg`, version templates and the fork suffix by which the version inside the ELF yields the channel;
- `checksums`: how to confirm downloaded files — GitHub `digest`, the release's `SHA256SUMS`, hashes pinned in the catalogue (see below);
- `docs`: links to the official build documentation (upstream README and wiki, the RiptOPL docs site with `install.html` and `ps1-vcd.html`, POPSTARTERINFO for POPStarter). They are shown on the install screen and on the device card; we do not write our own retellings of instructions.

**Build identity (LDR-2).** Build = family + channel + version string. The version comes from the ELF itself (ADR-0041), and for a release — from the asset or archive member name, release name or tag (`OPNPS2LD-v1.2.0-Beta-2271-b3a2199.7z`, `v1.2.0-Beta-2370-33a2b19-yaopl`, `v1.1.0`); the channel — from the catalogue's version templates: `v1.1.0` — OPL 1.1.0, `v1.2.0-Beta-…` without a suffix — upstream beta, `…-yaopl` — YAOPL. SHA-256 is needed for two things: to verify the downloaded file (hash from `SHA256SUMS` or the release `digest` at download time, pinned in the plan; for releases without a `digest` — pinned in the catalogue) and to confirm our installation (manifest, ADR-0040). We do not keep a hash history for new builds (product owner's decision 2026-10-25, ADR-0042): the version inside the ELF identifies any build, while a history would grow every day and would still miss builds replaced within a day. The table `data/loader-hashes/opl-stable.txt` — frozen releases only: OPL 0.5–0.9.3, whose version cannot be read, and 1.0.0, 1.1.0. Everything YAOPLM downloads itself is remembered in a local cache with the version and hash.

**Device screen.** After diagnostics (DEV-2) the device card shows a "Loader" line: `YAOPL v1.2.0-Beta-2335 (d3362de), UPDT/OPL.ELF` or `OPL, version unknown` or `not installed`, and the buttons "Install…", "Update to …" (when the same channel has a newer build), "Replace…" (explicit build or channel change) and "Check for updates". Network only for these actions or for the daily check enabled in settings (ADR-0007). Next to them a "Documentation" link to the official pages of the installed build. For builds with PS1 included, the card shows the component status: "POPStarter and Ember installed", "`POPS/POPS.ELF`, `POPS/IOPRP252.IMG` and BIOS in `EMBER/` are missing" with exact paths and a link to the documentation describing where the user obtains these files themselves.

**Device card states and its buttons.** Every connected medium has a card, not only prepared ones. Install, update and replace buttons exist only where the loader can work at all: on the system disk, a read-only volume and any medium with a "Not suitable for OPL" verdict (DEV-2: NTFS on USB, exFAT for OPL 1.1.0, GPT, sector not 512 bytes) they are absent, and the automatic update window is not shown for them; detection of the installed loader still runs and is shown on the card. States of the other media:
- *foreign files, no OPL layout*: "Install…" — the wizard first offers to format the medium (DEV-3, with all the protections of ADR-0013; before v0.2 the item is unavailable, and the second option remains) or keep the files and add OPL alongside, then choose the build and boot method;
- *empty medium*: "Install…", formatting optional;
- *layout present, loader absent* (for example, a flash drive with games): "Install loader…" — folders and ELF only, games untouched;
- *loader installed and up to date*: "up to date" mark, menu "Reinstall…" (replace the ELF and restore folders, games and settings remain; the "from scratch" option = formatting first via DEV-3), "Replace…", "Check for updates";
- *loader installed, update available*: badge and "Update to …" button;
- *loader of a known family, version unknown*: "Replace…" only.

Each state has a "What next" hint with one main button (UI-6, §5.11.2). A removable medium with a "Not suitable for OPL" verdict that formatting will fix (wrong FS or layout, 512-byte sector) gets a "Prepare medium…" hint — the DEV-4 wizard with mandatory formatting via DEV-3; install, update and replace buttons are still absent for such a medium (ADR-0038).

**Automatic update offer.** When a medium appears in the list (on opening the devices screen, via the "Refresh list" button or via volume polling while the screen is open), YAOPLM detects the loader without network access and compares it with the latest build of the same channel from the catalogue cache. If it is newer, once per "medium (volume serial number), installed build" pair an "Update available" window is shown: the installed and new version with build details (LDR-6, §5.10.2) — release date, size, short changelog, links to the release and "What changed since your version"; "Not now" is remembered in settings, and the button on the card opens the window again. The "Offer loader updates automatically" setting (on by default) disables this window entirely: it does not appear at program start or when a medium appears, while the "Update to …" and "Check for updates" buttons on the card remain. The catalogue itself and release details are refreshed from the network only via "Check for updates" or the enabled daily check (ADR-0007).

**Installation (LDR-3).** Choosing the build and channel from the catalogue (stable OPL 1.1.0 and the beta have different channels; build cards, version details and version selection — §5.10.2), the boot method (the preset gives the ELF path; a custom path can be set) and, optionally, initial settings. Before downloading we show the licence and a link to the changelog. Plan: folders → ELF to a temporary file, `fsync`, rename → backup of the previous ELF to `.yaoplm/backup/loader/` → manifest `.yaoplm/manifest.json` (build, channel, version, hash, path, date, source) → initial settings preserving foreign keys. The memory card is not touched; after installation we show the BOOT-1 lines for the chosen launcher. For RiptOPL the whole `APPS/APP_RIPTOPL/` package is installed (`title.cfg`, icons, `settings_riptopl.cfg` next to the ELF).

Product owner's decisions 2026-10-25 (ADR-0042):
- **Presets:** on a flash drive, FreeMcBoot by default — `UPDT/OPL.ELF`; PS2BBL — `APPS/OPNPS2LD.ELF`; OPL application — `APPS/OPL/OPNPS2LD.ELF` with `title.cfg` (for YAOPL — `APPS/YAOPL/`). "Don't know" installs the ELF into both standard paths.
- **MX4SIO:** the wizard asks where the medium will run (USB or MX4SIO), default by bus. For MX4SIO — the application preset or a custom path and lines for PS2BBL-MX4SIO (`massX:/…`) and OSDMenu (`mx4sio:/…`), or "OPL on the memory card" (folders only).
- **Foreign ELF:** a known OPL is replaced with a backup; a file of another family or an unrecognized one — only with the "Replace this file" mark, a copy is also made. The last three copies per path are kept.
- **Language pack** — optional, only the selected language and its font, flat in `LNG/`; updated together with the ELF.
- **Startup settings** — only in advanced mode: `usb_mode=2` (and `language_text`), other keys and their order are preserved; not written for MX4SIO. `LICENSE` and `CREDITS` are not copied to the medium.

**Detection (LDR-4).** Candidates: preset paths, `APPS/*/title.cfg`, any ELF by fingerprints in the root, `APPS/`, `UPDT/`, `OPL/`. For each we read the version string inside the ELF (ADR-0041) and compute SHA-256; the build is determined in order: version inside the ELF and hash against the frozen releases table, confirmed by the manifest if its hash matches the file → `title.cfg` → file names → settings fingerprints (which give only the family). Channel — by version and catalogue templates (`fingerprints.versions`, `fingerprints.suffix`, ADR-0042). The manifest never decides alone: if its hash matches the file, the build is "exactly" the one recorded in it; if not, the ELF was replaced without YAOPLM, and the build is determined by the file itself (version inside the ELF, `title.cfg`, names), and the card shows "Manifest stale: a different build is on the medium". Result with a confidence level: "exact" (version inside the ELF, frozen release hash or confirmed manifest), "by indirect signs" (names, `title.cfg`), "family only".

**Update.** We compare with the latest known build of the same family and channel; "newer" is determined by the build number from the version or the release date. An update replaces only the ELF (and the application package for RiptOPL) with the same plan as installation: settings, games and other files are untouched, the previous ELF remains in a backup with a "Roll back" button (on the card — "Restore <version>"; rolling back swaps the file and the copy, so a second rollback undoes the first, ADR-0045). We never offer another fork or another channel (stable 1.1.0 is not offered the beta) and never change the ELF path without an explicit choice. An unknown build of a known family gets only the "Replace with …" offer, which the user chooses themselves. How replacement works — §5.10.3 (LDR-7).

**PS1: POPStarter, POPS and Ember.** POPStarter (a PS1 game launcher via the POPS emulator from PSBBN) is distributed without source code or licence; the POPS files (`POPS.ELF`, `IOPRP252.IMG`) are Sony's property; Ember in RiptOPL is under `LICENSE-BETA.txt` and requires the user's BIOS. YAOPLM does not download or bundle them: it only prepares the layout (`POPS/`, VCD, launchers `XX.<VCD>.ELF`, `EMBER/games/`), checks that the user has placed their own copies, and warns if they are missing (PS1-1, v0.3).

#### 5.10.2 Build and version selection (LDR-6)

Product owner's decision 2026-10-24, ADR-0038. The install wizard (LDR-3, DEV-4) and "Replace…" (LDR-7) start with build selection.

**Build cards.** Each catalogue build is a card: name, who publishes it (official OPL or a fork), channel and one or two lines about what it is good for and whom it suits. The text is stored in the catalogue (`summary`), and a catalogue test checks that every build has a description: the catalogue is updated without releasing the program, so the texts live there rather than in translation files. One build is marked "Recommended" (catalogue field `recommended`: OPL 1.2.0 beta, the default profile from §4; product owner's decision 2026-10-25, ADR-0042). Initial content:

| Build | What it is good for | Limitations |
|---|---|---|
| OPL 1.1.0 (official, stable, 2021) | a release proven over the years, referenced by old instructions; PNG, JPG and BMP covers | no exFAT or ZSO; each game file must be one piece (ADR-0037); no fixes after 2021 |
| OPL 1.2.0 beta (official, tag `latest`) | built on every OPL change: exFAT, ZSO, up to 64 pieces per game, fresh compatibility fixes | a beta without a fixed version: the build under the tag is replaced in place |
| YAOPL (the YAOPLM owner's fork) | the same 1.2.0 beta plus fixes for hangs and crashes on unusual files (cheats, ISO, ZSO, `SYSTEM.CNF`, `ul.cfg`, CFG, VMC; list in the fork's `FORK.md`); format rules are the same | not an official OPL build |
| RiptOPL (NathanNeurotic's fork; v0.3) | PS1 games via POPStarter and Ember, games from the memory card emulator SD (MMCE), Neutrino core, its own themes, RetroAchievements in the `rolling-ra` variant | no versions, only variants rebuilt several times a day; settings in its own files |

**Version details.** Under the selected build — what will be installed:
- the version string (`v1.2.0-Beta-2370-33a2b19-yaopl`, `v1.1.0`) from the asset or release name;
- the release date (`published_at`; for floating tags — the asset upload date);
- the download size and how much it will take on the medium;
- a short changelog: the beginning of the release text, no more than 10 items, plain text, links only to github.com; if there is no text or it contains only service lines — "Changelog not published";
- links to the release page and the build commit (hash from the version string); in the update window also "What changed since your version" — a comparison of the installed and new build commits when both hashes are known;
- licence and authors (LDR-2).

The details come from the catalogue cache and the GitHub Releases API: one request with an `ETag` per repository when the wizard opens or on "Check for updates" (ADR-0007). Without network we show what is in the cache with a note "Changelog unavailable without network"; installation from one's own archive (LDR-2) shows the version and file hash.

**Version selection.** By default the latest build of the channel is installed. Another version ("Other version…", advanced mode only) can be chosen where the repository keeps releases. The list is built from the repository's releases and filtered by the catalogue: only tags for which YAOPLM has a profile (§4) get in. What the repositories keep (research/07 §1.11, checked 2026-10-24):
- **Official OPL:** releases `latest`, `v1.1.0`, `v1.0.0`, `v1.0.0-rc1` and releases 0.5–0.9.3. In the selection, `latest` (1.2.0 beta) and `v1.1.0`; older versions match no profile. Earlier 1.2.0 betas are not kept on GitHub (`latest` is overwritten in place), so they cannot be selected. The OPL README points to an archive of all builds on MEGA, but YAOPLM does not download from there (it is not a GitHub release, and a custom MEGA client would be needed, ADR-0014): the user installs such a build from their own archive (LDR-2), and if its hash is in the history (§5.10.1), it is recognized.
- **YAOPL:** currently only `latest` (`FORK.md`: only the `master` tip build is published). Decision: the fork's CI publishes every build also as an immutable release with a version tag (`v1.2.0-Beta-2370-33a2b19-yaopl`) next to `latest` and does not delete such releases. Then version selection and rollback without a local backup also work for YAOPL.
- **RiptOPL:** no versions. The variants `rolling` (inceBtion theme), `rolling-korium` and `rolling-ra` (pre-release) are rebuilt on every change, the `current-fan-favorite` snapshot is updated occasionally, and the `rebuild-*` and `checkpoint/*` tags have no releases. A variant is selected (each is a catalogue channel), not a version.
- **wOPL and the MMCE fork (v0.3):** versioned releases exist (`v1.0`, `v1.1` and `latest`; `mmce-beta1` and `OPL-MMCE-beta-2`), version selection works.

A non-latest version is installed as an ordinary installation of the same channel: the manifest remembers the version, the update offer and "Not now" work as usual (§5.10.1).

#### 5.10.3 Build replacement (LDR-7)

Product owner's decision 2026-10-24, ADR-0038. "Replace…" on the device card is an explicit action: an update never changes the family or channel by itself (ADR-0022).

**What remains.** The game layout is shared across the OPL family (`CD/`, `DVD/`, `ul.cfg` with parts, `ART/`, `CFG/`, `VMC/`, `CHT/`, `APPS/`), so games, covers, CFG, VMC and cheats are untouched. YAOPLM does not migrate or delete the loaders' global settings: official OPL and YAOPL store `conf_opl.cfg` primarily on the memory card, RiptOPL writes its own `settings_riptopl.cfg` and reads `conf_opl.cfg` only for initial values (research/07 §1.1). After returning to official OPL its previous settings are in place.

**Plan.** The same wizard as for installation (build selection and version details — §5.10.2), the current build marked "Installed". Transactions in order (ADR-0021):
1. conversions the user left checked (see below);
2. folders that are missing but needed by the new build (LIB-3, `POPS/` and `EMBER/` for RiptOPL);
3. the ELF or application package of the new build by the boot-method preset; the previous ELF — to `.yaoplm/backup/loader/`. If the new build's path differs, the previous ELF stays in place by default; it can be removed with a checkbox and deletion confirmation;
4. the manifest: the new build, while the previous one is remembered for "Restore previous build".

The loader changes last: if writing is interrupted during the conversions, a working previous build remains on the medium. After replacement we show the BOOT-1 lines if the ELF path changed, and the medium's active profile (§5.1.1) switches according to the new manifest. "Restore previous build" does not roll back the conversions: ISO and PNG work on any build of the family.

**What will change for games.** Before confirmation the plan checks the library against the new build's profile using the same rules as scanning (LIB-4, ADR-0037), and shows a "What will change for games" section: games, reason and what can be done.

| Transition | What will stop working | What the plan offers |
|---|---|---|
| to OPL 1.1.0 from 1.2.0 beta, YAOPL or RiptOPL | exFAT medium: 1.1.0 cannot read it, no games will be visible | the usual replacement is unavailable, the reason is stated; options — keep the build or prepare the medium anew in FAT32 (DEV-4; copy the games to a computer first) |
| | ZSO games: 1.1.0 does not know ZSO | ZSO → ISO conversion in the same plan (GAME-4; on FAT32 a game over 4 GiB goes to `ul`), the checkbox per game is on; a game for which there is not enough space stays in the "will not be visible" list |
| | games whose file is not one piece (`gameFragmentedOpl110`), and games with an unknown number of pieces | only the list and the OPL README advice: copy the games to a computer, format the medium and add them again; YAOPLM does not defragment by itself (ADR-0037) |
| from OPL 1.1.0 to 1.2.0 beta, YAOPL or RiptOPL | JPG and BMP covers: 1.2.0 betas read only PNG (since commit `c5a12a0`, 2024-10-22, research/02) | re-encode to PNG in the same plan as covers, with a backup (ART-2, ADR-0025) |
| from RiptOPL to official OPL or YAOPL | PS1 games from `POPS/` and `EMBER/games/` disappear from the list (the files remain); compatibility bits 0x40/0x80 and the keys `$CoreLoader`, `$Neutrino*`, `$Ember*`, `$VMCDisable_N` are not applied by official OPL; it does not read `ART/art.tar` | list of affected games; CFG are not changed (unknown keys are preserved, CFG-1) |
| to RiptOPL | covers deeper than 8 bits per channel are not loaded by RiptOPL (ART-6) | re-encoding such covers (together with ART-6) |
| between OPL 1.2.0 beta and YAOPL | nothing: the profile is the same | "Nothing changes for games" |

Stages: v0.2 — transitions between OPL 1.1.0, 1.2.0 beta and YAOPL; v0.3 — RiptOPL together with fork installation. wOPL stores CFG in libconfig format (CFG-1), so replacing it and back is a separate decision when it enters the installation catalogue.

### 5.11 Interfaces
| ID | Requirement | Stage |
|---|---|---|
| UI-1 | Desktop: library, game card, devices, jobs, settings | MVP |
| UI-2 | English-only interface from the first launch | MVP |
| UI-3 | Light and dark theme, full keyboard navigation | MVP |
| UI-4 | Simple mode, on by default (product owner's decision 2026-10-24): hides sorting, counters, conversion, set import, the jobs page and technical details; consistent across all screens, everything hidden is available in advanced mode. Plan: `docs/plans/m5-ui-simple-mode.md` | v0.2 |
| UI-5 | Interface without overlaps or horizontal scrolling at any window size from 800×600 to 2560×1440; an automated test walks all screens and buttons | v0.2 |
| UI-6 | Beginner path in simple mode (product owner's decision 2026-10-24): the program guides from an inserted flash drive to a game on the console. The device card says what state the medium is in and suggests the next step: prepare the medium (formatting by OPL rules with DEV-3 protections, folders, OPL installation), add games, download images, verify on the console. The "What next" hint — on the device card, in the empty library and at the end of every completed step; everything according to the official OPL documentation (§5.11.2) | v0.2 |
| CLI-1 | `yaoplm`: `scan`, `info`, `add`, `rename`, `art fetch`, `cfg` (show and edit: `--mode`, `--vmc`, `--set`, `--unset`), `vmc ls`/`vmc create`, `rpc` | MVP (part), v0.2 (`cfg`, `vmc`) |
| SRV-1 | `yaoplm-server`: the same interface in the browser. Token login, listens on 127.0.0.1 by default, access only to allowed roots, Docker image | v0.2 |

#### 5.11.1 Simple mode (UI-4): what is visible

Product owner's decision 2026-10-24, ADR-0034. Advanced mode shows everything; simple mode shows what is below. An element is hidden on all screens or none: a property visible on the game card can be filtered in the library, and an action hidden in the library is hidden in the card too.

| Screen | Visible in simple mode | Hidden in simple mode |
|---|---|---|
| Shell | menu: library, media, settings; path of the open medium; theme; floating write panel with "Cancel"; write result notification | "Jobs" item; "All jobs" link |
| Core connection (both modes) | in the browser — only when the core does not respond: the banner "No connection to YAOPLM at <address> — actions are unavailable until the connection returns", nothing behind it is clickable | permanent "Connected" indicator; the desktop has no indicator at all (the core is in the same process) |
| Library | search; "Disc type" (CD/DVD) and "Format" (ISO/ZSO/UL) filters; "Rescan"; "Download images" (one click: all types, missing only, then plan); "Add games"; number of games and size; "N files OPL cannot see"; incomplete operation banner; "Create folders"; warnings; cards with a warning and a dump badge | "With warnings: N"; "Without cover: N"; sorting and its direction; grid or list (always grid); "More" menu: "Import images", "Verify dumps", "Convert…"; medium path under the panel |
| Add games | everything except the medium path; the add toggles — only here | medium path next to "Library" (it is in the header) |
| Game card | title, badges (ID, disc type, format, size, DVD-9); "Rename" (title only); "Move to CD/DVD"; "Delete"; warnings; ID, database title, disc type, format, dump verdict, size; "Download all images" and image slots; game settings: compatibility modes 1–6 and slot 1 memory card (choose from `VMC/` or "Create" with a default 8 MiB name), "Save…" (product owner's decision 2026-10-24, ADR-0036) | "Convert…"; "Verify dump"; in the rename window — ID correction and file name style; path, ID in the image, name format, layers, number of pieces on the medium (IMP-7b); ul parts; in game settings — slot 2, name, size and replacement of a new card, GSM, cheats, PADEMU, alternate boot, CFG path and keys; number of cheats |
| Media | "Refresh", "Open folder…", "Show all volumes"; path, label, FS, free space, verdict, "Open library"; warnings and diagnostic errors | bus, type, model, serial number, cluster, partition; informational tips |
| Jobs | floating panel: progress of each write and search with "Cancel"; write result — as a notification | page; library scanning is not shown in advanced mode either |
| Settings | "Simple mode", theme; "What next" hints (UI-6); file names; Redump DAT (compact); keyboard; about | "Verify IDs in images when scanning"; image sources, mirrors, cache |
| Logs | under the add plan and in the "Download images" window — search progress and result | full job logs (image search, dump verification, set import, image variants); in advanced mode the log is collapsed by default everywhere |
| Plan window | result (what will be written and deleted, space before and after), warnings, game operations, delete confirmation checkbox | steps of each operation (folders, backup, renames) and service operations without a game; in advanced mode steps are collapsed by default (product owner's decision 2026-10-24) |
| "What next" hints (UI-6) | a hint with an explanation and one main button on the device card, in the empty library and at the end of a write; "Got it" | — (in advanced mode the hint collapses to one line with a button) |
| Loader wizard (LDR-6, LDR-7) | build cards with a description and "Recommended"; version, date, size, changelog, links; for replacement — the "What will change for games" section | "Other version…"; custom ELF path; startup settings; SHA-256 |

#### 5.11.2 Beginner path (UI-6)

Product owner's decision 2026-10-24, ADR-0038. Simple mode is on by default and has priority: its job is to take someone preparing a medium for PS2 for the first time from an inserted flash drive to a game on the console without reading forums. At each step the program answers "what next" with one hint and one main button. Everything the hints say about OPL and the console comes from the official OPL documentation and the selected build.

**Steps.**
1. *Medium.* While there are no media, the "Media" screen says: "Insert a flash drive or SD card — it will appear here" (in the browser — "Open the library folder"). An inserted medium appears by itself (volume polling, §5.10.1), and its card shows the state in plain words and a hint (table below).
2. *Preparation* (DEV-4). The wizard asks two questions: "How do you boot OPL?" (FreeMcBoot from the memory card, PS2BBL, OPL already on the memory card, "Don't know") and "Which OPL to install?" (build cards, the recommended one selected, §5.10.2; if OPL is on the memory card — "Which OPL version is on the card?" with the answer "Don't know"). The wizard chooses the FS and layout itself by the OPL README and DEV-2 rules (research/06): MBR, 512-byte sector; exFAT with the default cluster size if both the selected build (1.2.0 betas) and the boot method can read it; otherwise FAT32 (OPL 1.1.0; booting OPL from this flash drive via FreeMcBoot, which cannot read exFAT, research/07). "Don't know" → FAT32: all OPL builds and all boot methods read it, and games over 4 GiB automatically go to `ul` (IMP-5). If the FS is already suitable and the user wants to keep the files, formatting is skipped. Then the usual plan: formatting with all DEV-3 protections (model, serial number, size, confirmation by typing the size, re-check of the disk), folders (LIB-3), loader (LDR-3). In server mode and in Docker there is no formatting (ADR-0013): there the wizard only creates folders and installs the loader, and the hint says the medium can be formatted in the desktop program.
3. *Games.* Preparation result: "Medium ready. What next: add games" with an "Add games" button. The empty library shows the same in the centre: "No games yet. Press "Add games" or drag your disc images here: ISO, ZSO, BIN/CUE, CHD, zip, 7z. Titles and covers will be fetched automatically" (META-1 and ART-8 are on by default). The program does not download game images or suggest where to get them (§7).
4. *Images.* Covers are searched when adding (ART-8). If some are not found, the write result says "N games have no covers" with a "Download images" button (ART-4); otherwise the step is skipped.
5. *Verification on the console.* After the first successful write of games and on the card of a ready medium — "How to run on PS2": safely eject the medium; the BOOT-1 lines for the chosen boot method (menu item or ELF path); "In OPL settings enable the device (USB or MX4SIO): OPL shows folders and games only on an enabled device" (OPL README, "How to use" section); if games are not visible — library warnings (no `BOOT2`, file not in one piece, exFAT on 1.1.0, JPG on 1.2.0 beta) and links to the official build documentation (the catalogue's `docs`).

**Device card hint.** The first matching state is taken:

| State | Hint | Main button |
|---|---|---|
| incomplete operation (log, ADR-0021) | the write was interrupted and needs to be completed or rolled back | "Restore…" |
| not suitable for OPL and cannot be fixed (system disk, read-only, sector not 512 bytes) | why this medium will not work and which one will | — |
| not suitable, formatting will fix it (removable: wrong FS or layout) | "OPL will not read this medium: <reason>. It can be prepared anew; all files on it will be deleted" | "Prepare medium…" (formatting mandatory) |
| empty; foreign files without an OPL layout | "There is no OPL here. Prepare the medium: folders and loader" | "Prepare medium…" |
| layout present, loader absent | "OPL folders are present, no loader. Install OPL or mark that it is on your memory card" | "Install OPL…"; second button — "OPL on the memory card" |
| loader present (or marked on the memory card), no games | "Medium ready. Add games" | "Add games" |
| loader update available | new version and date (LDR-6) | "Update to …" |
| some games have no covers | "N games have no covers" | "Download images" |
| games with warnings | "OPL may not show or run N games" | "Open library" |
| everything ready | "You can verify on the console" | "How to run on PS2" |

The "OPL on the memory card" mark is remembered for the medium by volume serial number and cleared in the card menu.

**Hint rules.**
- The core computes the step from what it already knows (`InspectDevice`: verdict, detected loader, layout; scanning: number of games, missing covers, warnings; logs), without network and without new device calls. The step is carried as an enum value, not text; the web interface and `yaoplm device` display it in their own words.
- A hint is one or two sentences about the next action in YAOPLM and one main button. Facts about OPL and the console come only from the official documentation of the installed build (README and wiki, the catalogue's `docs` links) and our research with a source link. We do not retell instructions in full (§5.10.1) but link to them via "More".
- Where: the device card, the library (empty or on an unprepared medium), the write result notification (preparation, adding games, images, conversion, loader replacement): "Done. What next: …" with a button.
- "Got it" hides this step's hint for this medium. The "What next" hints toggle in settings (on by default) turns them all off; "Show hints again" restores the closed ones. In advanced mode the hint is one line with a button, without explanation.
- Texts go through i18n (English) per the glossary in `docs/I18N.md`.

## 6. Non-functional requirements

- **Data safety.**
  - Every write goes through "plan → confirmation → execution".
  - Writing is atomic: to a temporary file, then rename.
  - Multi-step operations (for example, `ul` splitting) keep a log so they can be rolled back or completed.
  - We touch nothing outside the selected root.
- **Performance.** Hashing in a single pass over the file in several threads. Copying close to the medium's speed limit. The interface stays responsive with 3000+ games.
- **Platforms.**
  - Windows 10+ x64.
  - macOS universal (ad-hoc signing first).
  - Linux x64 and arm64 (baseline glibc — Ubuntu 22.04), Steam Deck.
  - Docker amd64 and arm64.
- **Privacy.** No telemetry. We go online only to sources enabled by the user, with a clear User-Agent.
- **Licence.** GPL-3.0-or-later (ADR-0002).

## 7. Data and legal restrictions

- We do not distribute or download commercial games, BIOS or POPS files.
- **We do not use OPL Manager's private cover server.** Backups of its database that the author published on archive.org are not a server: YAOPLM reads the latest one per image via Range requests (ADR-0035); the user can import any copy themselves (ART-5).
- Covers are not part of the build: we download on request and cache locally.
- Built-in databases — only with compatible licences and attribution. Source comparison — research/03 §6.
- Before the public release we need to:
  - ask Redump about the permitted DAT download frequency;
  - obtain a developer key from ScreenScraper (the source code is ready, ADR-0035);
  - ask the maintainers of compatibility lists to state a licence.

## 8. Out of scope for v1

OPL theme simulator and editor, mobile versions, our own cloud cover service, writing to a physical memory card (including FreeMcBoot and PS2BBL installation). Formatting of removable media moved to DEV-3 (ADR-0013).

## 9. Open questions

1. ~~RiptOPL: is 8-bit PNG mandatory?~~ Closed. The author writes "8-bit PNG required" (psx-place, 2026-09-25), while the RiptOPL code (`src/textures.c`) rejects only PNG deeper than 8 bits per channel; RGB, RGBA and palette load. The "indexed PNG" requirement came from OrbitPS2; for RiptOPL it is a recommendation (research/07).
2. hdl-dump and pfsshell: GPL-2-only or GPL-2-or-later? This determines whether we can port their code into ours (under GPL-3) or only call them as separate programs.
3. Compare `ul` splitting with a reference (OPL Manager, USBUtil) on a real game.
4. Metadata: which fields to show on the game card and write to CFG by default.
5. The user's memory card emulator: model, firmware, whether it supports MMCE (this determines LDR-3 for the emulator SD).
6. Formatting. research/06 recommends our own Rust formatter (MBR + FAT32 + exFAT following the FatFs `f_mkfs` logic, checked in CI), because system utilities do not provide the needed parameters, and Windows did not format FAT32 above 32 GB until spring 2026. Open: whether exFAT patents interfere with our own exFAT formatter under GPL-3 on Windows and macOS; the fallback is the system exFAT formatter on those OSes.
7. ~~Which build does the install wizard mark "Recommended" for a beginner (LDR-6)?~~ Closed 2026-10-25: official OPL 1.2.0 beta (ADR-0042); changed by one catalogue field `recommended`.
