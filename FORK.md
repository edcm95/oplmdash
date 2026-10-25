# Fork notes

`oplmdash` is a downstream fork of [YAOPLM](https://github.com/belomaxorka/yaoplm), a
cross-platform OPL library manager.

- Upstream repository: <https://github.com/belomaxorka/yaoplm>
- Fork point: `0b545c4ff78e57342315ad65691e452c75192ef3` (`main`, 2026-10-25)
- License: GPL-3.0-or-later; `LICENSE` is unchanged.
- Upstream documentation remains authoritative for the core, formats and CLI.

## What this fork changes

The downstream work is kept in separate directories and commits so upstream fixes can be merged:

- `docs/oplmdash/`: investigation, plan and downstream ADRs.
- `packaging/docker/`: Docker image for the NAS local-mount deployment.
- `compose.yaml`, `.env.example`: deployment defaults.
- Server and web changes for resumable uploads, session auth, read-only mode and the simplified
  `oplmdash` UI are added on top of the upstream API.

The Rust core, format handling, CLI and existing server API are inherited from upstream. We do not
copy code from OPL Manager or from OPL itself; see `docs/oplmdash/DECISIONS.md` ADR-0005 and
ADR-0006.

## Syncing with upstream

```sh
git fetch upstream
git switch oplmdash/v0.1
git merge upstream/main
```

Keep downstream changes small and isolated. If a change belongs in the core or the generic server,
send it upstream instead of carrying a permanent patch.

## Attribution

YAOPLM and its contributors retain copyright over the inherited code. See `NOTICE` and `LICENSE`.
Open PS2 Loader is a separate project; this fork only reads and writes its on-disk formats.
