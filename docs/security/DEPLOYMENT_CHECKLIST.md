# oplmdash deployment checklist

Use this before exposing the server beyond a trusted LAN/VPN. It is deliberately short; the
evidence is in `FINDINGS.md`.

## Before first start

- [ ] Put the server behind a TLS reverse proxy; never publish port 7878 to the public internet.
      Enable HSTS at the proxy.
- [ ] Generate a long random token: `openssl rand -hex 32`. Put it in `.env` with mode `0600`.
      Do not commit it, paste it into a chat, or leave it in shell history. Rotate it if it may
      have leaked (F-06: there is no logout/rotation yet).
- [ ] Set `PUID`/`PGID` to a non-root user that owns the OPL folder. Do not use `0`.
- [ ] Keep the library backed up and test a restore. The server can delete user files; the plan
      confirmation is a guard, not a backup.
- [ ] Ensure no untrusted user or process can write to the library root and that it contains no
      symlinks you did not create. Until F-01 and F-11 are fixed, a symlink in the root is an
      escape primitive.
- [ ] Prefer a read-only `/opl` mount (`:ro`) and read-only mode; if the server must write,
      verify the UID/GID and mount propagation on the target NAS first.
- [ ] If v0.1 does not need art/loader/Redump network access, set `YAOPLM_OFFLINE=1` and/or
      block egress at the container/firewall. The compose file does not set it today.
- [ ] Do not enable the upload endpoint until F-01 and F-02 are fixed. Prefer the documented
      `/opl/incoming` drop-folder workflow if it is available in your build.

## Container

- [ ] Pin the image by digest, and pin the base images in the Dockerfile by digest.
- [ ] Run as a non-root UID/GID, `cap_drop: ALL`, `no-new-privileges:true` (compose already
      does the last two; keep them).
- [ ] Add `read_only: true` and a `tmpfs` for the writable temp dirs; add
      `seccomp`/AppArmor profiles and resource limits (`cpus`, `mem_limit`, `pids_limit`).
- [ ] Add `stop_grace_period` (and SIGTERM handling in the server, F-13) so a restart does not
      kill an in-flight write.
- [ ] Restrict the published port to the LAN/VPN interface or a localhost bind behind the proxy.
- [ ] Keep `/data` (cache, uploads, journal) on a separate, backed-up volume; size it.

## Operations

- [ ] Redact `?token=` in the reverse-proxy access log; do not log request bodies or
      Authorization headers. The token should never be in a log, screenshot or URL bookmark.
- [ ] Limit who holds the token. Today it is a single shared bearer credential with no
      per-user sessions, logout or rotation (F-06).
- [ ] Monitor disk free space. Uploads have no quota or preflight yet (F-02); an authenticated
      client can fill the volume.
- [ ] Re-run the audit checklist after fixing F-01/F-02/F-03/F-11, and run the full CI matrix
      (fmt, clippy, tests, web tests, fuzz, real-FS, audits, image scan).

## Acceptable residual posture (until the Critical/High findings are fixed)

Single owner, trusted LAN or VPN, TLS proxy, long random token, no untrusted writers/symlinks
in the root, read-only library mount where possible, uploads disabled or explicitly accepted.
This is **not** the plan's "private server" bar; see `AUDIT_REPORT.md` §2.
