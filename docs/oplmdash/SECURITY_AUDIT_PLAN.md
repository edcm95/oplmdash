# oplmdash security audit plan

Status: ready to execute
Created: 2026-10-25
Target: `oplmdash` downstream fork of YAOPLM
Branch: `oplmdash/v0.1`
Base commit: `0b545c4ff78e57342315ad65691e452c75192ef3`
Working tree: Phase 0 changes are currently uncommitted; freeze them before the audit.

Related documents:

- `docs/oplmdash/INVESTIGATION_AND_PLAN.md`
- `docs/oplmdash/DECISIONS.md`
- `docs/SPIKE.md`
- `FORK.md`, `NOTICE`
- Upstream: `docs/ARCHITECTURE.md`, `docs/SPEC.md`, `docs/DECISIONS.md`, `docs/research/`

## 0. How to use this plan

1. Read `CLAUDE.md`, `FORK.md`, `docs/oplmdash/INVESTIGATION_AND_PLAN.md`, and this file.
2. Freeze the target: commit the Phase 0 work or record `git diff` as a patch and the exact
   `git rev-parse HEAD`.
3. Work on a dedicated branch, for example `security/audit-2026-10`.
4. Do not test against a real removable drive or the owner's live library. Use temp directories,
   fixture libraries, and read-only copies.
5. Record every finding with evidence and a reproducible test. Do not fix silently; keep fixes in
   separate commits with tests.
6. Stop and ask before any destructive test, external scan, live network request, or change to the
   owner's environment.

The audit is complete only when the deliverables in section 9 exist and the exit criteria in
section 8 are met.

## 1. Scope and assumptions

### In scope

- `apps/server`: HTTP routes, auth, upload, static files, media, WebSocket events, Docker packaging.
- `apps/cli`: commands that can write, delete, move, or fetch over the network.
- `crates/yaoplm-api`: every `Request` variant, plan/execute boundary, jobs, path policy, settings.
- `crates/yaoplm-core`, `yaoplm-formats`, `yaoplm-device`, `yaoplm-hash`, `yaoplm-net`,
  `yaoplm-i18n`: parsers, filesystem operations, network clients, caches.
- `web`: React SPA, token handling, API client, upload UI, XSS/CSRF surfaces, i18n rendering.
- `packaging/docker/Dockerfile`, `compose.yaml`, `.dockerignore`, `.env.example`.
- CI workflows, dependency lockfiles, release/signing configuration.
- Data-safety logic: plan → confirm → execute, journal, backups, atomic writes, recovery.
- OPL format writers and readers, because a malformed write can destroy a library.

### Out of scope

- OPL firmware, the PS2 console, the user's NAS OS, and the reverse proxy configuration unless the
  owner provides it for review.
- Full source review of third-party crates and npm packages; use advisory/SBOM tools and review
  only high-risk or patched dependencies.
- The Tauri desktop app unless it shares a changed security boundary. The server is the target.
- The owner's home network, Wi-Fi, DNS, and physical security.

### Deployment assumptions to verify

- Private server, reachable from a LAN or VPN, behind a TLS reverse proxy.
- A long random `YAOPLM_TOKEN`; no token in source control.
- The OPL folder is a bind mount; the container runs non-root.
- The app is not exposed directly to the public internet.
- The owner keeps backups of the library.

If any assumption is false, raise the severity of the related findings.

### Assets

- The OPL game library and all user files under the configured roots.
- The token, Screenscraper credentials, signing keys, and any `.env` contents.
- The host filesystem outside the roots, through path or symlink bugs.
- Availability of the server, the NAS, and the library.
- Integrity of `ul.cfg`, `games.bin`, CFG, ART, VMC, CHT, and APPS data.
- The user's browser session and any data in `localStorage`.

### Threat actors

- A LAN attacker without the token.
- A malicious website opened in the owner's browser (CSRF, DNS rebinding, XSS, token theft).
- A user with the token but without shell access (abuse of inherited write/RPC endpoints).
- A malicious or corrupted game image, archive, CFG, CHT, VMC, or ZSO.
- A compromised dependency, base image, GitHub Action, or release artifact.
- A local unprivileged process on the NAS.
- An attacker who obtains a leaked token from URL, logs, browser storage, or a screenshot.

### Trust boundaries

