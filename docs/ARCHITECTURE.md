# YAOPLM — architecture

> Version 0.1, 2026-10-22. Rationale for the stack choice — [research/04-stack.md](research/04-stack.md) and [DECISIONS.md](DECISIONS.md).

## 1. Overall scheme

```
                ┌───────────────────────── web/ (React + TS SPA) ─────────────────────────┐
                │  features: library · game · devices · jobs · settings                    │
                │  src/api/transport.ts:  TauriTransport | HttpTransport | MockTransport   │
                └──────────────┬──────────────────────────────┬────────────────────────────┘
                     invoke + Channel                 HTTP /api/rpc + WS /api/events
                ┌──────────────▼─────────────┐   ┌────────────▼─────────────┐   ┌───────────────┐
                │ apps/desktop (Tauri 2)     │   │ apps/server (axum)       │   │ apps/cli      │
                │ thin adapter               │   │ thin adapter + SPA       │   │ (clap)        │
                └──────────────┬─────────────┘   └────────────┬─────────────┘   └──────┬────────┘
                               └──────────────┬───────────────┴────────────────────────┘
                                   crates/yaoplm-api  (Request / Response / JobEvent, dispatch, JobManager)
                                              │
     ┌──────────────┬───────────────┬─────────┴─────┬────────────────┬──────────────┬──────────────┐
  yaoplm-core   yaoplm-formats   yaoplm-hash    yaoplm-device     yaoplm-net     yaoplm-hdd (later)
  model,        ISO9660, CNF,    single pass:   disks, FS,        cover and      APA/PFS
  scanner,      ul.cfg, CFG,     CRC32/SHA1/    copying,          metadata
  jobs,         ZSO, CUE, VMC,   MD5, DAT       splitting,        sources,
  SQLite cache  ART, CHT, APPS   matcher        fragmentation     resize, SMB
```

**Main rule:** the core crates (`yaoplm-core`, `-formats`, `-hash`, `-device`) **do not depend on tauri, axum or tokio**. CI checks this with `cargo tree -i tauri -p yaoplm-core` (and analogous commands for axum and tokio): we expect an empty result. The core is synchronous: `std::fs` plus threads (rayon or std::thread). Asynchrony exists only at the edges (`yaoplm-net`, `apps/server`, `apps/desktop`).

## 2. Stack

Versions checked 2026-10-22 (research/04). Before adding a dependency, verify the current version and licence.

| Area | Choice | Version | Comment |
|---|---|---|---|
| Core language | Rust stable (MSVC on Windows) | ≥1.90, pinned in `rust-toolchain.toml` | |
| Desktop | Tauri | 2.12.x | Tauri 3 is still in alpha, not used |
| Server | axum | 0.8.x | tokio only here and in desktop/net |
| CLI | clap (derive), indicatif | current | |
| Types for TS | ts-rs | 12 | **not** tauri-specta (still RC) |
| Frontend | React 19 + TypeScript + Vite | current | package manager pnpm via corepack |
| Data in the frontend | TanStack Query, TanStack Virtual 3.14 | | list virtualization is mandatory |
| UI components | shadcn/ui + Tailwind CSS | | no heavy CSS effects (WebKitGTK) |
| Routing | React Router (library mode) | 7.x | |
| i18n | react-i18next | | English (`en`) |
| ISO9660 | our own code (ADR-0016) | — | the reader mirrors OPL's isofs; the fixture generator is in `yaoplm-test-support` |
| CHD | chd | 0.4 | pure Rust, behind the `chd` feature |
| Archives | zip 9, sevenz-rust2 0.24 | | RAR — `unrar` behind the `rar` feature (licence!) |
| Hashes | crc32fast 1.5, sha1 0.11, md-5 0.11 | | one read pass, parallel hashers |
| ZSO | lz4_flex 0.14 | | we write the format ourselves (24-byte header); there is no LZ4 HC mode, so files are slightly larger than with `ziso.py -c 9` |
| Images | image 0.25, fast_image_resize 6 | | PNG, later 8-bit palette for RiptOPL |
| XML / YAML | quick-xml 0.42; serde_norway or serde_yaml_ng | | `serde_yaml` is archived, `serde_yml` is unsafe — not used |
| HTTP | reqwest 0.13 (rustls) | | retries, rate limits, ETag |
| SQLite | rusqlite (bundled) | | cache only; the source of truth is the files on the device |
| Disks | windows-sys 0.61, rustix 1.1 | | volumes, FS, cluster, serial number, bus, preallocation; directly through OS APIs (ADR-0019) |
| SMB | smb 0.12 | | a young crate, behind the `smb` feature, later |
| UI tests | Playwright, vitest | | e2e against `yaoplm-server` |

