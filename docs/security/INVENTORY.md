# oplmdash Phase 0 inventory

Target: `0b545c4ff78e57342315ad65691e452c75192ef3` + uncommitted Phase 0 work.
Branch: `security/audit-2026-10`.
Generated: 2026-10-25.

## 1. HTTP routes

From `apps/server/src/lib.rs:78-91` and `apps/server/src/upload.rs:83-90`.

| Method | Path | Auth | Body limit | Handler | Writes |
|---|---|---|---|---|---|
| GET | `/health` | none | n/a | `lib.rs:93` | no |
| POST | `/api/rpc` | token + origin | axum default (2 MiB) | `rpc.rs:10` | depends on `Request` |
| GET | `/api/events` | token (query allowed) + origin | n/a | `events.rs:16` | no |
| GET | `/media/art` | token (query allowed) + origin | n/a (query `path`) | `media.rs:63` | no |
| POST | `/api/uploads` | token + origin | axum default | `upload.rs:92` | creates staging dir + empty file |
| GET | `/api/uploads/{id}` | token + origin | n/a | `upload.rs:141` | no |
| DELETE | `/api/uploads/{id}` | token + origin | n/a | `upload.rs:256` | deletes staging dir |
| POST | `/api/uploads/{id}/chunk` | token + origin | 16 MiB (`MAX_CHUNK`) | `upload.rs:149` | appends to staging file |
| POST | `/api/uploads/{id}/finish` | token + origin | n/a | `upload.rs:217` | fsync |
| GET | everything else | none (SPA) | n/a | `assets.rs:12` | no |

Auth middleware: `apps/server/src/auth.rs:15`. It checks the loopback `Host` only when
`loopback_only && token.is_none()`, checks `Origin` == `Host`, and requires a Bearer token or
(for WS/media only) `?token=`.

## 2. `Request` variants

From `crates/yaoplm-api/src/protocol.rs:107-568`. Classification for server mode follows the
v0.1 scope in `docs/oplmdash/INVESTIGATION_AND_PLAN.md` §5.2 and the plan §5.

Legend: **allow** = v0.1 in scope; **read** = read-only, may stay hidden; **deny** = must be
denied in server mode (currently no deny-list exists; see F-03).

| Variant | Writes | Network | Server mode |
|---|---|---|---|
| `Ping` | no | no | allow |
| `Version` | no | no | allow |
| `ListDevices` | no | no | allow (roots only) |
| `InspectDevice` | no | no | allow (root-scoped) |
| `ScanLibrary` | no | no | allow |
| `ImageInfo` | no | no | allow |
| `ListDir` | no | no | allow (root-scoped) |
| `PlanInit` | plans mkdir | no | allow |
| `PlanImport` | plans copy/convert | no (reads archives) | allow restricted: `.iso`/`.zso`, no `compress`, no `layout: ul`, no `covers` |
| `ConvertGames` | plans UL/ZSO conversion + delete | no | deny |
| `RenameGame` | plans rename | no | allow |
| `DeleteGame` | plans delete | no | allow |
| `MoveGame` | plans move | no | allow |
| `PlanRecovery` | plans recovery | no | allow |
| `ExecutePlan` | executes | no | allow (confirm delete) |
| `DiscardPlan` | drops plan | no | allow |
| `GetJob` / `ListJobs` / `CancelJob` | no | no | allow |
| `FetchArt` | plans art writes | yes | deny |
| `FindArt` | no (job) | yes | deny |
| `ImportArt` | plans art writes | no (reads local sets) | deny (not in v0.1 allow-list) |
| `SetArt` | plans art write | no (local file/bytes) | allow |
| `ReadCfg` | no | no | allow |
| `WriteCfg` | plans CFG (+`cards`) | no | allow, but reject `cards` (VMC) |
| `ListVmc` | no | no | allow read-only |
| `CreateVmc` | plans VMC write | no | deny |
| `DeleteArt` | plans art delete | no | allow |
| `VerifyGames` | no (job) | no | deny (Redump deferred) |
| `RedumpStatus` | no | no | deny |
| `UpdateRedump` | no | yes | deny |
| `Attribution` | no | no | allow |
| `ArtCache` | no | no | allow |
| `ClearArtCache` | deletes app cache | no | allow (app data only) |
| `LoaderCatalog` | no | no | deny |
| `UpdateLoaderCatalog` | no | yes / local mirror | deny |
| `LoaderRelease` | no | yes / local mirror | deny |
| `DownloadLoader` | stores ELF | yes / local mirror | deny |
| `ImportLoader` | stores ELF | no | deny |
| `LoaderFiles` / `LoaderLanguages` | no | no | deny |
| `PlanInstallLoader` | plans install | no (uses stored ELF) | deny |
| `LoaderBootHints` | no | no | deny |
| `PlanUpdateLoader` | plans replace | no | deny |
| `PlanRollbackLoader` | plans restore | no | deny |
| `CachedLoaderRelease` | no | no | deny |