1. Browser ↔ server HTTP/WebSocket.
2. Server ↔ library roots on the host filesystem.
3. Server ↔ internet (art, loaders, Redump, Screenscraper).
4. Container ↔ host (mounts, UID/GID, capabilities).
5. Build/release pipeline ↔ runtime image.

## 2. Ground rules

- Read-only first. Make a copy of fixtures before testing writes.
- Never point tests at `D:` or any real removable drive.
- Use `tempfile`, fixture libraries, and `YAOPLM_OFFLINE=1` for tests that must not reach the
  network.
- Do not run automated scanners against the owner's live host without permission.
- Do not commit tokens, cookies, customer data, or screenshots with secrets.
- Redact secrets in reports; rotate any secret that may have leaked.
- Prefer proof by test: a failing test or a curl/Playwright repro is stronger than a comment.
- Keep findings reproducible after a clean checkout.

## 3. Audit phases

### Phase 0 — Freeze and inventory (0.5 day)

- [ ] Record `git rev-parse HEAD`, `git status --short`, and a patch of uncommitted changes.
- [ ] Generate a file inventory of server, API, core, formats, device, net, web, Docker, CI.
- [ ] Enumerate every HTTP route: method, path, auth middleware, body limit, handler, writes.
- [ ] Enumerate every `Request` variant in `crates/yaoplm-api/src/protocol.rs`; mark
      `v0.1 in scope`, `inherited but hidden`, or `must be disabled in server mode`.
- [ ] Enumerate every filesystem write, delete, rename, and network request.
- [ ] Generate an SBOM for Rust, npm, and the container base images.
- [ ] Write the threat model (assets, actors, boundaries, abuse cases) and confirm it with the owner.
- [ ] Create `docs/security/AUDIT_REPORT.md` and `docs/security/FINDINGS.md`.

Deliverable: inventory tables and a signed-off threat model.

### Phase 1 — Authentication, session, and browser boundary (1 day)

- [ ] Review `apps/server/src/auth.rs` and the token lifecycle.
- [ ] Verify constant-time comparison, empty-token handling, and startup refusal on a non-loopback
      bind without a token.
- [ ] Check `Origin`/`Host` validation, DNS rebinding protection, reverse-proxy behaviour, and
      `X-Forwarded-*` assumptions.
- [ ] Review the `?token=` path for WebSocket and `/media`; check leakage through logs, history,
      referrers, proxies, and screenshots.
- [ ] Review token storage in `localStorage`; assess XSS impact and the absence of HttpOnly cookies.
- [ ] Check brute-force resistance and rate limiting on `/api/*` and upload endpoints.
- [ ] Check the planned session/CSRF model against the current token-only model.
- [ ] Verify logout, token rotation, and multi-tab behaviour (or document that they do not exist).
- [ ] Review error messages for token or path disclosure.

Deliverable: auth findings with a concrete token-theft and CSRF abuse-case analysis.

### Phase 2 — Path and filesystem safety (1 day)

- [ ] Review `PathPolicy` and every `resolve` call.
- [ ] Test `..`, absolute paths, symlinks, hard links, junctions, UNC paths, Windows drive-relative
      paths, alternate data streams, trailing dots/spaces, case-insensitive collisions, and
      Unicode normalization.
- [ ] Test TOCTOU: replace a validated path with a symlink between check and open/write.
- [ ] Verify no write can escape a root through `PlanInit`, `PlanImport`, rename, move, delete,
      CFG, ART, VMC, CHT, APPS, loader install/update, or recovery.
- [ ] Review temp-file creation, permissions, atomic rename, `fsync`, journal, and backup paths.
- [ ] Test CIFS/SMB semantics: rename over an existing file, case folding, permission errors,
      interrupted writes, and stale handles.
- [ ] Test FAT32/exFAT limits and fragmentation checks; verify a failed check does not write.
- [ ] Verify delete/trash cannot remove a shared file used by another game or VMC.
- [ ] Check that `games.bin` and `ul.cfg` invalidation rules are followed after writes.

Deliverable: path-safety matrix and failing tests for every escape or data-loss case.

### Phase 3 — Upload and untrusted parsers (1.5 days)

- [ ] Review `apps/server/src/upload.rs` end to end.
- [ ] Test offset arithmetic, duplicate chunks, concurrent chunks, truncated bodies, huge
      `Upload-Offset`, size overflow, and finish before completion.
- [ ] Test disk exhaustion: no quota, no free-space preflight, many parallel sessions, stale
      `.oplmdash/uploads` directories.
