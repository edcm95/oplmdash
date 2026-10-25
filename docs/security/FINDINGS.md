# oplmdash security findings

Target: `0b545c4ff78e57342315ad65691e452c75192ef3` + uncommitted Phase 0 work.
Branch: `security/audit-2026-10`.
Date: 2026-10-25.
Method: source review, inventory, threat model, targeted static checks. No dynamic tests were
run in this container (no `cc`, no Docker, no scanners; see `AUDIT_REPORT.md` §4).

Severity follows `SECURITY_AUDIT_PLAN.md` §7. Every entry has title, severity, location,
preconditions, reproduction, impact, fix, regression test and status.

## Summary

| ID | Severity | Title | Status |
|---|---|---|---|
| F-01 | Critical | Upload staging symlink escape -> arbitrary write outside the root | Fixed in code; tests pending |
| F-02 | High | Upload has no quota/free-space/target-limit/checksum/cleanup; buffers and serializes all uploads | Fixed in code; tests pending |
| F-03 | High | Full inherited RPC surface reachable in server mode; no allow-list | Fixed in code; tests pending |
| F-11 | High | `ScanLibrary` follows symlinks and can read outside the root | Fixed in code; tests pending |
| F-04 | Medium | No rate limiting, request timeouts or connection limits | Open |
| F-05 | Medium | No CSP / frame protection / security headers | Open |
| F-06 | Medium | Token in URL/`localStorage`; no session, logout or rotation | Open |
| F-07 | Medium | Internal errors leak host paths and OS details | Open |
| F-08 | Medium | Docker/Compose hardening gaps (curl+shell, unpinned images, no read-only rootfs/limits) | Open |
| F-09 | Medium | Supply-chain/CI gaps (mutable action tags, no audits/SBOM/image scan) | Open |
| F-10 | Medium | No read-only mode; `/opl` mounted read-write | Open |
| F-12 | Medium | `/media/art` TOCTOU and unbounded in-memory read | Open |
| F-13 | Medium | SIGTERM not handled; Docker stop can kill a write | Open |
| F-14 | Medium | On-device sources checked by length only between plan and execute | Open |
| F-15 | Low | `Host: localhost:evil` accepted as loopback | Open |
| F-16 | Low | Any non-empty token accepted (no length/entropy check) | Open |
| F-17 | Low | `/health` discloses the version without auth | Open |
| F-18 | Low | Debug build reads `web/dist` from disk | Open |
| F-19 | Low | `ExternalLink` accepts any URL scheme | Open |
| F-20 | Low | No `minimumReleaseAge` in pnpm config | Open |
| F-21 | Low | Compose publishes on all interfaces; PUID/PGID can be 0 | Open |
| F-22 | Info | Dockerfile provenance label is wrong | Open |
| F-23 | Info | `constant_time_eq` leaks the token length | Open |
| F-24 | Info | `forbidden()` returns code `Unauthorized` with HTTP 403 | Open |

Verified/mitigated items are in §3.

---

## F-01 — Upload staging symlink escape -> arbitrary write outside the root

**Severity:** Critical (plan §7: "arbitrary write outside a root").

**Location:** `apps/server/src/upload.rs:99-139` (`create_session`), especially:

```rust
let policy = PathPolicy::Within(state.config().roots.clone());
let root = policy.resolve(&body.root)?;              // canonicalised
...
let dir = root.join(STAGING_DIR).join(UPLOADS_DIR).join(&id); // .oplmdash/uploads/<id>
tokio::fs::create_dir_all(&dir).await?;              // follows symlinks
let path = dir.join(&body.name);
OpenOptions::new().write(true).create_new(true).open(&path).await
```

**Preconditions:** an attacker can create a symlink at `<root>/.oplmdash` (a local
unprivileged process with write access to the root, or another SMB client with write access to
a Linux-backed share). The API itself cannot create symlinks, so a token-only remote attacker
cannot do this; a local attacker or a second writer can.

**Reproduction (code-level; not executed here):**