Cargo features: `chd`, `rar`, `smb`, `hdd`. The default build stays lightweight and free of problematic licences.

## 3. Repository structure

```
yaoplm/
├─ Cargo.toml                 # [workspace]: crates/*, apps/cli, apps/server, apps/desktop/src-tauri
├─ rust-toolchain.toml
├─ package.json  pnpm-workspace.yaml
├─ crates/
│  ├─ yaoplm-core/            # Game, Library, Device, Profile; scanner; JobManager primitives; SQLite cache; ProgressSink
│  ├─ yaoplm-formats/         # iso9660 + system_cnf, ul_cfg + usba_crc32, cfg, zso, cue/bin, chd, archives, art naming, vmc, cht, apps
│  ├─ yaoplm-hash/            # multi-hash pipeline, Logiqx/Redump DAT, matching
│  ├─ yaoplm-i18n/            # Phrase: catalogue key + parameters, English text from web/src/locales/en (ADR-0029)
│  ├─ yaoplm-device/          # volumes, FS type, copying/splitting/preallocation, fragmentation (per OS)
│  ├─ yaoplm-net/             # ArtSource/MetadataSource, cache, rate limits, resize
│  ├─ yaoplm-hdd/             # later: APA/PFS
│  ├─ yaoplm-api/             # Request/Response/JobEvent (serde + ts-rs), dispatch(), JobManager
│  └─ yaoplm-test-support/    # fixtures: tiny ISO with SYSTEM.CNF, BIN/CUE, ZSO, DAT, FAT32 image, "virtual" large file
├─ apps/
│  ├─ cli/                    # `yaoplm` binary
│  ├─ server/                 # `yaoplm-server` binary (/api/rpc, /api/events, /media/*, SPA via rust-embed, /health)
│  └─ desktop/src-tauri/      # Tauri 2: rpc + subscribe commands, protocol for covers, dialog/updater/single-instance plugins
├─ web/                       # SPA for desktop and server
│  ├─ src/api/                # transport.ts, generated/ (ts-rs output, do not edit by hand)
│  ├─ src/features/           # library, game, devices, jobs, settings
│  ├─ src/locales/{en}/
│  └─ tests/                  # vitest; e2e/ (Playwright against server)
├─ data/                      # built-in databases (generated by script, with ATTRIBUTION.md)
├─ packaging/                 # docker/, flatpak/, sidecars/ (later)
├─ fuzz/                      # cargo-fuzz: ISO/CFG/ul.cfg/ZSO/CUE parsers
├─ docs/
└─ .github/workflows/         # ci.yml, release-desktop.yml, release-cli-server.yml
```

## 4. API layer