- [ ] Test filename validation: traversal, control characters, reserved Windows names, very long
      names, Unicode, and names that collide on a case-insensitive filesystem.
- [ ] Test staging inside the root: scanner visibility, `PlanImport` acceptance, `transfer: move`,
      cleanup, and restart behaviour.
- [ ] Verify upload content is never executed; review parser exposure to hostile ISO/ZSO/CHD,
      zip/7z, CFG, CHT, VMC, `ul.cfg`, and `SYSTEM.CNF`.
- [ ] Check archive handling for zip-slip, absolute paths, symlinks, nested archives,
      decompression bombs, and size limits.
- [ ] Run and extend fuzz targets: `iso`, `system_cnf`, `cfg`, `ul_cfg`, `zso`, `cht`, `tar`,
      `ps2_elf`; add upload/archive cases.
- [ ] Check panics, integer overflows, unbounded allocations, and error messages in parsers.
- [ ] Add a checksum option and a cleanup policy if missing.

Deliverable: upload abuse-case report and fuzz findings.

### Phase 4 — HTTP and browser hardening (1 day)

- [ ] Review every route and response header in `apps/server/src/lib.rs`, `assets.rs`, `media.rs`,
      `rpc.rs`, `events.rs`, `upload.rs`.
- [ ] Add and verify: `Content-Security-Policy`, `X-Content-Type-Options: nosniff`,
      `X-Frame-Options`/`frame-ancestors`, `Referrer-Policy`, `Permissions-Policy`, and HSTS at the
      proxy.
- [ ] Check CORS: no wildcard, no credentials with `*`, preflight handling, and `Origin` rules.
- [ ] Check MIME types, content sniffing, static cache headers, ETag handling, and range requests.
- [ ] Check WebSocket origin validation, message size, idle timeout, and backpressure.
- [ ] Check error bodies and job logs for stack traces, host paths, tokens, and internal URLs.
- [ ] Review the SPA for XSS: `dangerouslySetInnerHTML`, `innerHTML`, `eval`, URL injection,
      markdown/HTML rendering, file-name display, i18n interpolation, and third-party widgets.
- [ ] Check token leakage through source maps, `localStorage`, `sessionStorage`, URL, logs, and
      screenshots.
- [ ] Check open redirects, `postMessage`, `window.opener`, and link handling.
- [ ] Review body limits and request timeouts on every route.

Deliverable: header diff, browser threat model, and XSS/CSRF repros where applicable.

### Phase 5 — RPC surface and inherited features (1 day)

- [ ] Build a table of all `Request` variants with: purpose, auth, writes, network, danger.
- [ ] Confirm the v0.1 allow-list: scan, library, details, art display/set/delete, rename, delete,
      move, CFG, import `.iso`/`.zso`, jobs, recovery.
- [ ] Disable or gate out-of-scope variants in server mode: convert, VMC, PS1/APPS, cheats,
      art fetching, loader download/install/update/rollback, Redump update/import, loader import,
      and any host-disk enumeration.
- [ ] Verify `ListDevices` in server mode only shows configured roots.
- [ ] Verify `ListDir` cannot list outside roots or expose sensitive host paths.
- [ ] Verify `ExecutePlan` requires the expected confirmation and cannot be replayed or confused
      with another root/plan.
- [ ] Verify job cancellation and recovery cannot bypass plan confirmation.
- [ ] Verify read-only mode (when added) blocks every write path, not just the UI.

Deliverable: RPC exposure matrix and a patch that denies out-of-scope operations in server mode.

### Phase 6 — Network egress, SSRF, and secrets (0.5 day)

- [ ] Inventory every outbound request: art sources, loader releases, Redump, Screenscraper,
      mirrors, update checks.
- [ ] Check URL allow-lists, redirects, DNS rebinding, private/link-local/metadata addresses,
      `file://`, response size limits, timeouts, and decompression limits.
- [ ] Verify `YAOPLM_OFFLINE=1` blocks every non-local host.
- [ ] Check cache poisoning and cache path traversal.
- [ ] Check TLS verification, proxy env vars, and User-Agent.
- [ ] Review secret handling: `YAOPLM_TOKEN`, Screenscraper dev id/password, signing keys, `.env`,
      logs, CI, Docker layers, and browser storage.
- [ ] Run `gitleaks` and review the history for accidental secrets.
- [ ] Verify error messages do not echo credentials.

Deliverable: egress/SSRF findings and a secrets-handling checklist.

