# oplmdash

A web-first manager for Open PS2 Loader (OPL) libraries, built to run on a NAS and be used from
any browser.

> **Status: Phase 0 (pre-alpha).** No release yet. This repository is a downstream fork of
> [YAOPLM](https://github.com/belomaxorka/yaoplm); see [FORK.md](FORK.md).

## Why oplmdash

OPL libraries usually live on a USB drive or a NAS share. Most managers are desktop apps, so moving
multi-GB images to the server or editing a share from a laptop is awkward. oplmdash keeps the
library on the device that OPL already reads and adds a small web UI:

- scan and inspect an existing OPL folder without touching it;
- upload `.iso` and existing `.zso` files from a browser, with resume;
- rename, move between `CD/` and `DVD/`, delete to trash, and edit per-game CFG through a
  plan/confirm flow;
- show art, compatibility settings, device health and job history;
- run as one non-root Docker container next to the share.

## Planned v0.1 scope

- Docker image and `compose.yaml` for a bind-mounted OPL root.
- Read-only scan and dashboard, mobile-friendly library and game details.
- Safe writes: rename, delete (trash), CD/DVD move, CFG edit; plan preview and audit log.
- Resumable browser upload for `.iso` and existing `.zso`.
- Session login, CSRF protection, rate limiting, read-only mode.
- OPL 1.1.0 and 1.2.0 beta profiles.

Not in v0.1: `ul.cfg` writing, ZSO conversion, formatted VMC creation, PS1/POPS, APPS, cheats,
art fetching, loader install/update. Large images on FAT32 are refused with a clear message until
UL support lands.

## Quick start (development)

Requirements: Rust (pinned in `rust-toolchain.toml`), Node.js 24+, pnpm via corepack, and a C
toolchain for Rust build scripts.

```sh
corepack enable pnpm
pnpm install
pnpm -C web build
cargo run -p yaoplm-server -- --root /path/to/opl --data-dir ./data
```

Open <http://127.0.0.1:7878>. The server listens on loopback without a token. For another address,
pass `--token` and open the UI once as `http://<host>:7878/?token=<token>`.

Uploads are opt-in while their audit findings are being closed: pass `--uploads` (or
`YAOPLM_UPLOADS=true`) to create the `/api/uploads/*` routes. Server mode also exposes a safe RPC
subset; `--rpc-allow-all` (or `YAOPLM_RPC_ALLOW_ALL=true`) enables the deferred v0.1 operations.

## Docker

The image is built from `packaging/docker/Dockerfile`:

```sh
cp .env.example .env
# edit .env: set a long YAOPLM_TOKEN and OPL_ROOT
docker compose up -d --build
```

The compose file mounts `OPL_ROOT` at `/opl` and a named volume at `/data`, runs as
`${PUID}:${PGID}` (default `1000:1000`), and exposes port `7878`. The container refuses to start
on a non-loopback bind without `YAOPLM_TOKEN`.

`OPL_ROOT` must be writable by `PUID`/`PGID`. The image creates `/data` for `1000:1000`; if you
use another UID/GID, create the `oplmdash-data` volume first and `chown` it to that user.
Uploads and the full RPC surface stay off unless `YAOPLM_UPLOADS` or `YAOPLM_RPC_ALLOW_ALL` is
set; both are commented out in `.env.example`.

## Security

The server is Phase 0 and is **not approved for public exposure**. The audit's Critical/High
findings (F-01, F-02, F-03, F-11) are fixed in code, but their regression tests and the full CI
matrix still need to run before sign-off. Until then, use it only as a single-owner tool on a
trusted LAN/VPN, behind TLS, with a long random token. See the [audit report](docs/security/AUDIT_REPORT.md)
and the [deployment checklist](docs/security/DEPLOYMENT_CHECKLIST.md).

Uploads are disabled unless `--uploads` (`YAOPLM_UPLOADS=true`) is given. Server mode answers a
safe RPC subset by default; `--rpc-allow-all` (`YAOPLM_RPC_ALLOW_ALL=true`) exposes the deferred
v0.1 operations. Prefer a read-only `/opl` mount where possible.

## Documentation

- [Investigation and plan](docs/oplmdash/INVESTIGATION_AND_PLAN.md) (EN)
- [oplmdash decisions](docs/oplmdash/DECISIONS.md) (EN)
- [Phase 0 spike](docs/SPIKE.md) (EN)
- [Security audit plan](docs/oplmdash/SECURITY_AUDIT_PLAN.md) (EN)
- [Security threat model](docs/security/THREAT_MODEL.md) (EN)
- [Security inventory](docs/security/INVENTORY.md) (EN)
- [Security findings](docs/security/FINDINGS.md) (EN)
- [Security audit report](docs/security/AUDIT_REPORT.md) (EN)
- [Deployment security checklist](docs/security/DEPLOYMENT_CHECKLIST.md) (EN)
- [Upstream specification](docs/SPEC.md) (EN)
- [Upstream architecture](docs/ARCHITECTURE.md) (EN)
- [Upstream roadmap](docs/ROADMAP.md) (EN)
- [Upstream decisions](docs/DECISIONS.md) (EN)
- [OPL format research](docs/research/02-opl-formats.md) (EN)

## Relationship to YAOPLM

oplmdash is a downstream fork of [belomaxorka/yaoplm](https://github.com/belomaxorka/yaoplm),
pinned at commit `0b545c4`. The Rust core, format handling, CLI and server are inherited.
oplmdash adds the NAS/Docker packaging, opt-in resumable uploads, a server-mode RPC allow-list and
a smaller web UI.
Changes are kept in separate commits so upstream fixes can be merged; see [FORK.md](FORK.md).

## Legal

oplmdash does not include or download games, BIOS files, POPS files or other proprietary
PlayStation software. Use dumps of discs you own. It does not call OPL Manager's private art
server. Not affiliated with Sony, the OPL project, OPL Manager or YAOPLM.

## License

[GPL-3.0-or-later](LICENSE). Upstream copyright and attribution are preserved in [NOTICE](NOTICE)
and [FORK.md](FORK.md).