- Each operation is a `Request` variant in `yaoplm-api`. For example: `ListDevices`, `InspectDevice`, `ScanLibrary`, `ListDir`, `PlanImport`, `ExecutePlan`, `RenameGame`, `FetchArt`, `FindArt`, `ImportArt`, `SetArt`, `DeleteArt`, `Attribution`, `ReadCfg`, `WriteCfg`, `CancelJob`.
- `FetchArt` — a job that ends with a plan (`JobResult::Plan`): the cover search runs for minutes and keeps a per-game log (`JobLogEntry::Art`), while writing to the medium remains a plan with confirmation (ADR-0025). The image types are set by `kinds` (ADR-0032). With `import: <import plan id>` the job ends with a set of covers (`JobResult::Covers`), which `PlanImport.covers` embeds into the add plan (ADR-0031). `FindArt` — a job that returns variants of one image to choose from (`JobResult::ArtVariants`); `ImportArt` reads the user's image sets and ends with a plan (ADR-0033). The network is used only inside `FetchArt` and `UpdateRedump` jobs: `yaoplm-api` holds the tokio runtime and calls `yaoplm-net` from the job thread.
- `VerifyGames` — a read-only job: image hashes, Redump DAT verdicts, one log line per game (`JobLogEntry::Verify`), result `VerificationReport`. `RedumpStatus` and `UpdateRedump` (download at most once a week or import a file) manage the DAT (ADR-0026). `ConvertGames` returns an ISO ↔ ZSO ↔ ul conversion plan (ADR-0027); `PlanImport` with `compress` writes images directly as ZSO, and zip and 7z sources are read through `ArchiveSources` (ADR-0028). `PlanImport` with `verify` hashes images in the same pass as the write; `ExecutePlan` puts the hashes into the cache and returns verdicts in `ExecutionReport.checks` (ADR-0030).
- The core returns user-facing texts as phrases (ADR-0029): `Phrase { key, params }` next to the English text — for plan, transaction and job titles (`titlePhrase`, `itemPhrase`), for notifications, warnings, tips and skip reasons (`phrase`), for log lines (`detailPhrase`, `errorPhrase`) and for errors (`ApiError.phrase`). The English text is rendered from the interface's `en` catalogue (`yaoplm-i18n`); the client renders it.
- Loaders (LDR-1, LDR-2, ADR-0042, ADR-0043): `LoaderCatalog` returns the active catalogue (from `main` if this program version read it, otherwise the bundled one), `LoaderFiles` — the saved ELFs. Jobs `LoaderRelease` (what will be downloaded, `JobResult::LoaderOffer`), `DownloadLoader` (holds the SHA-256 the user saw, `JobResult::LoaderFile`), `ImportLoader` (own file without network) and `UpdateLoaderCatalog` (`JobResult::LoaderCatalog`); the network goes through `yaoplm_net::loaders`, and each request may have a `mirror`. Installation (LDR-3, BOOT-1, ADR-0044): `PlanInstallLoader` — a plan from the saved ELF (folders → ELF at each path with a backup of the previous one → language pack → manifest `.yaoplm/manifest.json` → startup settings), `LoaderBootHints` — lines for launchers, `LoaderLanguages` — pack languages; the core is `yaoplm_core::loaders::{install, boot, languages, manifest}`. Update and rollback (LDR-4, ADR-0045): `DeviceInfo.installed` — installed builds with the latest known channel version from saved GitHub responses (without network), `CachedLoaderRelease` — release details from the same cache, `PlanUpdateLoader` — an installation plan with `update` (ELF, language pack and manifest only), `PlanRollbackLoader` — swapping the ELF and the latest backup; the core is `loaders::{installed, rollback}`.
- Game settings and memory cards (CFG-2, VMC-1, ADR-0036): `ReadCfg` returns the game's settings and all CFG keys, `WriteCfg` builds an edit plan (settings by OPL menu rules, keys as is, new cards in the same transaction), `ListVmc` lists `VMC/` with the OPL verdict, save count and games, `CreateVmc` plans empty cards. The plan's transaction carries a list of "before → after" keys (`cfgChanges`); the execution result — post-write notifications (`ExecutionReport.notices`, for example a card in several pieces).
- `dispatch(Request) -> Result<Response, ApiError>`. Long operations immediately return a `JobId`, and progress arrives as a `JobEvent` stream. Plans also keep a step log (`JobEvent::Log`: a transaction or step started, completed, rolled back, failed); `GetJob` returns it in full, so a reconnecting client loses nothing.
- `ListDir` shows folders for choosing sources in the browser: the server exposes only its roots and what is inside them (`PathPolicy`), the desktop — volumes; hidden files and symlinks are not shown.
- Transports:
  - **desktop**: Tauri command `rpc(req)` and `subscribe(channel)` via `tauri::ipc::Channel`;
  - **server**: `POST /api/rpc` and WebSocket `/api/events`;
  - **CLI**: `yaoplm rpc '<json>'` plus convenient subcommands on top of the same `dispatch`.