### Phase 7 — Dependencies and supply chain (0.5 day)

- [ ] Run `cargo audit`, `cargo deny check`, `cargo tree`, and `cargo geiger`.
- [ ] Review advisories, yanked crates, duplicate versions, build scripts, `unsafe`, and licences.
- [ ] Run `pnpm audit`; review lockfile integrity, exact versions, and `minimumReleaseAge`.
- [ ] Check GitHub Actions pinning, permissions, secrets, and artifact provenance.
- [ ] Check Docker base image digests and scan the final image with `trivy`/`docker scout`.
- [ ] Generate SBOMs (`syft`) and compare with the build.
- [ ] Check release signing and updater keys are not in the repo.

Deliverable: dependency report with accepted risks and upgrade/patch plan.

### Phase 8 — Docker and deployment (0.5 day)

- [ ] Review `packaging/docker/Dockerfile` and `compose.yaml`.
- [ ] Verify non-root, UID/GID, `cap_drop: ALL`, `no-new-privileges`, read-only root filesystem,
      `tmpfs`, seccomp/AppArmor, and resource limits.
- [ ] Remove or justify `curl`/shell in the runtime image; consider distroless.
- [ ] Check build-stage leakage: source, caches, secrets, `.git`, `.env`, test data.
- [ ] Check volume permissions, bind-mount propagation, and read-only OPL mode.
- [ ] Check port exposure, reverse-proxy TLS, HSTS, timeouts, and upload body limits.
- [ ] Check healthcheck behaviour and log volume.
- [ ] Verify the container cannot reach host paths outside the mounts.
- [ ] Test image build and `docker compose up` on a disposable Docker host.

Deliverable: hardened Dockerfile/compose diff and a deployment checklist.

### Phase 9 — Data safety and crash consistency (1 day)

- [ ] Review the plan/journal/execute/rollback code and its tests.
- [ ] Kill the process during copy, rename, delete, CFG write, `ul.cfg` append, and `games.bin`
      invalidation; verify recovery.
- [ ] Fill the disk; verify a clear error and no partial file that OPL would read.
- [ ] Simulate permission denied, CIFS rename failure, stale handles, and power loss.
- [ ] Verify one writer per root and that concurrent jobs cannot interleave.
- [ ] Verify backups are created before destructive steps and are restorable.
- [ ] Verify delete requires explicit confirmation and uses trash by default.
- [ ] Verify a failed import does not leave a game visible to OPL in a broken state.
- [ ] Verify `ul.cfg`, `games.bin`, CFG, ART, VMC, CHT, and APPS invariants from the OPL audit.

Deliverable: fault-injection report and regression tests.

### Phase 10 — Verification, fixes, and report (1 day)

- [ ] Re-run the full test matrix after fixes.
- [ ] Add regression tests for every fixed finding.
- [ ] Re-test the original repros.
- [ ] Update `docs/oplmdash/DECISIONS.md` with security decisions.
- [ ] Write the final report with severity, evidence, fix, and status.
- [ ] Get owner sign-off and schedule a re-audit after major changes.

Deliverable: final report, patches, tests, and a go/no-go for private-server use.

## 4. Detailed checklist

### Authentication and authorization

- [ ] Token is required on every non-health route when configured.
- [ ] Token comparison is constant-time.
- [ ] Empty token is treated as no token.
- [ ] Non-loopback bind without token fails at startup.
- [ ] `Origin` and `Host` checks cannot be bypassed with casing, ports, IPv6, or proxies.
- [ ] Query token is limited to WebSocket/media and documented as a leak risk.
- [ ] Token is not stored in URLs longer than necessary.
- [ ] Token is not in logs, errors, job output, or source maps.
- [ ] Rate limiting exists for auth and upload.
- [ ] Session logout/rotation exists or is explicitly deferred.
- [ ] Read-only mode blocks writes at the API layer.

### Filesystem and paths

- [ ] All paths are canonicalized before policy checks.
- [ ] Symlink/TOCTOU races are tested.
- [ ] No write escapes a root.
- [ ] No read escapes a root except explicitly allowed staging.
- [ ] Windows reserved names, ADS, UNC, and trailing-dot cases are handled or rejected.
- [ ] Case-insensitive collisions are tested.
- [ ] Temp files are created in the target directory and renamed atomically.
- [ ] File and directory permissions are restrictive.
- [ ] CIFS/SMB failure modes are tested.
- [ ] Delete/trash cannot remove shared data.

