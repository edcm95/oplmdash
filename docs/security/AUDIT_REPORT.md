# oplmdash security audit report

Date: 2026-10-25
Auditor: automated source audit (opencode)
Target: `oplmdash` downstream fork of YAOPLM
Frozen target: `0b545c4ff78e57342315ad65691e452c75192ef3` (branch `security/audit-2026-10`)
Snapshot: `/tmp/oplmdash-audit-target.tar.gz`; tracked diff `/tmp/oplmdash-audit-target.patch`
Companion files: `THREAT_MODEL.md`, `INVENTORY.md`, `FINDINGS.md`, `DEPLOYMENT_CHECKLIST.md`

## 1. Executive summary

The inherited core is the strong part of this fork. Paths in the core go through a strict
`RelPath`/`HostFs::resolve_checked` layer, plans are single-use and bound to their root, the
plan -> confirm -> execute flow requires `confirmDelete` for user data, archive handling is
read-only (no zip-slip extraction), the cache is keyed safely, and the CI matrix is unusually
thorough. Those are real, verified controls, not just documentation.

The server and packaging layer added by this fork is where the risk is. The audit found one
Critical and three High findings:

1. **F-01 (Critical):** the resumable-upload staging directory is built with
   `tokio::fs::create_dir_all` under the root without the core's checked-path layer. A symlink
   planted at `<root>/.oplmdash` redirects the write outside the root: arbitrary file write
   with attacker-controlled content.
2. **F-02 (High):** uploads have no quota, free-space preflight, target-filesystem (FAT32
   4 GiB) check, checksum or stale cleanup; they buffer each 16 MiB chunk, hold one global
   mutex across I/O and run outside the single-writer queue. A token holder can fill the disk,
   block uploads or race a plan.
3. **F-03 (High):** every inherited `Request` is reachable in server mode. The v0.1 scope
   (scan/library/details/art display-set-delete/rename/delete/move/CFG/ISO-ZSO import/jobs/
   recovery) is not enforced at the API layer; convert, VMC, art fetch, loader
   download/install/update/rollback, Redump and loader import are all callable with the token.
4. **F-11 (High):** `ScanLibrary` uses `std::fs` (`fsutil::Listing::read`) with
   `entry.metadata()`, which follows symlinks and does not skip them; a symlinked `ul.cfg`,
   `CD`, `DVD`, `ART` or `CFG` lets the scanner read outside the root.

There are also Medium findings for missing rate limiting/timeouts, missing CSP/frame
protection, token-in-URL/`localStorage` with no session/logout/rotation, host-path leakage in
errors, Docker hardening/supply chain, no read-only mode, a media TOCTOU and unbounded read,
SIGTERM handling, and an on-device source integrity gap. The details, repros, fixes and
regression-test suggestions are in `FINDINGS.md`.

## 2. Go/no-go

**NO-GO for the plan's "private server" exit criteria** until F-01, F-02, F-03 and F-11 are
fixed and *tested* (or explicitly accepted by the owner with a mitigation and a target date).
The four are fixed in code as of this revision, but the full test suite could not run in this
container (no `cc`); the plan's exit criteria require no open Critical/High findings and a
passing matrix, so the verdict stays NO-GO until CI verifies the fixes.

**Conditional use is possible today only as a single-owner, trusted-LAN/VPN tool** with all of
the following, and with the owner's explicit acceptance of the residual risk:

- a long random `YAOPLM_TOKEN`, TLS at a reverse proxy, no public exposure;
- no untrusted user or process can write to (or create symlinks in) the library root;
- uploads are disabled unless needed (and F-01 is fixed before enabling them);
- `/opl` is mounted read-only or the owner accepts F-10;
- the deployment checklist in `DEPLOYMENT_CHECKLIST.md` is followed;
- the owner does not rely on the server to contain a compromised token.

This is a narrower posture than the plan's target. The product decision to expose this to a
LAN with a shared token and resumable uploads is not safe until the four High/Critical items
are closed.

## 3. What was verified as good

- **Core write safety (V-01):** `RelPath` rejects empty/`.`/`..`/separators; `HostFs`
  canonicalizes and enforces `starts_with(root)`; `create` uses `create_new(true)`;
  `rename`/`remove` use the checked path; the final-component symlink is not followed for
  rename/remove.
- **Plan confirmation (V-02):** single-use plan store, 1 h TTL, max 32; `confirmDelete`
  required for user-data deletion; plan bound to its stored root.
- **Archive handling (V-03):** archives are read as folders, not extracted; `archive::list`
  drops drives and `..`; the importer writes via `RelPath`/`DeviceFs`. Zip-slip into the device
  is not applicable.
- **Cache (V-04):** `safe_key` is a strict allow-list.
- **Listings and devices (V-05/V-06):** `ListDir` is root-scoped, skips hidden entries and
  symlinks, caps at 5000; server `ListDevices` shows only configured roots.
- **Upload offset arithmetic (V-07):** offset/size/duplicate/finish/cancel logic is sound
  (the resource/abuse side is F-02).
- **Auth basics (V-08):** non-loopback bind without a token refuses to start; empty token is
  no token; constant-time comparison; loopback Host check; Origin == Host.