- The frontend selects the transport via `isTauri()`. Each function is written once.
- The interface-to-core connection can be lost only in the browser: in the desktop the core is in the same process. Therefore the connection state is shown only for the HTTP transport, and only when there is no connection (UI-4 plan, `docs/plans/m5-ui-simple-mode.md`, task 8). Sharing a folder for PS2 over SMB is the job of the OS or NAS, not of this connection.
- TS types are generated by `ts-rs` into `web/src/api/generated/`. CI checks that the generated output matches what is committed.

## 5. Job model and write safety

1. **Plan.** The core builds a `Plan` — a list of steps (copy, split, rename, write CFG, format a memory card, download a cover, delete `games.bin`…), with the data volume and warnings. Nothing is written to disk.
2. **Confirm.** The interface shows the plan. Deletions and overwrites are highlighted.
3. **Execute.** `JobManager` executes the plan with a cancellation token and progress events.
   - Large files are written to `*.yaoplm-part`, then renamed.
   - Small files (CFG, `ul.cfg`): temporary file, `fsync`, rename, plus a backup.
   - Each plan transaction (one game, one rename) keeps a journal in `.yaoplm/journal/` on the device: before the commit point a failure is rolled back, after it it is completed (ADR-0021). CFG backups — in `.yaoplm/backup/`.
   - Write jobs run one at a time, in queue order: parallel writing to one medium fragments games.
4. All paths are normalized and checked to be inside the device root. No `..` or symlinks pointing outside.
5. User files are deleted only on explicit confirmation. In the CLI this requires the `--yes` flag.

## 6. State storage

- The application data directory (`dirs`, overridden by `YAOPLM_DATA_DIR` and the server's `--data-dir`): `cache.sqlite`, `art-cache/<source>/<key>/` (the original and derived PNGs; `art-cache/local/` — images from the user's files and sets; `art-cache/oplm-index/<item>/` — the central catalogue of the OPL Manager copy, ADR-0035), `db/redump-ps2.dat` with metadata `db/redump-ps2.json` and the last request mark `db/redump-ps2.asked` (from M5, ADR-0026), `loaders/` (loader ELFs `elf/<sha256>.elf` with a description, language packs `langs/<sha256>`, GitHub release responses with `ETag`, the catalogue from `main`; from M5, ADR-0043, ADR-0044), later `settings.toml`. Created on the first write to the cache: scanning only reads it. Desktop and CLI: `%LOCALAPPDATA%\yaoplm`, `~/.local/share/yaoplm`, `~/Library/Application Support/yaoplm`.
- SQLite holds:
  - the cover manifest: source, URL, ETag, `Last-Modified`, date, SHA-256, licence, attribution (from M4);
  - "not found" results with a 7-day TTL (from M4);
  - the image hash cache (CRC-32, SHA-1, MD5) keyed by "volume + path + size + mtime" of each file (from M5, ADR-0026); clearing the cover cache does not touch it;
  - raw CD data track hashes keyed by the pair "SHA-1 and image size" (the `tracks` table, from M5, ADR-0030);
  - later: library indexes.
- The built-in title database (META-1) is not in the data directory but embedded in the binary: `data/titles.json.gz` (ADR-0024).
- The source of truth is the files on the device. The cache can be deleted without loss.
- Interface settings (profiles selected for media, name style, open root, library view, image verification) are stored from M3 in the browser's or WebView's `localStorage` (`yaoplm.settings`). `settings.toml` will appear when the CLI and server need settings.

## 7. Data sources