### Upload

- [ ] Declared size and offset are validated.
- [ ] Duplicate/concurrent chunks are safe.
- [ ] Partial writes are truncated or retried safely.
- [ ] Disk exhaustion is handled.
- [ ] Quotas and free-space preflight exist.
- [ ] Stale uploads are cleaned up.
- [ ] Filenames are validated.
- [ ] Uploaded data is parsed with limits and fuzzed.
- [ ] `PlanImport` consumes staging only through the root policy.
- [ ] Checksum verification is available.

### HTTP and browser

- [ ] CSP is present and tested.
- [ ] `nosniff`, frame protection, referrer policy, permissions policy are present.
- [ ] CORS is strict.
- [ ] Body limits and timeouts are set.
- [ ] WebSocket origin and size are checked.
- [ ] Static files do not expose source, maps, or secrets.
- [ ] Error messages do not leak host paths or tokens.
- [ ] No XSS sinks in the SPA.
- [ ] Token storage is documented and defended.
- [ ] CSRF is addressed for the chosen auth model.

### RPC

- [ ] Every `Request` variant is classified.
- [ ] Out-of-scope variants are denied in server mode.
- [ ] `ListDevices`/`ListDir` are root-scoped.
- [ ] `ExecutePlan` is bound to the plan, root, and confirmation.
- [ ] Jobs cannot bypass confirmation.
- [ ] Recovery cannot replay a destructive plan.
- [ ] Read-only mode covers every variant.

### Network and secrets

- [ ] Every outbound host is allow-listed.
- [ ] Redirects and DNS rebinding are handled.
- [ ] Private/link-local/metadata addresses are blocked.
- [ ] Response size and decompression limits exist.
- [ ] `YAOPLM_OFFLINE=1` is effective.
- [ ] Secrets are not in git, logs, images, or browser storage.
- [ ] `gitleaks` is clean or findings are rotated.

### Dependencies and container

- [ ] `cargo audit`/`cargo deny` are clean or accepted.
- [ ] `pnpm audit` is clean or accepted.
- [ ] Actions are pinned and least-privilege.
- [ ] Base images are pinned and scanned.
- [ ] SBOM exists.
- [ ] Container is non-root, capability-limited, and read-only where possible.
- [ ] Runtime image contains no shell/curl unless justified.
- [ ] Build context contains no secrets or test data.

### Data safety

- [ ] Plan/confirm/execute is enforced for every write.
- [ ] Journal and backups are written before destructive steps.
- [ ] Crash recovery is tested.
- [ ] One writer per root is enforced.
- [ ] `ul.cfg`/`games.bin`/CFG/ART/VMC/CHT/APPS invariants hold.
- [ ] Failed operations leave OPL-readable state.

## 5. Known risks to verify first

These are starting points, not confirmed vulnerabilities.

1. Token-only auth, token in `localStorage`, token in URL/query for WebSocket/media; no session
   cookie, CSRF token, logout, or rotation.
2. No rate limiting on `/api/*`, including upload and token checks.
3. No CSP or common security headers.
4. The full inherited RPC surface is reachable with the token, including loader install/update,
   convert, VMC, Redump, art fetch, and loader import.
5. Upload staging inside the library root; no quota, no free-space preflight, no checksum, no
   stale cleanup; in-memory sessions.
6. No read-only mode; compose mounts the OPL root read-write.
7. Docker runtime includes `curl` and a shell; no read-only rootfs or custom seccomp.
8. No dependency audit, SBOM, or image scan yet.
9. No TLS in the server; relies on a reverse proxy.
10. Art/loader/Redump network paths may be SSRF-adjacent; `YAOPLM_OFFLINE` is not set in compose.
11. Parser fuzzing exists upstream but was not run against hostile uploads in this session.
12. Uncommitted Phase 0 changes mean the audit target must be frozen first.

## 6. Commands

Run from the repository root. Use a proper Rust/C toolchain or the Docker builder; this container
has no system `cc`.

```sh
git status --short
git rev-parse HEAD
git diff > /tmp/oplmdash-audit-target.patch

cargo metadata --no-deps --format-version 1 > /tmp/oplmdash-metadata.json
cargo tree > /tmp/oplmdash-tree.txt
cargo audit
cargo deny check
cargo geiger
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo fmt --all -- --check

pnpm install --frozen-lockfile
pnpm -C web lint
pnpm -C web typecheck
pnpm -C web test
pnpm audit

gitleaks detect --source . --redact
syft dir:. -o spdx-json > /tmp/oplmdash-sbom.spdx.json
trivy fs --scanners vuln,secret,misconfig .
hadolint packaging/docker/Dockerfile
```