- **CSRF basics (V-09):** JSON/octet-stream content types and no CORS headers block simple
  cross-site forms.
- **Web XSS basics (V-10):** no `dangerouslySetInnerHTML`/`eval`/`innerHTML` in production;
  React escaping; `ExternalLink` uses `rel="noreferrer"`.
- **CI (V-11):** `YAOPLM_OFFLINE=1`, fmt/clippy/tests, core purity, TS bindings, web
  lint/typecheck/test/build, e2e, FAT32/exFAT real-FS, fuzz build, OPL cross-checks.
- **Atomic writes and one-writer queue (V-13):** `.yaoplm-part` + sync + rename + `sync_dir`;
  preflight cleans temp files, checks the journal and the device fingerprint; `Queue::Write`
  serializes writes.

## 4. Method and limitations

Method: read the plan and fork docs; freeze the target (branch + tarball + patch); inventory
routes, `Request` variants, writes and network; read the server, API, device, core, net and web
surfaces; build the threat model; targeted static checks; verify the core write layer and the
plan/queue logic.

Limitations of this run (the audit plan anticipated the toolchain gap):

- the container has no `cc`, so `cargo test`, `cargo clippy`, `cargo fmt --check`,
  `cargo audit` and `cargo deny` could not run;
- no Docker, `trivy`, `docker scout`, `hadolint`, `syft` or `gitleaks`;
- `pnpm` is not installed (corepack is present; enabling it needs a network fetch);
- fuzzing (nightly) and the `--ignored` real-data suites were not run;
- fault injection (kill during copy/rename/delete/CFG/`ul.cfg`/`games.bin`, disk full,
  permission denied, CIFS rename failure, power loss) was not run;
- no dynamic exploit was executed; F-01/F-11 are code-level proofs with a precise repro.

These gaps do not change the Critical/High findings (they are visible in the source), but they
do mean the plan's exit criterion "full test suite ... pass" is **not yet verified** for this
target. Run the full matrix in the Docker builder/CI before sign-off.

## 5. Recommended fix order

1. **F-01** (Critical): *done in code.* Staging now refuses a symlinked `.oplmdash`/`uploads`,
   canonicalises the created directory inside the root, and uploads are opt-in via `--uploads`.
   The regression test still needs to run in CI.
2. **F-11** (High): *done in code.* `fsutil::Listing::read` now uses `entry.file_type()` and
   skips symlinks. The symlinked `ul.cfg`/`CD` regression test still needs to run in CI.
3. **F-03** (High): *done in code.* `rpc::handle` parses each request and enforces
   `allowed_in_server_mode` unless `--rpc-allow-all` is set. The table-driven `Request` test
   still needs to run in CI.
4. **F-02** (High): *done in code.* Uploads enforce size, FAT32 4 GiB and free-space limits;
   sessions use per-upload locks; stale staging is swept on startup and hourly; completed
   uploads return SHA-256. Streaming and writer-queue integration remain.
5. **F-05/F-06** (Medium): CSP/frame protection and session auth (or the documented trusted-LAN
   restriction), WS/media tickets, logout/rotation.
6. **F-04/F-07/F-12/F-13/F-14** (Medium): rate limits/timeouts, error redaction, media open
   hardening/size cap, SIGTERM/grace period, source integrity.
7. **F-08/F-09** (Medium): Docker hardening and supply chain (digests, no curl/shell,
   read-only rootfs/limits, audits, SBOM, image scan, SHA-pinned Actions).
8. **F-15..F-24** (Low/Info): hardening and cleanup.

## 6. Deliverables

- `docs/security/THREAT_MODEL.md` — assets, actors, boundaries, abuse cases.
- `docs/security/INVENTORY.md` — routes, `Request` matrix, writes, network.
- `docs/security/FINDINGS.md` — every finding with evidence, repro, fix, test, status.
- `docs/security/AUDIT_REPORT.md` — this report and the go/no-go.
- `docs/security/DEPLOYMENT_CHECKLIST.md` — what the owner must do before exposing the server.
- Target freeze: branch `security/audit-2026-10`, snapshot
  `/tmp/oplmdash-audit-target.tar.gz`, tracked diff `/tmp/oplmdash-audit-target.patch`,
  `git rev-parse HEAD` = `0b545c4ff78e57342315ad65691e452c75192ef3`.

F-01, F-02, F-03 and F-11 are fixed in code on this branch; the regression tests named in
`FINDINGS.md` still need to run in CI. The Medium/Low findings remain open. Commit the fixes
separately and run the full matrix before sign-off.

## 7. Sign-off

Owner: ____________________  Date: __________

- [x] F-01 fixed in code; regression test still pending
- [x] F-02 fixed in code; regression test still pending
- [x] F-03 fixed in code; regression test still pending
- [x] F-11 fixed in code; regression test still pending
- [ ] Medium findings have an owner, a mitigation and a target date
- [ ] Full test matrix, clippy, fmt, web tests and the new regression tests pass
- [ ] Deployment checklist reviewed
- [ ] Go for private-server use / stay restricted to the trusted LAN/VPN posture
