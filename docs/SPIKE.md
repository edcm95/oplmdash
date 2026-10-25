# Phase 0 spike

Date: 2026-10-25. Status: **go**, with the open items below.

## What is in place

- Fork pinned at upstream `0b545c4`; `FORK.md` and `NOTICE` added, `LICENSE` unchanged.
- `README.md` describes `oplmdash`, the v0.1 scope and the fork relationship.
- `packaging/docker/Dockerfile`, `compose.yaml`, `.dockerignore` and `.env.example` added.
- Resumable upload prototype in `apps/server/src/upload.rs`:
  - `POST /api/uploads` creates a session;
  - `POST /api/uploads/{id}/chunk` appends with `Upload-Offset`;
  - `GET /api/uploads/{id}` returns the current offset;
  - `POST /api/uploads/{id}/finish` checks the declared size;
  - `DELETE /api/uploads/{id}` removes the staging directory.
- The staged path is accepted by `PlanImport` (covered by a test). Import with
  `transfer: "move"` consumes the staged file.
- The web UI is still the inherited YAOPLM SPA; no upload screen is wired yet.

## Verification

Environment: Debian 12 aarch64, Rust 1.99.1, Node 22.23.3, pnpm 12.15.0. The container has no
system `cc`/`gcc` and no `sudo`. For local verification only, Zig 0.13 was used as `CC`, `AR` and
the Rust linker. This workaround is not committed; the Docker builder uses the normal Rust image.

Passed:

- `pnpm install` via corepack;
- `pnpm -C web typecheck`;
- `pnpm -C web test` (96 tests);
- `pnpm -C web lint` (0 errors, 2 upstream TanStack Virtual warnings);
- `cargo check -p yaoplm-server -p yaoplm-cli`;
- `cargo test -p yaoplm-server` (9 upload/unit tests + 12 HTTP tests);
- `cargo clippy -p yaoplm-server --all-targets --all-features -- -D warnings`;
- `cargo fmt --all -- --check`.

Not verified:

- Docker image build and `docker compose up` — Docker is not installed in this environment.
- A real OPL library scan and a real upload from the SPA.
- Multi-arch build (`amd64`/`arm64`).

## Upload prototype notes

- Staging is `<root>/.oplmdash/uploads/<id>/<name>`. It stays inside the root so the existing
  `PathPolicy` allows `PlanImport` to read it; OPL ignores the hidden directory.
- Sessions live in memory. A restart loses the session but leaves the staged file; a later
  cleanup job must remove stale `.oplmdash/uploads/*` directories.
- Chunks are limited to 16 MiB by `DefaultBodyLimit`; the browser should use 8 MiB chunks.
- The endpoint is behind the existing token/origin middleware. It does not add session auth yet.
- Uploads are not quota-limited and do not preflight free space; `PlanImport` checks space later.
- A failed append truncates back to the previous offset before returning an error.

## Go/no-go

Go. The risky part — handing a staged file to the existing import planner without weakening the
root allow-list — works and is tested. The remaining Phase 0 work is packaging verification on a
Docker host and a decision on trimming the inherited SPA. Both are M1 work.

## Next

1. Build the image on a Docker host, run `docker compose up`, scan a fixture library and upload a
   small ISO.
2. Decide: trim the inherited SPA or build a smaller `oplmdash` SPA on the same API.
3. Add session auth and read-only mode before exposing writes on a LAN.
4. Wire the upload screen to the prototype endpoints and execute `PlanImport` with `move`.
5. Add stale-upload cleanup and quotas.