- `yaoplm-net`: one HTTP client (User-Agent `YAOPLM/<version> (+repo)`, no more than one request per second per source, conditional and Range requests, back-off on 429/5xx, `YAOPLM_OFFLINE=1` forbids everything except this computer), a cache with a manifest and image sources `SourceId::{Xlenore, Oplm, Libretro, ScreenScraper}` (ADR-0035); each has a licence tag, attribution and a mirror mode (a folder with the repository layout, which is also the mock for tests), except ScreenScraper. The source order is per type (`sources::order`), the region policy — `region`. Metadata comes from the built-in database; `DatSource` appeared with META-2.
- Order for MVP:
  1. ID from the image;
  2. built-in title database;
  3. images by type (ADR-0035): xlenore (raw URL by serial number), the OPL Manager copy on archive.org (zip table of contents once, then a Range request per image), libretro-thumbnails (`thumbnails.libretro.com`, then raw GitHub, by the Redump name from the built-in database), ScreenScraper with a developer key; no GitHub API;
  4. Redump DAT (v0.2).
- Prohibited: the private OPL Manager server, scraping sites without an API, an image bundle.
- Loaders (LDR-1, LDR-2) are also a data source: the build catalogue `data/loaders/` (an index and a file per build: family, channel, repository, licence, profile, assets, hash sources for verifying downloads, folders, path presets, fingerprints with version templates, a description). The catalogue is read by the core (`yaoplm_core::loaders`: bundled copy, strict schema-based parsing). There is no hash history for new builds: the build on the medium is identified by the version inside the ELF (ADR-0041), while the table `data/loader-hashes/opl-stable.txt` — only for frozen releases. Download only from official releases on user action, hash verification, cache (ADR-0014, ADR-0022, ADR-0042).

## 8. Device and writing (yaoplm-device)

- Volume detection via OS APIs (ADR-0019): FS, free space, cluster, sector, serial number, bus, partition.
- Copying: sequential writing in large blocks, preallocation (per OS: `fallocate`/`F_PREALLOCATE`/`SetFileInformationByHandle`), speed calculation.
- `ul` splitting: exactly 1 GiB (0x80000 sectors of 2048 bytes) per chunk, except the last.
- Fragmentation (v0.2): Windows `FSCTL_GET_RETRIEVAL_POINTERS` (`DeviceFs::fragments`, from M5 checks new VMCs after writing), Linux `FIEMAP`, macOS `F_LOG2PHYS_EXT` (together with the library check, IMP-7b).
- Formatting removable media (DEV-3, v0.2) and raw disk access (HDD, later) — only through a separate `yaoplm helper --stdio` process with elevated privileges (ADR-0013). The main process builds the plan; the helper executes it after re-checking the disk.
- Media profiles: USB, MX4SIO, memory card emulator SD (MMCE), iLink, HDD in exFAT. The profile sets the FS, layout, cluster and where to install the loader (SPEC §4, LDR-3).

## 9. Server mode

- Listens on `127.0.0.1` by default. For LAN you must explicitly specify `--bind 0.0.0.0` and set a token.
- Access only to roots from the allow-list (`--root /mnt/ps2`); mirrors of cover sources must also be inside the roots.
- `GET /media/art?path=` serves covers for `<img>`: only `ART/<ID>_<SUFFIX>.png|jpg|bmp` inside the roots, with `ETag`; the token is accepted in `?token=`, as for `/api/events` (ADR-0012).
- Docker: multi-stage build (web → musl binary → distroless/static), images for amd64 and arm64 in GHCR.

## 10. Testing, CI, packaging

Testing is described in [TESTING.md](TESTING.md). Briefly about CI:
- `ci.yml`, on every PR:
  - Rust: `fmt` and `clippy -D warnings`;
  - tests on Windows, Linux and macOS;
  - web: lint, typecheck, vitest;
  - Playwright against `yaoplm-server`;
  - core dependency checks;
  - check that generated TS types are up to date.
- `release-desktop.yml` builds with `tauri-action@v1` for Windows (NSIS/MSI), macOS universal (dmg, ad-hoc signing) and Linux (AppImage, deb, rpm) only on a `v*` tag (private repository minutes, ADR-0011) and publishes a pre-release draft. The updater is not connected: there is no signing key (CLAUDE.md, rule 5).
- `release-cli-server.yml` builds the binaries and the Docker image.
- Signing: SignPath Foundation for Windows (after the first release), Apple Developer ($99 a year) — when users appear. The updater key is stored in GitHub Secrets and **nowhere else**.