Fuzzing (nightly):

```sh
cd fuzz
cargo +nightly fuzz run -O iso -- -max_total_time=60
cargo +nightly fuzz run -O system_cnf -- -max_total_time=60
cargo +nightly fuzz run -O cfg -- -max_total_time=60
cargo +nightly fuzz run -O ul_cfg -- -max_total_time=60
cargo +nightly fuzz run -O zso -- -max_total_time=60
cargo +nightly fuzz run -O cht -- -max_total_time=60
cargo +nightly fuzz run -O tar -- -max_total_time=60
cargo +nightly fuzz run -O ps2_elf -- -max_total_time=60
```

Container:

```sh
docker build -f packaging/docker/Dockerfile -t oplmdash:audit .
trivy image --scanners vuln,secret,misconfig oplmdash:audit
docker scout cves oplmdash:audit
docker run --rm --read-only --tmpfs /tmp -p 7878:7878 \
  -e YAOPLM_TOKEN=audit-only -v /tmp/opl-fixture:/opl:ro oplmdash:audit
```

## 7. Severity

- **Critical:** remote code execution, auth bypass, arbitrary write outside a root, secret
  exfiltration, unrecoverable data loss without confirmation.
- **High:** stored XSS, SSRF to internal/metadata services, path traversal, unauthenticated write,
  container escape, disk exhaustion that takes down the host.
- **Medium:** CSRF requiring user interaction, authenticated DoS, sensitive information leak,
  missing security headers with a plausible exploit.
- **Low:** hardening gaps, defense-in-depth, best-practice deviations.
- **Info:** observations and follow-ups.

Every finding must include: title, severity, affected version/commit, file:line, preconditions,
reproduction, impact, evidence, fix, regression test, and status.

## 8. Exit criteria

- No open Critical or High findings.
- Every Medium finding has an owner, a mitigation, and a target date.
- Out-of-scope RPC variants are denied in server mode, not only hidden in the UI.
- Security headers, rate limiting, upload quotas, and stale-upload cleanup are implemented or
  explicitly accepted with a documented risk.
- Read-only mode and session auth are either implemented or the deployment is restricted to a
  trusted LAN/VPN with a token and TLS.
- Dependency and image scans are clean or have accepted, documented risks.
- Docker runs non-root, capability-limited, and with a read-only root filesystem where possible.
- Full test suite, clippy, fmt, web tests, and new regression tests pass.
- The owner has reviewed and signed off the report.

## 9. Deliverables

- `docs/security/AUDIT_REPORT.md` — final report and go/no-go.
- `docs/security/FINDINGS.md` — one entry per finding with evidence and status.
- `docs/security/THREAT_MODEL.md` — assets, actors, boundaries, abuse cases.
- Patches on `security/audit-2026-10` with regression tests.
- SBOM files under `docs/security/sbom/` or CI artifacts.
- A short deployment checklist for the owner: TLS, token, VPN/LAN, read-only mount, backups.

## 10. Suggested schedule

| Day | Work |
|---|---|
| 0.5 | Freeze, inventory, threat model, SBOM |
| 1 | Auth, session, browser boundary |
| 1 | Paths, filesystem, TOCTOU, CIFS |
| 1.5 | Upload, parsers, fuzzing |
| 1 | HTTP headers, CSP, XSS, WebSocket |
| 1 | RPC surface and out-of-scope denial |
| 0.5 | Network egress, SSRF, secrets |
| 0.5 | Dependencies, supply chain, container |
| 1 | Data safety and crash consistency |
| 1 | Fixes, regression tests, report, sign-off |

Total: about 8–9 working days for one auditor, less if the owner accepts a narrower first pass.

## 11. New-session kickoff

Paste this into the new session:

> Read `CLAUDE.md`, `docs/oplmdash/INVESTIGATION_AND_PLAN.md`, `docs/SPIKE.md`, and
> `docs/oplmdash/SECURITY_AUDIT_PLAN.md`. Execute the plan on branch `security/audit-2026-10`.
> Freeze the target first, create `docs/security/`, and start with Phase 0. Do not test against
> real drives or the live library. Report findings as you go and stop before any destructive test
> or external scan.