```sh
ROOT=/tmp/opl; mkdir -p "$ROOT" /tmp/escape
ln -s /tmp/escape "$ROOT/.oplmdash"
# with a valid token and Host/Origin:
curl -sS -X POST http://127.0.0.1:7878/api/uploads \
  -H 'Authorization: Bearer <token>' -H 'Content-Type: application/json' \
  -d '{"root":"/tmp/opl","name":"pwn.txt","size":4}'
# returns id + path; then:
curl -sS -X POST http://127.0.0.1:7878/api/uploads/<id>/chunk \
  -H 'Authorization: Bearer <token>' -H 'Content-Type: application/octet-stream' \
  -H 'Upload-Offset: 0' --data-binary 'pwn!'
# /tmp/escape/uploads/<id>/pwn.txt now exists; PlanImport would reject the
# canonicalised path (outside the root), but the write already happened.
```

**Impact:** arbitrary file creation with attacker-controlled content and a
validated-but-attacker-chosen file name (no separators/control characters) in any directory
the server UID can write. Depending on the host, this can overwrite a config, a service file,
a `authorized_keys`, or a writable script. It is a privilege boundary crossing: the local
attacker may not be able to write those directories, the server can. The subsequent
`PlanImport` rejects the path, but that is after the damage.

**Fix:** route staging through the same safe layer as the core:
`HostFs::new(&root)?.create_dir(&RelPath::new(".oplmdash/uploads/<id>")?)` and `create` on
`.oplmdash/uploads/<id>/<name>`; `HostFs::resolve_checked` rejects a symlinked `.oplmdash`.
Alternatively canonicalize the staging directory after creation and require
`starts_with(root)`, and refuse a staging path whose `.oplmdash` or `uploads` component is a
symlink (`symlink_metadata`); use `create_new(true)` as now. Add a `Config` toggle to disable
uploads.

**Regression test:** in `apps/server/src/upload.rs` tests, create `<root>/.oplmdash` as a
symlink to a temp dir outside the root; POST `/api/uploads`; assert the request fails (or the
file is only inside the root) and that nothing appears outside. Also test a symlinked
`.oplmdash/uploads` subdirectory.

**Status:** Fixed in code; tests pending. `create_session` now refuses a symlinked
`.oplmdash`/`uploads`, canonicalises the created staging directory and requires it to stay
inside the root, and uploads are off unless `--uploads` is given.

---

## F-02 — Upload has no quota/free-space/target-limit/checksum/cleanup; buffers and serializes all uploads

**Severity:** High (plan §7: "disk exhaustion that takes down the host").

**Location:** `apps/server/src/upload.rs:30` (`MAX_CHUNK`), `:36-47` (in-memory sessions),
`:99-139` (create, no free-space/FAT32 check), `:149-215` (`Bytes` body, global mutex held
across I/O), `:217-254` (finish, no checksum), `:256-271` (cancel).

