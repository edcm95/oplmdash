# oplmdash - Decision log (ADRs)

> Downstream `oplmdash` architecture decision records. Upstream yaoplm ADRs are in `../DECISIONS.md`;
> the numbers here are scoped to `oplmdash` and start at 0001.
> See `INVESTIGATION_AND_PLAN.md` for the full research and plan.

## ADR-0001 - Build on yaoplm as a GPL-3.0-or-later downstream fork

- **Status:** accepted, 2026-10-25.
- **Context:** OPL format handling and data-safe writes are the hard, risky parts. yaoplm already
  implements them in Rust with tests, fuzzing, plan/confirm/execute, journaling and backups. It also
  has an axum server and a React SPA, which matches the Docker + browser requirement.
- **Decision:** fork `belomaxorka/yaoplm` and keep the combined distribution under
  GPL-3.0-or-later. Keep `oplmdash` changes in clearly separated commits/directories so upstream
  fixes can be merged.
- **Consequences:**
  - We must ship source, keep the license and notices, and preserve upstream attribution.
  - We inherit a pre-alpha codebase and its roadmap; pin a commit and test before updating.
  - We must not copy code from non-GPL projects into the fork.
- **Alternatives rejected:** thin wrapper around the upstream server (too little control, no upload
  endpoint, two processes to secure); clean-room rewrite (months of work and high data-loss risk).

## ADR-0002 - Resumable browser upload in v0.1

- **Status:** accepted, 2026-10-25.
- **Context:** the app runs on a NAS; the user's ISOs may be on a laptop, not on the NAS. Asking the
  user to copy multi-GB files over SMB first defeats the purpose of the dashboard.
- **Decision:** implement chunked, resumable upload to a staging folder on the data volume, with
  optional SHA-256, then hand the staged path to the existing import plan.
- **Consequences:**
  - Need upload sessions, offsets, quotas, cleanup and proxy timeout documentation.
  - No format conversion during upload: `.iso` and existing `.zso` are stored as-is.
  - Files that do not fit the target filesystem (e.g. >4 GiB on FAT32) are refused in v0.1.

## ADR-0003 - Primary deployment: NAS Docker with a local mount

- **Status:** accepted, 2026-10-25.
- **Context:** the common OPL network setup is a NAS share read by the PS2 over SMBv1. The NAS can
  run Docker and mount the same share locally, avoiding a second SMB client.
- **Decision:** target Docker on a NAS with the OPL folder bind-mounted at `/opl`. Document the PC
  USB + Samba sidecar model as a later option, not part of v0.1.
- **Consequences:**
  - Compose examples and docs focus on bind mounts, UID/GID and CIFS semantics.
  - The app does not implement SMB1 or a Samba server in v0.1.
  - Direct SMB2/3 access and the Samba sidecar are deferred to v0.2.

## ADR-0004 - MVP scope: dashboard + safe library operations

- **Status:** accepted, 2026-10-25.
- **Context:** the first release should be useful without taking on every OPL format. UL splitting,
  ZSO conversion, VMC creation and PS1/APPS have the highest edge-case and data-loss risk.
- **Decision:** v0.1 covers scan, library/dashboard, game details, art display/manual set/delete,
  rename, delete (trash), move CD<->DVD, CFG editing, safe import of `.iso`/`.zso`, auth and audit.
- **Consequences:**
  - No `ul.cfg` writing, ZSO conversion, formatted VMC creation, PS1/POPS or APPS install in v0.1.
  - Large games on FAT32 are rejected with a clear message until UL support lands.
  - v0.2 adds UL, ZSO, VMC, art fetching, PS1/APPS and cheats.

## ADR-0005 - Provenance, licensing and data safety

- **Status:** accepted, 2026-10-25.
- **Decision:**
  - Keep `LICENSE` (GPL-3.0-or-later), add `FORK.md` and `NOTICE` with upstream provenance.
  - Do not copy OPL Manager (closed) or OPL C code (AFL-3.0).
  - Do not bundle cover art; fetch only from reviewed sources with attribution.
  - Every write goes through plan -> confirm -> execute; use atomic temp files, journal, backups and
    an optional trash instead of immediate deletion.
  - Never write outside the configured roots; reject `..`, symlink escapes and untrusted names.
- **Consequences:** slower feature development, but predictable and recoverable behaviour around
  irreplaceable user data.

## ADR-0006 - OPL source is the specification; keep compatibility oracle tests

- **Status:** accepted, 2026-10-25.
- **Context:** `oplmdash` writes into a folder that a PS2 running OPL must still read. A
  plausible-looking file can make a game disappear, fail to boot, or corrupt settings. Third-party
  managers disagree on edge cases, and some have real format bugs.
- **Decision:**
  - Treat `ps2homebrew/Open-PS2-Loader` (reviewed at `b3a2199`) as the behavioural specification,
    not other managers.
  - Keep and expand yaoplm's oracle tests that compile/run OPL's own C functions from a pinned
    checkout (`crates/yaoplm-formats/tests/opl_oracles.rs`, `vmc_oracles.rs`) in CI.
  - Enforce the guardrails in `INVESTIGATION_AND_PLAN.md` Appendix A.3: exact `ul.cfg` records,
    lowercase chunk part hex, 1 GiB chunks, <= 10 SMB parts, ASCII names, no stale `games.bin`,
    valid CFG limits, PNG-only art, formatted VMC, raw CHT codes, valid `title.cfg`, and safe
    rename/delete/media changes.
  - Count file-name lengths in bytes, not Unicode characters. Enforce ASCII-only names on write.
- **Consequences:**
  - More tests and CI time (C compiler + Python `lz4` + pinned OPL checkout).
  - Some upstream yaoplm code may need fixes (for example byte semantics in `iso_name`).
  - We can justify format decisions with a source reference, which matters for user-data safety.

## ADR-0007 - Upload staging inside the library root (Phase 0 prototype)

- **Status:** accepted for Phase 0, 2026-10-25; revisit in M3.
- **Context:** the server's `PathPolicy` only allows paths inside `--root`. Staging on the app data
  volume would need a second read allow-list just for uploads, and the import planner would have to
  treat it as a trusted source.
- **Decision:** stage uploads in `<root>/.oplmdash/uploads/<id>/<name>` for the prototype. OPL does
  not scan that hidden folder, and `PlanImport` already accepts it through the normal root policy.
  Import with `transfer: "move"` consumes the staged file after a successful plan.
- **Consequences:**
  - Uploads use space on the library volume; quotas and free-space preflight are still missing.
  - A server restart loses in-memory sessions but leaves staged files; stale directories need a
    cleanup job.
  - Read-only roots cannot accept uploads, which is the intended behaviour.
  - M3 may move staging to the data volume and add an explicit source allow-list; that is a new ADR.

## Pending decisions

- Branding, repository owner and image registry.
- Password + session auth vs token-only; viewer role in v0.1.
- Trash retention policy and upload quotas.
- Whether to expose inherited art fetch, loader install/update and Redump verification.
- OPL 1.1.0 vs 1.2.0 beta profile detection and enforcement (title 64/160, SMB 87/159,
  `games.bin` 92/188, exFAT/ZSO, contiguous files vs 64 fragments).