## 3. Filesystem writes, deletes, renames

Core is synchronous and goes through `yaoplm-device` (`DeviceFs`/`RelPath`), except the
scanner (`std::fs` directly) and the server upload/media (`tokio::fs`).

| Area | Code | Notes |
|---|---|---|
| Upload staging | `apps/server/src/upload.rs:99-139,149-215,217-271` | `tokio::fs`; inside `<root>/.oplmdash/uploads/<id>`; bypasses `HostFs` (F-01) |
| Plan execute | `crates/yaoplm-core/src/exec.rs` via `execute_with_events` | `.yaoplm-part` + sync + rename + `sync_dir`; journal; preflight |
| Init library | `ops::plan_init` | creates OPL folders |
| Import | `import::plan_import` | copy/move, UL split, ZSO compress, archives |
| Convert | `convert::*` | UL/ZSO/ISO, deletes the old form (needs confirm) |
| Rename / delete / move | `ops::*` | CFG/ART/CHT/VMC association handling |
| CFG | `settings::plan_settings` | backup, CRLF, preserve unknown keys |
| VMC | `settings` / `vmc` | formatted cards, contiguity check |
| Art set/delete/import | `art::set_art`, `art_pack::import_job` | PNG transcode, backups |
| Loader install/update/rollback | `loaders::*` | ELF + language pack + manifest + backups |
| Recovery | `recovery::plan_recovery` | journal rollback/resume |
| Art cache | `yaoplm-net/src/cache.rs` | `safe_key`; app data dir |
| Journal | `yaoplm-core/src/journal.rs` | `.yaoplm/journal/`; temp cleanup |
| `games.bin`/`ul.cfg` | `convert.rs:177`, `detect.rs`, `ul_cfg` | delete after folder changes; UL record updates |

## 4. Network requests

All through one client: `crates/yaoplm-net/src/http.rs:94-107` (User-Agent, 15 s connect,
60 s request, redirects limited to 5, 1 req/s per source, size cap, retries).

| Source | Host(s) | Code |
|---|---|---|
| xlenore covers | `raw.githubusercontent.com` | `sources.rs:255` |
| libretro thumbnails | `thumbnails.libretro.com`, `raw.githubusercontent.com` | `sources.rs:362-363` |
| OPLM art backup | `archive.org` (range requests) | `sources.rs:125`, `oplm.rs` |
| Screenscraper | `api.screenscraper.fr` | `screenscraper.rs:41` |
| Redump DAT | `redump.info` | `redump.rs:40` |
| Loader catalogue/API | `api.github.com`, `raw.githubusercontent.com` | `loaders.rs:44-47` |
| Loader downloads | `github.com` | `loaders.rs:45` |
| Mirrors (tests/offline) | local folder (resolved via `PathPolicy`) | `lib.rs:1077-1089` |

`YAOPLM_OFFLINE` (`http.rs:287-290`) refuses non-loopback initial URLs. Note: the redirect
policy does not re-check the host after a redirect, so a loopback URL that redirects to a
public host is followed in offline mode; see F-03/F-09 notes.

## 5. File inventory

- Server: `apps/server/src/{lib,main,auth,rpc,events,media,assets,upload}.rs`, `tests/http.rs`.
- API: `crates/yaoplm-api/src/*.rs` (17 modules, ~8.9k LOC).
- CLI: `apps/cli/src/{main,report}.rs`, `tests/cli.rs`.
- Core/format/device/hash/net/i18n: `crates/*`.
- Web: `web/src` (~16.5k TS/TSX), generated API types in `web/src/api/generated/`.
- Packaging: `packaging/docker/Dockerfile`, `compose.yaml`, `.dockerignore`, `.env.example`.
- CI: `.github/workflows/{ci,fuzz,release-desktop}.yml`.
- Fuzz: `fuzz/fuzz_targets/{iso,system_cnf,cfg,ul_cfg,zso,cht,tar,ps2_elf}.rs`.