**Preconditions:** a token holder (the plan's "user with the token") or an XSS that steals the
token.

**Reproduction:** create an upload with a huge `size` on a small/limited volume and send
chunks until the disk is full; the server returns an internal error but the partial file
stays. Alternatively open a slow chunk write against a slow disk; every other upload waits on
the single `uploads` mutex. Repeat with many concurrent 16 MiB chunks to grow memory (each
request buffers its body before the handler). Restart the server: sessions are lost, staged
files remain.

**Impact:** authenticated disk exhaustion (can take down the host), upload DoS, data loss on
restart, orphaned files, no integrity check, no target-filesystem preflight (a >4 GiB upload
to FAT32 writes until the FS refuses, contradicting the v0.1 decision D2), and upload writes
run outside the `Queue::Write` single-writer model (`jobs.rs:267-289,422-540`), so they can
interleave with a plan on the same root.

**Fix:** per-upload/total quota and free-space preflight (reuse the core's preflight and
`FsInfo`); enforce the target FS max file size and the FAT32 4 GiB rule with a clear error;
stream to the file instead of `Bytes` (or cap concurrent buffered chunks); move the mutex to
per-session and do not hold it across I/O; integrate with a per-root writer lock; clean stale
sessions/files on startup and on a timer; add an optional SHA-256 computed while streaming and
stored in the audit.

**Regression tests:** quota/free-space refusal; FAT32 4 GiB refusal; restart cleanup; two
concurrent uploads do not serialize/jump; concurrent upload + `ExecutePlan` do not interleave;
checksum mismatch refuses `finish`.

**Status:** Fixed in code; tests pending. Uploads now enforce the configured size limit, a
FAT32 4 GiB check and a free-space preflight; sessions use per-upload locks; the server sweeps
stale staging directories on startup and hourly; and completed uploads return a SHA-256 digest.

---

## F-03 — Full inherited RPC surface reachable in server mode; no allow-list

**Severity:** High (plan §8 exit criterion: "Out-of-scope RPC variants are denied in server
mode, not only hidden in the UI"; plan §7 "authenticated write / data loss").

**Location:** `apps/server/src/lib.rs:78-91` adds only the auth layer;
`crates/yaoplm-api/src/lib.rs:236-1073` dispatches every `Request`;
`crates/yaoplm-api/src/protocol.rs:107-568` defines 46 variants. No server-mode filter exists.

**Preconditions:** the token (or an XSS that steals it).

**Reproduction:** `POST /api/rpc {"type":"ConvertGames",...}`, `{"type":"CreateVmc",...}`,
`{"type":"FetchArt",...}`, `{"type":"DownloadLoader",...}`, `{"type":"PlanInstallLoader",...}`,
`{"type":"UpdateRedump",...}` all dispatch. The UI hiding them does not stop a direct call.

**Impact:** a token holder can invoke operations v0.1 deferred: UL/ZSO conversion and deletion,
VMC creation, art fetching/importing, loader download/install/update/rollback, Redump
update/import, loader import, and the reads around them. This can destroy data, run network
requests, or install a binary, outside the product's stated scope. It is the largest
authenticated abuse surface.

**Fix:** add a server-mode mode/allow-list to `Config` and check it before `dispatch`; default
deny in server mode. Keep `Ping`, `Version`, `ListDevices`, `InspectDevice`, `ScanLibrary`,
`ImageInfo`, `ListDir`, `PlanInit`, restricted `PlanImport` (`.iso`/`.zso`, no compress/UL/
covers), `RenameGame`, `DeleteGame`, `MoveGame`, `PlanRecovery`, `ExecutePlan`, `DiscardPlan`,
`GetJob`, `ListJobs`, `CancelJob`, `SetArt`, `ReadCfg`, `WriteCfg` (reject `cards`),
`ListVmc` (read), `DeleteArt`, `Attribution`, `ArtCache`, `ClearArtCache`. Deny the rest.
See `INVENTORY.md` §2. Also add a read-only mode (F-10).

**Regression tests:** table-driven test over every `Request`: allowed variants reach dispatch;
denied variants return `Forbidden` from the server layer; `PlanImport` with `compress: true`,
`layout: "ul"`, `covers`, or an archive/BIN/CUE/CHD source is rejected; `WriteCfg` with
`cards` is rejected.

**Status:** Fixed in code; tests pending. `rpc::handle` parses each request and refuses anything
outside `allowed_in_server_mode` unless `--rpc-allow-all` is given; the default server surface is
the v0.1 allow-list, including the `PlanImport`/`WriteCfg` restrictions.

---

## F-11 — `ScanLibrary` follows symlinks and can read outside the root

**Severity:** High (plan §7: "path traversal"; plan §4: "No read escapes a root except
explicitly allowed staging").

**Location:** `crates/yaoplm-core/src/fsutil.rs:38-63` (`Listing::read` uses
`entry.metadata()`, which follows symlinks, and does not skip `is_symlink`); used by
`crates/yaoplm-core/src/scan.rs:98,184,589,593` and `detect.rs` for `ul.cfg`, `SYSTEM.CNF`,
`conf_opl.cfg`, `games.bin`. `crates/yaoplm-api/src/dir.rs:142-146` correctly skips symlinks,
but the scanner does not.

**Preconditions:** an attacker can create a symlink in the library root (local process or SMB
writer). The token is needed to call `ScanLibrary`.

**Reproduction:** `ln -s /etc/shadow <root>/ul.cfg` (or `ln -s /some/other/library
<root>/CD`), call `ScanLibrary`; the scanner reads the target and parses it. For a symlinked
`ul.cfg`, the first 32 bytes of each record become a "name" and the scanner can surface them
in the library result (depending on the parser's validation); a symlinked `CD`/`DVD`/`ART`/
`CFG` directory is scanned from outside the root.

**Impact:** read escape / information disclosure. The scanner is read-only, but it can read
arbitrary files outside the root and, through parser-derived fields, show some of their bytes
in the UI (`ul.cfg` names, warnings). It also breaks the "roots are a sandbox" invariant and
makes later `ImageInfo`/media errors confusing (those correct paths are rejected).

**Fix:** in `Listing::read`, use `entry.file_type()` and skip symlinks (or mark them and
refuse to descend/read); alternatively route the scanner through `HostFs` (`resolve_checked`)
so the same canonicalization applies. Add a test: a symlinked `ul.cfg`/`CD` must not be read.

**Status:** Open.

---

## F-04 — No rate limiting, request timeouts or connection limits

**Severity:** Medium (plan §7: "authenticated DoS"; plan §5 known risk #2).

**Location:** `apps/server/src/lib.rs:78-91` (no tower layer); `rpc.rs:10-29` (no timeout;
`spawn_blocking`); `upload.rs:83-90` (only body limit); `events.rs` (WS per connection);
`/health` is unauthenticated.

**Preconditions:** LAN access (for `/health`/slowloris) or the token (for `/api`).

**Impact:** authenticated DoS of CPU, memory, blocking pool, uploads and WS connections;
unauthenticated connection/slowloris pressure on `/health`; token brute-force without lockout
(the token is long, but there is no backoff). A token holder can create plans (max 32, but
eviction churn), long scans, many uploads or WS streams.

**Fix:** per-IP and per-token rate limits, request timeouts (including `spawn_blocking` work),
max body size for `/api/rpc`, max concurrent uploads/WS connections, connection limits and
idle timeouts, and a token/lockout backoff. Consider `tower-governor`/`tower-http` or a small
custom layer.

**Regression tests:** N+1 requests return 429; a slow request is timed out; WS/upload limits.

**Status:** Open.

---

## F-05 — No CSP / frame protection / security headers

**Severity:** Medium (plan §7: "missing security headers with a plausible exploit"; clickjacking
of an authenticated UI is plausible).

**Location:** `apps/server/src/lib.rs`, `assets.rs`, `media.rs` (no header middleware);
`web/index.html` (no CSP meta).

**Impact:** the authenticated UI can be framed (`frame-ancestors` missing) and a victim who is
already authenticated can be tricked into clicking delete/execute-plan buttons; XSS impact is
higher because the token is in `localStorage` (F-06); MIME sniffing, referrer leakage and
missing `Permissions-Policy`/CORP.

**Fix:** add a middleware with `Content-Security-Policy` (`default-src 'self'`; adjust
`script-src`/`style-src` for Vite/Tailwind; use a hash/nonce for `/theme-init.js`),
`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY` and `frame-ancestors 'none'`,
`Referrer-Policy: no-referrer`, `Permissions-Policy`, `Cross-Origin-Resource-Policy`; HSTS at
the proxy. Test with Playwright/axe and a CSP report-only pass.

**Regression tests:** assert headers on `/`, `/api/rpc`, `/media/art`; Playwright frame test.

**Status:** Open.

---

## F-06 — Token in URL/`localStorage`; no session, logout or rotation

**Severity:** Medium (plan §5 known risk #1; sensitive information leak; High if proxy logs are
exposed or an XSS is found).

**Location:** `apps/server/src/auth.rs:26-41` (query token for `/api/events` and `/media/*`);
`web/src/api/transport.ts:120` (WS query token), `:200-215` (`localStorage`, `?token=`);
`web/src/api/media.ts:50-51` (token in `<img>` URL); no logout/rotation code.

**Preconditions:** token in a proxy access log, browser storage, screenshot or an XSS.

**Impact:** the shared token is a bearer credential for the whole library. It appears in the
reverse-proxy access log for the initial `?token=` request, every WS handshake and every media
URL; it sits in `localStorage` (readable by any XSS); there is no logout, rotation or
per-user session, so a leak cannot be revoked except by restarting with a new token. Multi-tab
and shared-token users cannot be isolated (upload ids are not an authorization boundary).

**Fix:** implement session login (HttpOnly, Secure, SameSite=Strict cookie), a CSRF token for
state-changing requests, a short-lived ticket for WS/media (POST with `Authorization`, then a
one-time query ticket), logout and rotation; stop putting the long-lived token in URLs; redact
`?token=` at the proxy; document the fallback if session auth is deferred (trusted LAN/VPN +
TLS + long token).

**Regression tests:** cookie flags; CSRF rejection; logout invalidates; WS/media ticket expiry;
query token rejected when sessions are on.

**Status:** Open. The plan's exit criteria allow this only with a trusted LAN/VPN + token + TLS
deployment (see `DEPLOYMENT_CHECKLIST.md`).

---

## F-07 — Internal errors leak host paths and OS details

**Severity:** Medium (plan §7: "sensitive information leak").

**Location:** `apps/server/src/upload.rs:104,117,126,203,236,250,267` and `media.rs:92` use
`ApiError::internal(error.to_string())`; `rpc.rs:27` uses `join.to_string()`; `upload.rs:78`
returns the absolute staged path; `main.rs:62,70` logs canonical roots/data dir.

**Impact:** an authenticated client learns host paths, mount points, UIDs and OS errors, which
helps chaining (F-01/F-11) and reconnaissance. Job logs may carry the same.

**Fix:** map internal errors to generic phrased messages in server mode; log details
server-side with `tracing`; return a relative staging path or an opaque id, not an absolute
host path.

**Regression tests:** force an I/O error (unwritable root) and assert the body has no host
path; check job logs for redaction.

**Status:** Open.

---

## F-08 — Docker/Compose hardening gaps

**Severity:** Medium (plan §7: hardening/container escape defense-in-depth; plan §5 known risks
#7/#8).

**Location:** `packaging/docker/Dockerfile`, `compose.yaml`.

- runtime is `debian:bookworm-slim` with `curl` and a shell (only `curl` is used by the
  healthcheck);
- base images are not pinned by digest;
- no `read_only: true`, `tmpfs`, custom seccomp/AppArmor profiles or resource limits
  (`cpus`, `mem_limit`, `pids_limit`);
- compose lets `PUID=0`/`PGID=0` (root) and mounts `/opl` read-write;
- `YAOPLM_OFFLINE` is not set in compose;
- healthcheck hardcodes port 7878 and depends on `curl`;
- no `stop_grace_period`; the server does not handle SIGTERM (F-13).

Good: `cap_drop: ALL`, `no-new-privileges:true`, `USER oplmdash` (but compose overrides),
`.dockerignore` excludes `.env`, test data and images.

**Impact:** larger attack surface (shell/curl), supply-chain drift (mutable base tags),
missing containment (no read-only rootfs/seccomp/limits), accidental root via PUID=0, and
egress not restricted.

**Fix:** pin digests; remove curl/shell (distroless or a tiny `--healthcheck` subcommand);
`read_only: true` plus tmpfs for writable dirs; seccomp/AppArmor; resource limits; validate
non-root PUID/PGID; document a read-only `/opl` mount; set `YAOPLM_OFFLINE=1` if v0.1 does not
fetch; make the healthcheck port configurable; add `stop_grace_period` and SIGTERM handling.

**Regression tests:** image build + `trivy`/`docker scout`; `docker compose up` with
`read_only`/limits; a healthcheck against a non-default port.

**Status:** Open.

---

## F-09 — Supply-chain/CI gaps

**Severity:** Medium (plan §7: compromised dependency/base image/Action; plan §5 known risk #8).

**Location:** `.github/workflows/ci.yml`, `fuzz.yml`, `release-desktop.yml`.

- Actions are pinned by mutable tags (`actions/checkout@v7`,
  `Swatinem/rust-cache@v2`, `actions/setup-node@v7`, `pnpm/action-setup@v4`,
  `tauri-apps/tauri-action@v1`, `actions/upload-artifact@v7`), not by commit SHA.
- No `cargo audit`, `cargo deny`, `pnpm audit`, `gitleaks`, `trivy`, `syft`/SBOM; no Docker
  image build/scan job.
- CI does run fmt, clippy, tests, core purity, TS bindings, web lint/typecheck/test/build,
  e2e, FAT32/exFAT real-FS tests, fuzz-target build, OPL cross-checks; `YAOPLM_OFFLINE: 1` is
  set (`ci.yml:22`). Good.

**Impact:** unknown advisories/yanked crates, mutable action tags, no provenance/SBOM, no
image scan.

**Fix:** pin every Action by SHA; add `cargo audit`/`deny`, `pnpm audit`, `gitleaks`, `trivy`
(fs and image), `syft` SBOM artifacts; fail on High/Critical; build and scan the image in CI.

**Status:** Open. (The audit container lacks these tools; they were not run.)

---

## F-10 — No read-only mode; `/opl` mounted read-write

**Severity:** Medium (containment; plan §5 known risk #6, exit criterion).

**Location:** `apps/server/src/lib.rs:26-35` (`Config` has no `read_only`);
`compose.yaml` mounts `${OPL_ROOT}:/opl` read-write; there is no API-layer write block.

**Impact:** a compromised token/server can alter or delete the library; there is no way to run
the dashboard safely against a backup or a read-only mount. The plan's exit criteria allow
deferral only for a trusted LAN/VPN + token + TLS deployment.

**Fix:** add `read_only` to `Config` and deny every mutating `Request` (and the upload
endpoints) at the API layer, not only in the UI; support `/opl:ro` and document it; add a
startup warning when read-only is off.

**Regression tests:** every mutating variant returns `Forbidden` in read-only mode; uploads
are disabled; reads still work.

**Status:** Open.

---

## F-12 — `/media/art` TOCTOU and unbounded in-memory read

**Severity:** Medium (read escape/DoS; local write + token).

**Location:** `apps/server/src/media.rs:40-101`: `policy.resolve` canonicalizes, then
`metadata` and `tokio::fs::read` operate on the canonical path; no size cap.

**Preconditions:** write access to the ART folder (local process/SMB) and the token.

**Impact:** a symlink swap between resolve and read can serve a file outside the root (read
escape). A huge file named like `ART/<ID>_COV.png` is read fully into memory (memory DoS).

**Fix:** open via `HostFs::resolve_checked`/`open` with `O_NOFOLLOW` semantics, check the
opened file's metadata, enforce a small size cap (e.g., 8 MiB) and image validation; stream or
reject.

**Regression tests:** symlink swap (or a symlinked final component) is rejected; an oversized
"art" file is rejected without reading it all.

**Status:** Open.

---

## F-13 — SIGTERM not handled; Docker stop can kill a write

**Severity:** Medium (data safety/availability).

**Location:** `apps/server/src/main.rs:85-87` handles only `ctrl_c`; `compose.yaml` sets no
`stop_grace_period`; Docker sends SIGTERM.

**Impact:** `docker stop`/restart kills the process after the timeout, possibly during a write
or a plan. The core has a journal/recovery, but graceful shutdown and a longer grace period
are expected; queued jobs may be lost.

**Fix:** handle SIGTERM (`tokio::signal::unix::signal(SignalKind::terminate())`) and use it
with `with_graceful_shutdown`; set `stop_grace_period`; optionally drain/finish or cancel
jobs and report.

**Regression tests:** send SIGTERM during a copy and assert the journal is recoverable and the
process exits cleanly.

**Status:** Open.

---

## F-14 — On-device sources checked by length only between plan and execute

**Severity:** Medium (data integrity; local write access).

**Location:** `crates/yaoplm-core/src/exec.rs:844-880` (`check_sources`): for
`source.on_device`, only `meta.len == file.len` is checked (no mtime/hash); host sources check
length and mtime. `PlanImport.verify` defaults off (`protocol.rs:199-204`).

**Preconditions:** local write access to the staged upload (or another writer) between plan and
execute; a same-length content change.

**Impact:** a plan copies modified content; a malicious local process can swap a game image for
another of the same size and the plan will not notice (unless `verify` is on). The upload
staging file is on the device, so this applies to the upload flow.

**Fix:** default `verify`/hash for uploads, or include size+mtime and a hash in the plan and
verify at execute; re-check the upload session's `complete` and the file's identity; consider
`fsync`/immutable staging.

**Regression tests:** modify a staged file with the same length between plan and execute;
execution must fail with `SourceChanged`.

**Status:** Open.

---

## Lower-severity and informational

**F-15 (Low) — `Host: localhost:evil` accepted as loopback.** `auth.rs:71-77` strips at the
last `:` when the prefix has no `:`, so `localhost:evil` becomes `localhost`. Only matters for
a loopback bind without a token behind a proxy that preserves Host. Fix: parse the authority
strictly (`http::uri::Authority` or validate the port is a `u16`; keep bracketed IPv6).
Test: `localhost:evil`, `127.0.0.1:notaport`, `[::1]:x`.

**F-16 (Low) — Any non-empty token accepted.** `main.rs:46` (`args.token.filter(|t| !t.is_empty())`);
the plan's own `docker run` example uses `YAOPLM_TOKEN=audit-only`. Fix: require a minimum
length/entropy (e.g. ≥32 random bytes) or warn loudly; document `openssl rand -hex 32`.

**F-17 (Low) — `/health` discloses the version without auth.** `lib.rs:93-95`. Fix: return
`{"status":"ok"}` only, or keep the version on the authenticated `Version` request.

**F-18 (Low) — Debug build reads `web/dist` from disk.** `assets.rs:6-10` (`rust-embed`
debug behaviour). A writable `web/dist` in a debug deployment could inject JS. Release embeds.
Document "release only" or make debug read-only/immutable.

**F-19 (Low) — `ExternalLink` accepts any URL scheme.** `web/src/components/ExternalLink.tsx:17-28`;
hrefs come from catalogue/attribution data (`entry.licenseUrl`, `change.link`,
`releaseUrl`, `doc.url`, verification URLs). A poisoned catalogue (e.g., a writable mirror)
could supply `javascript:`. Fix: allow only `http:`/`https:` and render other schemes as text.
Test a `javascript:` href.

**F-20 (Low) — No `minimumReleaseAge` in pnpm config.** `pnpm-workspace.yaml` lists only
packages; no `.npmrc`; `package.json` pins exact versions and `pnpm-lock.yaml` is committed
(good). Verify the global config or add `minimumReleaseAge` to `pnpm-workspace.yaml`.

**F-21 (Low) — Compose publishes on all interfaces; PUID/PGID can be 0.** `compose.yaml`
publishes `${OPLMDASH_PORT:-7878}:7878` and `user: "${PUID:-1000}:${PGID:-1000}"`; an operator
can set 0. Document the firewall/LAN requirement and reject UID/GID 0 (or warn).

**F-22 (Info) — Dockerfile provenance label is wrong.** `packaging/docker/Dockerfile`:
`org.opencontainers.image.source="https://github.com/edcm95/yayaoplm"` (typo/wrong repo).
Fix the URL (and the image name if needed).

**F-23 (Info) — `constant_time_eq` leaks the token length.** `auth.rs:103-105` returns early
on a length mismatch; the length is not usually secret. Standard, but note it.

**F-24 (Info) — `forbidden()` returns code `Unauthorized` with HTTP 403.** `auth.rs:46-51`
uses `ApiError::unauthorized`; the body code and status disagree. Use a forbidden error code.

---

## 3. Verified / mitigated (no finding)

- **V-01 Core write path safety.** `RelPath` rejects empty/`.`/`..` components and separators
  (`crates/yaoplm-device/src/fs.rs:74-101`); `HostFs::resolve_checked` canonicalizes and
  requires `starts_with(root)` (`fs.rs:331-347`); `create` uses `create_new(true)`
  (`fs.rs:403-409`); `rename`/`remove_file`/`remove_dir` use `resolve_checked`
  (`fs.rs:419-429`); the final-component symlink is not followed for rename/remove
  (`fs.rs:344-346`). This protects the core's writes against path/symlink escape. A narrow
  canonicalize->open race remains; consider `openat2(RESOLVE_BENEATH)`/`O_NOFOLLOW` for
  defense in depth.
- **V-02 ExecutePlan confirmation.** `PlanStore` is single-use, TTL 1 h, max 32
  (`plans.rs:25-26,436-448`); `lib.rs:532-556` rejects a plan with blockers, an empty plan, or
  user-data deletion without `confirmDelete: true`; the plan is bound to its stored root.
- **V-03 Archive handling is read-only.** `ArchiveSources` reads zip/7z members as folders; it
  does not extract to the device (`archives.rs:1-12`); `archive::list` drops drives and `..`
  (comment at `archives.rs:172`); the importer writes via `DeviceFs`/`RelPath`, so zip-slip into
  the device is not applicable. Fuzz targets exist (`tar` etc.).
- **V-04 Cache path safety.** `safe_key` allows only `[A-Za-z0-9-_.(),+!'&]` and replaces the
  rest with `_` (`cache.rs:113-133`); cache paths are `data_dir/ART_DIR/safe_key(...)`.
- **V-05 `ListDir` root scope.** `dir.rs:115-175` resolves via `PathPolicy`, skips hidden
  entries and symlinks, caps at 5000 (`MAX_ENTRIES`), and never offers a parent outside a root.
- **V-06 `ListDevices` in server mode** lists only configured roots (`lib.rs:249-268`), never
  host disks.
- **V-07 Upload offset arithmetic.** Offset must match, chunk ≤ remaining, duplicate/concurrent
  chunks are serialized by the mutex, `finish` requires the declared size, `cancel` is
  idempotent (`upload.rs:161-271`). The mutex-across-I/O and buffering are F-02.
- **V-08 Token basics.** Non-loopback bind without a token fails at startup
  (`main.rs:47-53`); an empty token is no token (`main.rs:46`); comparison is constant-time
  (`auth.rs:103-105`); `Host` is loopback-checked in the no-token case; `Origin` must match
  `Host`.
- **V-09 CSRF basics.** `/api/rpc` requires `application/json` and `/api/uploads/*/chunk`
  requires `application/octet-stream` (`rpc.rs:11-18,47-53`; `upload.rs:291-297`), so a simple
  cross-site form cannot send a valid body; no CORS headers are set, so browsers block
  cross-origin reads; the token is still required.
- **V-10 Web XSS basics.** No `dangerouslySetInnerHTML`/`eval`/`innerHTML` in production
  `web/src` (only a test); React escapes rendered strings; `ExternalLink` uses
  `rel="noreferrer"` (implies noopener) and the Tauri opener plugin. CSP is still missing
  (F-05) and the URL scheme is not checked (F-19).
- **V-11 CI coverage.** `ci.yml` sets `YAOPLM_OFFLINE: 1` and runs fmt, clippy, tests, core
  purity, TS bindings, web lint/typecheck/test/build, e2e, FAT32/exFAT real-FS tests, fuzz
  build and OPL cross-checks. Missing audits/scans are F-09.
- **V-12 Repository secrets.** A grep of tracked files found no keys; `.env` is ignored;
  `.dockerignore` excludes `.env`, test data and images; the release workflow has no signing
  key in the repo (it says there is no updater key yet). `gitleaks` was not available, so this
  is not a full history scan (F-09).
- **V-13 Atomic writes and one-writer queue.** `write_file`/`write_zso` write to
  `.yaoplm-part`, sync, rename, then `sync_dir` (`exec.rs:534-625`); `write_atomic` for small
  files (`fs.rs:247-262`); preflight cleans temp files, checks journal and device fingerprint
  (`exec.rs:896-927`); `Queue::Write` runs writes one at a time (`jobs.rs:267-289,422-540`).
  Uploads bypass this (F-02).

## 4. Not executed / test gaps

- `cargo test`, `cargo clippy`, `cargo fmt --check`, `cargo audit`, `cargo deny`: the audit
  container has no `cc`, so crates cannot build. Run in the Docker builder/CI.
- `pnpm -C web lint|typecheck|test|build`: `pnpm` is not installed (corepack is present but a
  network fetch would be needed). Run in CI.
- `docker build`, `trivy`, `docker scout`, `hadolint`, `syft`, `gitleaks`: not installed.
- Fuzzing (`cargo +nightly fuzz run ...`): nightly not set up here; targets exist and CI
  compiles them; run the weekly job and add upload/archive cases.
- Fault injection (kill during copy/rename/delete/CFG/`ul.cfg`/`games.bin`, disk full,
  permission denied, CIFS rename failure, power loss) and `games.bin`/`ul.cfg` invariants:
  not exercised. Run `cargo test --workspace` and the `--ignored` suites with
  `YAOPLM_TESTDATA`, plus the real-FS jobs.
