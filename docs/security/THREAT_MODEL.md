# oplmdash threat model

Status: draft for owner sign-off
Date: 2026-10-25
Target: `oplmdash` downstream fork of YAOPLM
Frozen target: `0b545c4ff78e57342315ad65691e452c75192ef3` (branch `security/audit-2026-10`)
Snapshot: `/tmp/oplmdash-audit-target.tar.gz`; tracked diff `/tmp/oplmdash-audit-target.patch`

This model follows `docs/oplmdash/SECURITY_AUDIT_PLAN.md` §1. It is the basis for the
findings in `FINDINGS.md` and the go/no-go in `AUDIT_REPORT.md`.

## 1. Scope

In scope:

- `apps/server` (HTTP, auth, upload, media, WebSocket, SPA serving, Docker runtime);
- `apps/cli` commands that write, delete, move or fetch;
- `crates/yaoplm-api` (every `Request`, the plan/execute boundary, jobs, path policy);
- `crates/yaoplm-core`, `yaoplm-formats`, `yaoplm-device`, `yaoplm-hash`, `yaoplm-net`,
  `yaoplm-i18n` where they touch parsers, filesystem I/O, network, caches;
- `web` (React SPA, token handling, API client, XSS/CSRF surfaces);
- `packaging/docker/`, `compose.yaml`, `.dockerignore`, `.env.example`;
- CI workflows, lockfiles, release configuration;
- data safety: plan -> confirm -> execute, journal, backups, atomic writes, recovery;
- OPL format readers/writers (a bad write can destroy a library).

Out of scope:

- OPL firmware, the PS2, the NAS OS, the reverse proxy configuration (unless provided);
- full source review of third-party crates/npm packages (advisory/SBOM tools instead);
- the Tauri desktop app except where it shares the server's security boundary;
- the owner's home network, Wi-Fi, DNS and physical security.

## 2. Deployment assumptions (to be confirmed by the owner)

1. The server is private: reachable from a LAN or VPN, behind a TLS reverse proxy, never
   exposed directly to the public internet.
2. `YAOPLM_TOKEN` is a long random value; it is not in source control, screenshots or logs.
3. The OPL folder is a bind mount; the container runs as a non-root UID/GID.
4. Art fetching, loader download/install and Redump update are either disabled or limited to
   the documented hosts.
5. The owner keeps backups and can restore them.
6. The library root is writable only by the owner and the server's UID; no untrusted user or
   process can create files or symlinks in it.

If any assumption is false, raise the severity of the related findings. In particular,
findings F-01 and F-11 assume an attacker can create a symlink in the library root; that is
false on a read-only mount, true on a shared writable Linux folder.

## 3. Assets

- The OPL library and every user file under the configured roots.
- The token, Screenscraper credentials, signing keys and `.env` contents.
- The host filesystem outside the roots (through path or symlink bugs).
- Availability of the server, the NAS and the library.
- Integrity of `ul.cfg`, `games.bin`, CFG, ART, VMC, CHT and APPS data.
- The user's browser session and anything in `localStorage`.

## 4. Threat actors

- A LAN attacker without the token.
- A malicious website opened in the owner's browser (CSRF, DNS rebinding, XSS, token theft).
- A user with the token but no shell access (abuse of inherited write/RPC endpoints).
- A malicious or corrupted game image, archive, CFG, CHT, VMC or ZSO.
- A compromised dependency, base image, GitHub Action or release artifact.
- A local unprivileged process on the NAS (can race, plant symlinks, read logs).
- An attacker who obtains a leaked token from a URL, log, browser storage or screenshot.

## 5. Trust boundaries

1. Browser <-> server HTTP/WebSocket.
2. Server <-> library roots on the host filesystem.
3. Server <-> internet (art, loaders, Redump, Screenscraper).
4. Container <-> host (mounts, UID/GID, capabilities).
5. Build/release pipeline <-> runtime image.

## 6. Abuse cases and controls

| # | Abuse case | Relevant control | Status |
|---|---|---|---|
| A1 | No token: read/write the library | token required on `/api` and `/media`; non-loopback bind refuses to start without a token | present |
| A2 | Steal the token from `localStorage` via XSS | CSP, frame protection, no HTML sinks in the SPA | CSP missing (F-05); no sinks found |
| A3 | Steal the token from a URL/log/referrer | query token limited to WS/media; token removed from the address bar | token still in proxy logs/media/WS URLs (F-06) |
| A4 | Clickjack the authenticated UI into deleting | `frame-ancestors`/`X-Frame-Options` | missing (F-05) |
| A5 | Brute-force/DoS the token or API | rate limiting, lockout, timeouts | missing (F-04) |
| A6 | Write outside a root | `RelPath` + `HostFs::resolve_checked` for core writes | present for core; upload staging bypasses it (F-01); scanner follows symlinks (F-11) |
| A7 | Read outside a root | `PathPolicy` for API reads | present for API/media; scanner follows symlinks (F-11); media TOCTOU (F-12) |
| A8 | Zip-slip through an uploaded archive | archives are read as folders, not extracted; `archive::list` drops `..` | mitigated (V-03) |
| A9 | Fill the disk with uploads | quota, free-space preflight, target-FS limit | missing (F-02) |
| A10 | Run an inherited destructive RPC with the token | server-mode allow-list | missing; all variants reachable (F-03) |
| A11 | Replay/execute a plan without confirmation | single-use plan store, `confirmDelete`, root binding | present (V-02) |
| A12 | SSRF from art/loader/Redump URLs | fixed hosts, redirects limited, offline mode | fixed hosts; offline redirect check is incomplete (F-03/F-09 note); no arbitrary URL input found |
| A13 | Secret in the repo/image/log | `.env` ignored, `.dockerignore`, no keys in repo | clean by grep; scanners not run (F-09) |
| A14 | Compromised dependency/action/base image | pinning, audits, SBOM | missing (F-09) |
| A15 | Container escape / over-privilege | non-root, `cap_drop: ALL`, `no-new-privileges`, read-only rootfs | partial: no read-only rootfs/seccomp/limits; shell/curl present (F-08) |
| A16 | Crash during a write leaves a broken game | atomic temp+rename, journal, recovery | present (V-13); not fault-injected here |
| A17 | Delete shared CFG/ART/VMC still used by another game | `related` checks other users | present by design; not re-tested here |
| A18 | Corrupt/malicious image/CFG/CHT/VMC/ZSO | parsers, fuzz targets, size limits | fuzz targets exist; not run here |

## 7. Sign-off

Owner: ____________________  Date: __________

Open Critical/High findings must be fixed or explicitly accepted (with a mitigation and a
target date) before the server is exposed beyond a trusted LAN/VPN with a token and TLS.
