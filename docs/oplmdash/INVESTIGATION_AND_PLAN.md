# oplmdash - Investigation and Plan

> Date: 2026-10-25
> Phase: investigation / planning. No production code written yet.
> This is a downstream `oplmdash` document. Upstream yaoplm ADRs live in `docs/DECISIONS.md`.
> Decisions for this fork are mirrored in `docs/oplmdash/DECISIONS.md` (ADR numbers start at 0001).
> Goal: a Docker-deployed web application that lets a user on another computer manage an
> Open PS2 Loader (OPL) library (USB folder or SMB share).

---

## Decisions locked (2026-10-25)

| # | Decision | Choice | Consequence |
|---|---|---|---|
| D1 | Base and license | Fork `yaoplm`, GPL-3.0-or-later | Inherit the Rust core/server; the fork and distribution stay GPL with attribution. |
| D2 | Getting ISOs in | Browser upload in v0.1 | Add resumable upload, staging and checksum to the server. |
| D3 | Deployment | NAS Docker with a local mount | Primary compose target; no Samba sidecar in v0.1. |
| D4 | MVP scope | Dashboard + safe library ops | Scan, details, art, rename/delete/move, CFG. No UL writing, ZSO conversion, VMC creation, or PS1/APPS in v0.1. |

Implication of D2 + D4: v0.1 uploads `.iso` and existing `.zso` as-is. If the target filesystem
cannot hold the file (for example a game >4 GiB on FAT32), the upload is refused with a clear
explanation; `ul` splitting is deferred to v0.2. No format conversion happens during upload.

---

## 0. Executive summary

### What we are building

`oplmdash` is a web-first OPL library manager:

- runs in Docker on a home server/NAS (primary) and later on a PC that owns the OPL folder;
- serves a browser UI to any machine on the LAN;
- reads and safely writes the OPL directory (`CD/`, `DVD/`, `ul.cfg`, `ART/`, `CFG/`, `VMC/`, `CHT/`, `APPS/`);
- never formats drives, never downloads games, never bundles copyrighted art;
- is safe around multi-GB files and FAT32/exFAT quirks.

### Key finding

One existing open-source project already implements almost exactly this:

**[belomaxorka/yaoplm](https://github.com/belomaxorka/yaoplm)** - GPL-3.0-or-later, Rust.

- One Rust core, three front ends: Tauri desktop, CLI, and **`yaoplm-server`** (axum + React SPA).
- The server is explicitly designed for "Docker on a NAS / home server" and already has:
  token auth, root allow-list, JSON-RPC over HTTP, WebSocket job events, SPA embedding,
  path traversal protection, plan -> confirm -> execute, atomic writes, journal/recovery,
  byte-exact OPL formats, fuzzing, and ~113 Rust test files.
- It is **pre-alpha**, upstream docs were mostly Russian (translated in this fork), there is **no Dockerfile yet** (roadmap item SRV-1 is open),
  it has **no browser upload** (imports only from paths already inside the server roots), and auth is a
  single shared token.

### Recommendation (chosen)

**Build on top of yaoplm as a downstream fork/distribution under GPL-3.0-or-later** (D1).

Rationale: the hard, dangerous part of this product is byte-exact OPL format handling and data-safe
writes, not the web UI. yaoplm already solves that part and has tests and real-format research behind it.
Reimplementing it would take months and put users' libraries at risk.

`oplmdash` should differentiate in the web/dashboard/deployment layer:

1. official Docker image + compose file (multi-arch, non-root, healthcheck);
2. simpler, mobile-friendly dashboard instead of the full desktop UI;
3. resumable browser upload for users whose ISOs are not already on the server;
4. proper session auth (single admin first, users/roles later);
5. read-only mode, audit log, and safer defaults;
6. documentation for NAS bind mounts and the SMBv1 reality of OPL.

The clean-room option is documented in section 4.4 as a fallback only; it is not the chosen path.

### Non-negotiable decisions

- Do **not** copy code from OPL Manager (closed source).
- Do **not** copy code from GPL projects into a non-GPL codebase.
- Because we fork yaoplm, the fork stays **GPL-3.0-or-later** with upstream attribution.
- Do **not** copy OPL's own C code (AFL-3.0); reimplement from observed behaviour and format docs.
- Do **not** bundle cover art; fetch only on demand, with attribution and source licensing review.
- Every write goes through plan -> confirm -> execute, with backups and a journal.

### OPL source review findings (2026-10-25)

Reviewed against OPL master `b3a2199` and the `v1.1.0` tag. Details, source references, corrections
and the full guardrail checklist are in Appendix A.

1. **Byte lengths, not Unicode characters.** OPL uses `strlen` and byte offsets; yaoplm's
   `iso_name` uses `chars()`. Enforce ASCII and byte semantics to avoid misclassifying names.
2. **OPL 1.1.0 differs from 1.2.0 beta.** 1.1.0: title 64 bytes, SMB filename 87 bytes,
   `games.bin` 92 bytes, PNG/JPG/BMP art, no exFAT/ZSO, each file contiguous. 1.2.0 beta:
   title 160, SMB 159, `games.bin` 188, PNG only, exFAT/ZSO, <= 64 fragments per game.
3. **`ul.cfg` live reload.** OPL notices changes by file size; tell users to refresh or restart
   OPL after a same-size edit (for example a title-only rename).
4. **VMC validation is narrower than "formatted".** OPL checks magic, card type and size only;
   we should still write fully formatted cards so the console does not have to.
5. **Config writes.** OPL flattens prefixes and drops comments/`#` keys; preserve the original
   bytes when a file is untouched.
6. **Path limits.** OPL uses fixed 256/128/64/32-byte buffers, and `$AltStartup` must fit
   `cdrom0:\<name>;1` in 32 bytes (21-char filename).
7. **Guardrails.** Appendix A.3 lists every action that would break OPL, from partial `ul.cfg`
   records and uppercase chunk part hex to stale `games.bin` and deleting a shared VMC.

---

## 1. Context and constraints

### 1.1 The user's environment

A typical OPL user has one of these:

| Setup | Where the OPL folder lives | How OPL reads it |
|---|---|---|
| USB stick / USB HDD | FAT32 or exFAT volume | BDM `usb` (`massN:`) |
| SD card in MX4SIO | FAT32/exFAT | BDM `sdc` (`massN:`) |
| NAS / home server | SMB share | SMBv1 (`smb0:`), only dialect `NT LM 0.12` |
| Internal PS2 HDD | APA/PFS partition | HDD mode (HDL format) |

For the first version we target the **folder-based** cases: USB, MX4SIO, and SMB shares that are
mounted into the container. Internal PS2 HDD (APA/PFS) is explicitly out of scope for v1.

### 1.2 Deployment models to support

1. **NAS Docker (recommended):** the OPL share is already local to the NAS; mount it into the
   container (`/opl`). OPL on the PS2 reads the same share over SMBv1. The app only touches the
   mounted filesystem.
2. **PC with USB drive:** Docker Desktop / Docker Engine mounts the USB volume. If the PS2 must read
   it over the network, a Samba sidecar (SMBv1) can export the same folder. This is a security
   trade-off and must be opt-in with a warning.
3. **Remote SMB without a host mount (later):** a native SMB2/3 client inside the app, independent
   of OPL's SMBv1 requirement. This adds credential management and is not needed for MVP.

### 1.3 Hard constraints

- OPL reads **SMBv1 only**. We should not pretend otherwise. A Samba sidecar is possible, but SMBv1
  is obsolete and should be isolated (separate share/VLAN, no guest access, strong password).
- OPL uses OEM code page 869 for names; **write ASCII-only file names**.
- FAT32 cannot hold files >= 4 GiB; large games need the `ul` split format.
- exFAT and ZSO require OPL 1.2.0 beta, not 1.1.0.
- A PS2 game library is irreplaceable user data. Data loss is the primary risk.

---

## 2. What "manage an OPL directory" actually means

This section is the domain reference for implementation. Facts are taken from the OPL source
(`ps2homebrew/Open-PS2-Loader`, `master`, October 2026) via the consolidated research in
`yaoplm/docs/research/02-opl-formats.md`, plus the official OPL Manager docs for art sizes.

### 2.1 Folder layout

At the root of the device (or under `usb_prefix` / `eth_prefix`):

```
<root>/
  CD/                 CD-media images: *.iso, *.zso
  DVD/                DVD-media images: *.iso, *.zso
  CD/games.bin        OPL scan cache (delete after changing CD/)
  DVD/games.bin       OPL scan cache (delete after changing DVD/)
  ul.cfg              USBExtreme/USBAdvance game table
  ul.<CRC>.<ID>.<nn>  1 GiB chunks (in the root, not in CD/DVD)
  CFG/<ID>.cfg        per-game settings
  CHT/<ID>.cht        cheats
  VMC/<name>.bin      virtual memory cards
  ART/<ID>_<SUFFIX>.png
  THM/thm_<Name>/     themes
  LNG/lang_<Name>.lng translations
  APPS/<folder>/      homebrew apps: title.cfg + ELF
  POPS/               POPStarter PS1 runtime and <Name>.VCD (not read by OPL itself)
  conf_opl.cfg        global OPL settings (may also live on the memory card)
  conf_network.cfg
  conf_game.cfg
  conf_apps.cfg
```

OPL's `sbCreateFolders()` creates `CFG THM LNG ART VMC CHT APPS`, plus `CD DVD` on image devices.

### 2.2 Game discovery and game IDs

OPL scans `CD/`, then `DVD/`, then `ul.cfg`.

Accepted image extensions: `.iso` and `.zso` only (case-insensitive on the last 4 bytes).
No `.cso`, `.bin`, `.img`, `.chd`.

**Old-style name:** `SLUS_200.62.Game Title.iso`

- total length >= 17 bytes;
- bytes at index 4 = `_`, 8 = `.`, 11 = `.`;
- ID = first 11 bytes;
- title = text between index 12 and the extension;
- title <= 160 bytes (OPL uses `strlen`, not character count);
- for SMB, the whole file name must be **<= 159 bytes** (the in-game driver truncates at 160).

**New-style name:** `Game Title.iso` - OPL reads the ID from inside the image.

**ID extraction (must match OPL exactly):**

- read at most 1023 bytes of `SYSTEM.CNF`;
- find `BOOT2` **case-sensitively**, optional whitespace around `=`;
- value is the first run of printable ASCII; whitespace ends it;
- skip everything up to and including the first `:` (e.g. `cdrom0:`);
- skip leading `\`;
- copy until the first `;` or end;
- truncate to **11 bytes**.

Example: `BOOT2 = cdrom0:\SLUS_200.62;1` -> `SLUS_200.62`.

An image with no `BOOT2` **silently disappears** from OPL. The manager must warn about it.

**CD vs DVD** comes only from the folder (`CD/` -> media 0x12, `DVD/` -> media 0x14). It is not
cosmetic: OPL reports the type to the game and throttles reads differently. A size heuristic is only
a UI suggestion; the user can override.

**Caches:** `CD/games.bin` and `DVD/games.bin` are raw arrays of 188-byte records. They are keyed by
file name and can go stale if an image is replaced under the same name. **Delete them after any change
to the folder.**

### 2.3 ISO and ZSO

- ISO9660 image, optionally ZSO-compressed.
- ZSO header: 24 bytes little-endian: magic `ZISO`, header size 0x18, u64 uncompressed size,
  u32 block size 2048, u8 version 1, u8 align, 2 reserved bytes.
- Index: `(total_bytes / 2048) + 1` u32 entries; bit 31 = block stored plain; bits 0-30 = file offset >> align.
- Blocks are **raw LZ4 blocks** (no frame), each decoding to 2048 bytes.
- ZSO exists only in OPL 1.2.0 betas; v1.1.0 cannot read it.
- Write `.zso` with a lowercase extension.

### 2.4 `ul.cfg` and chunk files

Used for games larger than 4 GiB on FAT32, and for USBExtreme/USBAdvance compatibility.

`ul.cfg` is a concatenation of **64-byte packed records**:

| Offset | Size | Field | Value |
|---|---|---|---|
| 0x00 | 32 | name | title, ASCII, not necessarily NUL-terminated |
| 0x20 | 3 | magic | `ul.` |
| 0x23 | 12 | startup | game ID, e.g. `SLUS_200.62` |
| 0x2F | 1 | parts | number of chunks |
| 0x30 | 1 | media | `0x12` CD, `0x14` DVD |
| 0x31 | 4 | unknown | 0 |
| 0x35 | 1 | byte08 | `0x08` |
| 0x36 | 10 | unknown2 | 0 |

Chunk names: `ul.%08X.%s.%02x`

- `%08X` = `USBA_crc32(name)`, uppercase hex;
- `%s` = game ID;
- `%02x` = part index, **lowercase** hex, starting at `00`;
- every chunk except the last is exactly **1 GiB** (1,073,741,824 bytes);
- max **10 parts** on SMB; BDM has no explicit limit but fragments count;
- titles <= 32 bytes, ASCII only.

`USBA_crc32` is **not** standard CRC-32. It uses polynomial `0x04C11DB7`, an inverted top-bit test,
a table stored reversed, no initial reset, hashes the terminating NUL, and has no final XOR.
Known-good implementations exist in yaoplm and ps2-game-manager; test with vectors.

### 2.5 `CFG/<ID>.cfg`

- text, one `key=value` per line;
- written with **CRLF**;
- key <= 31 bytes, value <= 255 bytes;
- lines starting with `#` are runtime-only and must never be written;
- **preserve unknown keys and their order**;
- `$Compatibility` bits: 0x01 accurate reads, 0x02 synchronous, 0x04 unhook syscalls,
  0x08 skip videos, 0x10 emulate DVD-DL, 0x20 disable IGR; 0x40/0x80 are unused upstream;
- VMC slots: `$VMC_0`, `$VMC_1`;
- metadata keys used by themes: `Title`, `Genre`, `Release`, `Developer`, `Description`, etc.

### 2.6 `ART/`

- path: `ART/<ID>_<SUFFIX>.png`;
- OPL 1.2.0 beta: **PNG only**; v1.1.0 also accepted JPG/BMP;
- width/height <= 1024; texture <= 720x512x4 bytes;
- recommended sizes from OPL Manager docs:

| Suffix | Type | PS2 | PS1 |
|---|---|---|---|
| `COV` | front cover | 140x200 | 200x200 |
| `COV2` | back cover | 242x344 | 222x200 |
| `LAB` | spine | 18x240 | 12x200 |
| `ICO` | icon | 64x64 | 64x64 |
| `LGO` | logo | 300x125 | 300x125 |
| `SCR`, `SCR2` | screenshots | 250x188 | 250x188 |
| `BG` | background | 640x480 | 640x480 |

- Apps use the boot ELF name: `ART/OPNPS2LD.ELF_COV.png`.
- Do not bundle art. Source licensing is not clear for all OPLM backups.

### 2.7 VMC

- `VMC/<name>.bin`, selected with `$VMC_0` / `$VMC_1`;
- exact size 8/16/32/64 MiB;
- PS2 memory card filesystem, raw user data without ECC;
- page 512, 2 pages/cluster, 16 pages/block, card type 2;
- on BDM the file must be **one contiguous cluster chain**;
- creating an empty file is not enough; OPL expects a formatted card.

### 2.8 CHT

- `CHT/<ID>.cht`, plain text;
- code lines: exactly 16 hex digits, e.g. `2032D3B4 00000063`;
- other non-empty lines are cheat names;
- master codes recognized by name (`mastercode`, `master code`, `enable code (must be on)`);
- max 250 cheats, max 5 hooks;
- raw/decrypted PS2RD codes only; no CodeBreaker/AR/GameShark decryption.

### 2.9 APPS and PS1/POPS

- `APPS/<folder>/title.cfg` with `title=` and `boot=` (lowercase keys), plus the ELF;
- legacy `conf_apps.cfg`: `Title=device:path/to/FILE.ELF`;
- PS1 games are launched as POPStarter apps:
  `POPS/<Name>.VCD` + launcher `XX.<Name>.ELF` (USB) or `SB.<Name>.ELF` (SMB);
  per-game folder `POPS/<Name>/` with `SLOT0.VMC`, `SLOT1.VMC`, `CHEATS.TXT`, etc.;
- OPL itself does not scan `POPS/`; the launcher must be in `APPS/` or `conf_apps.cfg`.

### 2.10 Fragmentation and filesystem limits

- OPL 1.2 BDM: all parts of a game share a 64-fragment budget; total > 64 -> "game is fragmented".
- OPL 1.1.0: each file must be contiguous (`USBMASS_IOCTL_CHECK_CHAIN`); title <= 64 bytes;
  SMB image file name <= 87 bytes; no exFAT or ZSO.
- FAT32: max file 4 GiB -> use `ul`.
- exFAT: supported by 1.2.0 beta only; use plain ISO/ZSO.
- SMB: max 10 `ul` parts; image file name <= 159 bytes on 1.2, <= 87 bytes on 1.1.0.
- 4Kn drives are not supported (512-byte logical sectors only).
- Preallocate and write sequentially; verify fragment count where the OS allows it.

### 2.11 Format cheat sheet

```
ID:        SYSTEM.CNF BOOT2, case-sensitive, after ':', strip '\', until ';', max 11
ISO name:  SLUS_200.62.Title.iso  (positions 4='_', 8='.', 11='.')
Limits:    title <= 160 bytes, file name <= 159 bytes on SMB, ASCII
ul record: 64 bytes, name[32] + "ul." + startup[12] + parts + media + ... + 0x08 at 0x35
ul chunk:  ul.<CRC32 uppercase>.<ID>.<part lowercase hex>; 1 GiB except last; <=10 on SMB
CFG:       key=value, CRLF, key <=31, value <=255, preserve unknown, never write '#'
ART:       ART/<ID>_<SUFFIX>.png, PNG, <=1024 px
VMC:       8/16/32/64 MiB, formatted PS2 MC, contiguous
ZSO:       "ZISO" + u64 size + 2048 block + u32 index + raw LZ4
Cache:     delete CD/games.bin and DVD/games.bin after changes
1.1.0:     title <=64 bytes, SMB filename <=87 bytes, PNG/JPG/BMP, no exFAT/ZSO, contiguous files
```

These constants were re-verified against the OPL source at commit `b3a2199`. See
Appendix A for the source references, corrections, and the "what would break OPL" checklist.

---

## 3. Audit of existing projects

### 3.1 Comparison

| Project | License | Stack | Status | Web/server | UL write | CFG | ART | VMC | PS1/APPS | Fit |
|---|---|---|---|---|---|---|---|---|---|---|
| **OPL Manager V24** | closed | Windows .NET 6 | last release 2024-09 | no | yes | yes | yes (private server) | yes | yes | reference only |
| **yaoplm** | GPL-3.0-or-later | Rust + axum + React/Tauri | active, pre-alpha | **yes** | yes | yes | yes | yes | planned | **best base** |
| **OPL PC Tools 4.0** | GPL-3.0 | C++/Qt | active, mature | no | yes | yes | yes | yes | no PS1/APPS | desktop reference |
| **OrbitPS2 Manager 1.3.1** | GPL-3.0 | Electron/Angular | active | no (wontfix) | **no** | yes | yes | empty cards | yes | UI reference |
| **opl-linux-toolbox 1.4.2** | GPL-3.0-only | Electron/Angular | active, fork of Orbit | no | yes | yes | yes | yes | yes | safety reference |
| **PyOPLM 0.73** | GPL-3.0-or-later | Python CLI | active | no | yes | no | yes | no | POPS | format reference |
| **libopl** | GPL-3.0 | Python CLI | abandoned 2019 | no | yes | no | yes | no | no | avoid |
| **ps2-game-manager** | MIT | Go TUI | active | no | yes | no | covers only | no | no | MIT reference |
| **opl-manager-next** | none | Electron/React | abandoned 2020 | no | no | no | no | no | POPS | avoid |
| **OPLUtil** | none | (repo empty) | abandoned | no | ? | ? | ? | ? | ? | avoid |
| **docker-smb-opl** | Dockerfile | Samba | old | n/a | n/a | n/a | n/a | n/a | n/a | sidecar only |

### 3.2 Deep notes

**yaoplm** (`belomaxorka/yaoplm`)

- 66k Rust LOC across crates/apps, 16.5k TS/TSX in `web/src`, ~113 Rust test files, fuzz targets for
  ISO/CFG/ul.cfg/ZSO/CUE parsers.
- Crate split is clean: `formats`, `core`, `hash`, `device`, `net`, `api`, `i18n`, plus CLI/server/desktop.
- Safety model: plan -> confirm -> execute, `*.yaoplm-part` temp files, atomic rename, journal in
  `.yaoplm/journal/`, CFG backups, per-root path policy, one write job at a time.
- Server: `POST /api/rpc`, WebSocket `/api/events`, `GET /media/art`, `GET /health`, SPA fallback.
  Token auth, root allow-list, loopback by default, DNS-rebinding protection.
- Gaps for us: no Dockerfile (roadmap SRV-1), no browser upload, single token auth, no users/audit,
  upstream docs were Russian (translated here), pre-alpha, API may churn.
- Some choices we may not need: Tauri desktop, loader installation/update, Redump verification,
  ScreenScraper. They can stay dormant or be trimmed in the `oplmdash` UI.

**OPL PC Tools** (`brainstream/OPL-PC-Tools`)

- The most mature OSS desktop manager; GPL-3.0, Qt6, Windows/Linux.
- Strong feature set: ISO/ZSO/UL conversion, install from disc, CFG, VMC read/write, cheats,
  archive.org art.
- Not a base for us: C++/Qt desktop, no macOS, no PS1/APPS, no web.
- Useful as a behavioural reference for UL/VMC/CFG edge cases.

**OrbitPS2 Manager / opl-linux-toolbox**

- OrbitPS2 is the strongest Electron manager and the recommended companion for RiptOPL.
- It **does not write `ul.cfg`**, so it cannot install >4 GiB games on FAT32. Its web-UI request was
  closed as wontfix. VMC creation is zero-filled, not formatted.
- `opl-linux-toolbox` is a GPL-3.0 Linux-first derivative with better safety (`.part` copies,
  SHA-256, rollback) and UL handling. Good source of UX/safety ideas, not a web base.

**PyOPLM**

- Useful, compact Python implementation of `ul.cfg`, ISO ID, ZSO, BIN/CUE tools.
- Not a base: CLI only, no CFG editor/VMC/web, and several format bugs:
  - chunk part index built with uppercase hex in `ULGame.get_filenames()` while OPL uses lowercase
    `%02x` (breaks case-sensitive SMB);
  - ISO rename caps titles at 32 characters and counts characters where OPL counts bytes;
    OPL allows 160 bytes;
  - media/CD detection is a 700 MB size heuristic;
  - no 10-part SMB limit enforcement.
- GPL-3.0-or-later; do not copy into a permissive project.

**ps2-game-manager**

- MIT, Go TUI, small and readable. Correct custom USBA CRC32 and basic `ul.cfg` handling.
- Too limited: `ul` only, no ISO/ZSO library, no CFG/VMC/ZSO, JPG covers (wrong for OPL 1.2),
  non-atomic `ul.cfg` writes, no web.
- Could be a source of MIT-licensed algorithms if we go clean-room, with attribution.

**libopl / opl-manager-next / OPLUtil**

- libopl: abandoned 2019, author calls it buggy.
- opl-manager-next: abandoned 2020, no license file, incomplete (317 LOC).
- OPLUtil: repository contains only a README; no source to audit.
- Do not build on any of these.

### 3.3 License matrix

| License | Projects | Can we fork/build on? | Obligations |
|---|---|---|---|
| GPL-3.0-or-later | yaoplm, PyOPLM, libopl, OPL-PC-Tools | yes, if our project is GPL-3.0-or-later | source, license, notices, no extra restrictions |
| GPL-3.0-only | opl-linux-toolbox | yes, as GPL-3.0-only | same as above, no "or later" |
| MIT | ps2-game-manager | yes, permissive | keep copyright/license notice |
| No license | opl-manager-next, OPLUtil | **no** (all rights reserved by default) | do not copy |
| Closed | OPL Manager | no | reference/behaviour only |
| AFL-3.0 | OPL itself | do not copy code | reimplement from behaviour |

Important: GPL is viral for derivative works. If `oplmdash` links/fork yaoplm code, the whole
distribution must be GPL-3.0-or-later. Running an unmodified GPL server as a separate process and
serving our own UI over its HTTP API is a different boundary, but the safe and honest choice is still
to license the combined distribution under GPL and provide sources.

---

## 4. Build vs fork decision

### 4.1 Option A - Fork yaoplm (recommended if GPL is acceptable)

What we get:

- byte-exact formats, tested against OPL source;
- safe write pipeline, journal, recovery, backups;
- server + SPA + generated TS types;
- CLI for automation;
- active upstream to pull fixes from.

What we add:

- `packaging/docker/` multi-stage build + compose + non-root + healthcheck;
- rebrand and simplify the UI for web/mobile;
- resumable upload endpoint + staging area;
- session auth, users/roles, audit log;
- read-only mode and safer defaults;
- docs for NAS mounts, permissions, SMBv1.

Costs/risks:

- GPL-3.0-or-later for the whole product;
- learning a 66k LOC Rust codebase;
- upstream is pre-alpha and may churn; keep divergence small and send fixes upstream;
- Upstream docs were Russian; this fork translated them to English.

### 4.2 Option B - Thin wrapper around upstream `yaoplm-server`

Run the upstream server binary unchanged in Docker, reverse-proxy `/api` and `/media`, and serve our
own simple SPA. Add a small sidecar for auth/uploads.

Pros:

- smallest codebase, fastest to a demo;
- upstream updates are trivial (bump pinned version);
- we can replace the UI entirely without forking Rust.

Cons:

- no upload endpoint, so uploads need a sidecar that writes into the root and then calls `PlanImport`;
- no users/audit; auth is a shared token;
- API is pre-alpha and may break;
- we cannot fix core bugs ourselves without forking anyway;
- two processes to deploy and secure.

This is a good **spike** and a fallback, not the long-term shape.

### 4.3 Option C - Clean-room new codebase

Write our own backend and frontend, using OPL source and format research as the specification.

Pros:

- license freedom (MIT/Apache);
- minimal, web-first codebase tailored to the use case;
- no upstream churn.

Cons:

- must reimplement UL split + USBA CRC, ZSO, CFG, VMC, ART, ID parsing, APPS/POPS;
- data-loss risk is high without years of edge cases;
- 3-6x the effort; real PS2 testing is mandatory;
- "clean-room" must be real: do not read GPL implementation code while writing ours; use the OPL
  source (AFL) and the format notes only. Even then, legal risk is lower but not zero.

### 4.4 Recommendation and fallback

**Chosen (D1): Option A** - downstream fork of yaoplm, GPL-3.0-or-later, with a small, clearly
separated `oplmdash` layer (Docker, UI, upload, auth). Start with a short Option B spike to validate
the API and deployment, then fork.

**Not chosen:** Option C (clean-room) remains the fallback only if the GPL decision is reversed:

- scan/read-only dashboard, game details, art display;
- safe rename/delete/move, CFG editing, art import;
- no `ul.cfg` writing, no ZSO conversion, no VMC creation, no PS1/APPS in v0.1;
- add `ul`/ZSO/VMC only after a format test suite and real-console validation.

Do not "borrow" yaoplm code into a permissive project.

### 4.5 Upstream strategy

- Pin a specific yaoplm commit/tag; vendor it or use a git submodule/fork.
- Keep `oplmdash` changes in separate commits/directories so `git rebase`/merge stays possible.
- Send Docker, upload, and auth improvements upstream as PRs where they fit.
- Maintain `NOTICE`/`FORK.md` describing provenance and changes.
- Track upstream releases; do not auto-merge blindly.

---

## 5. Product definition

### 5.1 Users and jobs

| User | Job to be done |
|---|---|
| PS2 owner with a USB drive | see what is installed, fix names/art/CFG, add games, know what is broken |
| NAS owner | manage the shared OPL folder from a laptop/phone without SMB mounts |
| Collector | large library: search, filter, duplicates, missing art, dump/format warnings |
| Tinkerer | convert ISO/ZSO/UL, edit compatibility, create VMC, manage cheats/apps |

### 5.2 MVP scope (v0.1) - decided

Must have:

- Docker image + compose; one OPL root mounted from the NAS; data volume for cache/uploads.
- Read-only first-run mode; writes enabled only by explicit configuration.
- Scan library: CD/DVD ISO/ZSO, ul.cfg, ART, CFG, VMC, CHT, APPS (read).
- Dashboard: counts, total size, media/format breakdown, warnings (no BOOT2, bad names,
  missing art, stale games.bin, non-ASCII ul names, >64 fragments where detectable).
- Library grid/list with search/filter/sort and virtualization for 3000+ games.
- Game detail: ID, title, media, format, size, paths, warnings, art.
- Safe operations: rename, delete (trash + confirmation), move CD<->DVD, CFG edit, art set/delete.
- Resumable browser upload of `.iso` and existing `.zso` into `CD/` or `DVD/`, with checksum and staging.
- Import preflight: free space, FAT32 4 GiB limit, name/ID validation, `games.bin` invalidation.
- Plan preview + job progress + cancel + job log.
- Session login (single admin), CSRF protection, audit log.
- Mobile-friendly layout.

Explicitly not in v0.1 (deferred to v0.2):

- `ul.cfg` writing / UL splitting for games >4 GiB on FAT32.
- ZSO conversion (uploading an existing `.zso` is allowed).
- VMC creation (listing/reading VMCs is read-only in v0.1).
- PS1/POPS, APPS installation, cheats editor.
- Automatic art fetching (display and manual set/delete only until sources are reviewed).
- Internal HDD (APA/PFS), direct SMB client, Samba sidecar.

Should have (v0.2):

- UL split, ZSO conversion, formatted VMC creation, art fetch, PS1/APPS, cheats.
- Multi-user roles, audit UI, backup browser, duplicate finder.
- Direct SMB2/3 client and Samba sidecar for PC-hosted USB.
- Loader install/update if inherited and desired.

### 5.3 Non-goals

- Downloading games, BIOS, POPS files, or copyrighted art bundles.
- Formatting drives in v1.
- Implementing an SMB1 server inside the app.
- Being a general file manager.
- Supporting every OPL fork's custom features in v1.

---

## 6. Architecture

### 6.1 Chosen shape (fork path)

```
Browser (LAN)
   |
   | HTTPS (reverse proxy) / HTTP (LAN)
   v
oplmdash container
   +-------------------------------------------------------+
   |  axum server                                          |
   |    /            -> embedded React SPA                 |
   |    /api/rpc     -> JSON-RPC (yaoplm protocol)         |
   |    /api/events  -> WebSocket job events               |
   |    /api/upload  -> resumable upload (new)             |
   |    /media/art   -> art bytes with ETag                |
   |    /healthz     -> liveness/readiness                 |
   |                                                       |
   |  oplmdash-api / yaoplm-api                            |
   |    Request/Response/JobEvent, dispatch, JobManager    |
   |                                                       |
   |  core crates (sync, no axum/tokio)                    |
   |    formats, core, hash, device, net                   |
   +-------------------------------------------------------+
        |                         |
        v                         v
   /opl (rw or ro)          /data (cache, uploads, db)
   OPL library              SQLite, art cache, staging
```

Key rules:

- core crates stay synchronous and UI/transport agnostic;
- server is a thin adapter; all business logic in core/api;
- filesystem is the source of truth; SQLite is only cache/index/auth/audit;
- one writer at a time per root (job queue + lock);
- no shell commands; no `..`; symlinks resolved and rejected if outside root.

### 6.2 Chosen stack (fork path)

- Rust stable (pinned), axum 0.8, tokio only at the edges.
- React 19 + TypeScript + Vite + TanStack Query/Virtual + Tailwind/shadcn.
- SQLite via rusqlite (bundled) for cache/auth/audit.
- Docker multi-stage: pnpm build -> cargo build (musl) -> distroless/static; non-root.
- Generated TS types via ts-rs; CI checks freshness.
- i18n: English only; inherited Russian support was removed.

If clean-room: Rust + axum + React is still the best fit for large-file I/O and single-binary
Docker. A TypeScript-only stack (Fastify/NestJS + React) is faster to write but weaker for raw
device/fragmentation work; Python/FastAPI is fine for a read-only MVP but needs careful streaming
and concurrency. Go is a good middle ground (single binary, good I/O) but has fewer OPL libraries.

### 6.3 Docker packaging

Multi-stage Dockerfile:

1. `node:24` (or pinned) builds `web/` with pnpm.
2. `rust:1.90` + musl target builds `yaoplm-server` release.
3. `gcr.io/distroless/static` (or `scratch` with CA certs) copies the binary and `web/dist`.
4. `USER 10001:10001`, `EXPOSE 7878`, `HEALTHCHECK` on `/health`.
5. Entrypoint reads env: `OPLMDASH_BIND`, `OPLMDASH_TOKEN`, `OPLMDASH_ROOTS`, `OPLMDASH_DATA_DIR`,
   `OPLMDASH_READ_ONLY`, `OPLMDASH_LOG`.

Compose example (documentation):

```yaml
services:
  oplmdash:
    image: ghcr.io/<org>/oplmdash:latest
    ports: ["7878:7878"]
    environment:
      OPLMDASH_BIND: 0.0.0.0:7878
      OPLMDASH_TOKEN: ${OPLMDASH_TOKEN:?set a strong token}
      OPLMDASH_ROOTS: /opl
      OPLMDASH_DATA_DIR: /data
      OPLMDASH_READ_ONLY: "1"
    volumes:
      - /mnt/opl:/opl
      - oplmdash-data:/data
    user: "1000:1000"
    restart: unless-stopped
```

Operational notes:

- The container user must be able to read/write `/opl` (match UID/GID of the host share).
- SMB/CIFS bind mounts may not support atomic rename, hard links, fsync, or POSIX permissions
  reliably. Test on the target NAS; document fallbacks.
- If the share is remote CIFS, prefer mounting it on the host and bind-mounting the mountpoint.
- Do not run as root. Do not expose without TLS + auth on untrusted networks.
- Provide a `read_only: true` root mode and a separate writable data volume.

### 6.4 Upload design (browser -> server)

For users whose ISOs are not already on the server:

- `POST /api/uploads` -> create upload session (name, size, sha256 optional, target root).
- `PUT /api/uploads/<id>?offset=` -> stream chunk to `data/uploads/<id>.part`; no memory buffering.
- `POST /api/uploads/<id>/complete` -> verify size/hash, fsync, move into a staging folder inside the
  target root (same filesystem, atomic rename), then call `PlanImport` with that path.
- Resume: server reports received bytes; client retries from the last offset.
- Limits: max size, max concurrent uploads, quota, idle timeout, cleanup of abandoned sessions.
- Reverse proxy: raise body size/timeouts; support HTTP/1.1 chunked or HTTP/2.
- Never trust the client filename: sanitize, ASCII, length limits; keep original title separately.
- Checksum (SHA-256) computed while streaming; compare if provided; store in audit.
- After import, delete staging; keep upload session record for audit.

Alternative MVP: document `/opl/incoming/` as a drop folder on the server and only support
`PlanImport` from server paths. Much less code; good for NAS users who can copy over SMB anyway.

### 6.5 Auth and security

MVP:

- single admin password or strong token from env; login sets an HttpOnly, Secure, SameSite=Lax
  session cookie; CSRF token for state-changing requests;
- rate-limit login; lockout/backoff;
- token never in URLs except the WebSocket handshake (upstream pattern);
- no CORS by default; same-origin only; reject foreign `Host` (DNS rebinding);
- CSP, `X-Content-Type-Options`, `Referrer-Policy`, frame deny;
- path policy: only configured roots, no symlink escape, no `..`, no absolute paths from client;
- uploads and art served with correct content types and `ETag`;
- audit log of every write: who, when, what, plan id, result.

Later:

- users table, argon2id, roles (admin/viewer/uploader);
- API tokens for automation;
- per-root permissions;
- optional OIDC for homelab users.

### 6.6 Safety model (inherited/kept)

1. **Plan**: compute steps, sizes, warnings, conflicts; write nothing.
2. **Confirm**: UI shows plan; destructive steps highlighted; explicit checkbox for delete.
3. **Execute**: job queue, progress events, cancel token.
4. **Atomicity**: large files to `*.part` then rename; small files temp + fsync + rename + backup.
5. **Journal**: per-root `.oplmdash/journal/`; rollback before commit, roll forward after.
6. **Backups**: CFG and `ul.cfg` before modification; retention policy.
7. **Locks**: one writer per root; readers see a consistent snapshot.
8. **Recovery**: on startup, detect incomplete journal entries and offer resume/rollback.
9. **Read-only mode**: disables all writes at the API layer, not just the UI.
10. **Trash (optional)**: deletes move to `.oplmdash-trash/<timestamp>/` with retention, instead of
    immediate unlink. This is a strong safety feature for a web UI.

### 6.7 API surface

Keep yaoplm's protocol if forking. Add:

- `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/auth/me`;
- upload endpoints (above);
- `GET /api/stats` (dashboard aggregates, cached);
- `GET /api/audit` (paginated);
- `GET /api/config` (safe runtime config: roots, read-only, version);
- optional `GET /api/events` SSE fallback for proxies that block WebSocket.

Core requests already available in yaoplm:

```
ListDevices, InspectDevice, ScanLibrary, ImageInfo, ListDir,
PlanInit, PlanImport, ConvertGames, RenameGame, DeleteGame, MoveGame,
PlanRecovery, ExecutePlan, DiscardPlan, GetJob, ListJobs, CancelJob,
FetchArt, FindArt, ImportArt, SetArt, DeleteArt, Attribution, ArtCache, ClearArtCache,
ReadCfg, WriteCfg, ListVmc, CreateVmc,
VerifyGames, RedumpStatus, UpdateRedump,
loader catalog/install/update/rollback requests
```

### 6.8 Frontend screens

```
/login
/library                 grid/list, search, filters, stats, bulk selection
/library/add             upload or server path, options, plan preview
/library/game/:id        details, art slots, CFG, VMC, cheats, actions
/devices                 root health, filesystem, free space, warnings, profile
/jobs                    active/history, progress, log, cancel, recovery
/settings                roots, naming, profile, art sources, language, theme, read-only
/about                   version, licenses, attribution, source link
```

Keep it simple and mobile-first. Avoid desktop-only patterns (drag from OS, native dialogs).
Use a server-side path browser for imports; use file upload only when needed.

---

## 7. Roadmap (decided)

### Phase 0 - Spike (about 1 week)

> **Status 2026-10-25:** README, provenance, Docker files and the upload prototype are in place.
> See `docs/SPIKE.md` for the verified commands and the go/no-go. The Docker build and the SPA
> decision are still open.

- [x] Fork/pin upstream yaoplm at a known commit; add `FORK.md`, `NOTICE`, keep `LICENSE`.
- [ ] Build and run `yaoplm-server` from source in a container.
- [ ] Mount a synthetic OPL library (fixtures) and a real copy (read-only).
- [ ] Exercise `ScanLibrary`, `ListDir`, `RenameGame` plan, `ReadCfg`, `FetchArt` (offline).
- [x] Add a minimal Dockerfile + compose for the NAS local-mount model.
- [x] Prototype the resumable upload endpoint writing to staging and handing the path to `PlanImport`.
- [ ] Decide whether to trim the inherited SPA or build a smaller `oplmdash` SPA on the same API.
- [x] Deliverable: `docs/SPIKE.md` with findings, Dockerfile prototype, screenshots, go/no-go.

### M1 - Fork + Docker + read-only dashboard (2-3 weeks)

- Rebrand to `oplmdash`; add GPL notices and upstream attribution.
- Docker image + compose; non-root, healthcheck, multi-arch.
- Simplify the SPA: library, game details, art, device health, jobs.
- Scan 1000+ game fixture library; virtualization; mobile layout.
- Read-only mode by default; no write endpoints exposed.
- Acceptance: `docker compose up` on a NAS-like mount; scans without writes; works on phone.

### M2 - Auth + safe writes (2-3 weeks)

- Session login, CSRF, rate limit, audit log.
- Rename, delete (trash), move CD<->DVD, CFG edit, art set/delete.
- Plan preview, progress, cancel, recovery UI.
- Backup browser and restore.
- Acceptance: e2e tests for every write; simulated crash recovery; no data loss in fault injection.

### M3 - Browser upload + import (3-4 weeks)

- Resumable upload with checksum, staging, resume after disconnect.
- Import `.iso` and existing `.zso` into `CD/` or `DVD/`; no UL/ZSO conversion.
- Preflight: free space, FAT32 4 GiB limit, ID/name validation, `games.bin` invalidation.
- Job queue with one writer per root; progress/cancel.
- Acceptance: 10 GB upload resumes; a game that fits the filesystem appears and boots in OPL;
  a >4 GiB FAT32 upload is refused with a clear message.

### M4 - Art + dashboard polish + hardening (2-3 weeks)

- Art display, manual set/delete; optional fetch only after source licensing review.
- Dashboard stats, warnings, empty states, mobile polish.
- Audit log UI, backup/restore UI, troubleshooting page.
- Security review, dependency audit, SBOM.
- Acceptance: axe/accessibility pass, no high-severity findings, art and CFG visible in OPL.

### M5 - Release v0.1.0 (1-2 weeks)

- Multi-arch image (amd64/arm64), healthchecks, graceful shutdown, image signing.
- Documentation: install, NAS mounts, permissions, SMBv1 warning, backup/restore, troubleshooting.
- CI: fmt/clippy/test/web build/e2e/docker build.
- Release notes, known limitations, source offer.

### Later (v0.2+)

- UL split, ZSO conversion, formatted VMC, PS1/POPS, APPS, cheats.
- Multi-user roles and OIDC.
- Direct SMB2/3 client and Samba sidecar for PC-hosted USB.
- Internal HDD (APA/PFS), NBD.
- Loader install/update.
- CSV/HTML export, duplicate finder, Redump verification.

---

## 8. Testing strategy

### Unit and property tests

- `SYSTEM.CNF` parsing: case, whitespace, missing `:`, `;`, long IDs, BOM, NUL.
- Old/new file-name parsing and limits, counted in bytes (not Unicode chars).
- Profile-specific limits: 1.1.0 title 64 / SMB 87 / `games.bin` 92 / no exFAT/ZSO /
  contiguous files; 1.2 title 160 / SMB 159 / `games.bin` 188 / 64 fragments.
- `USBA_crc32` known vectors; `ul.cfg` round-trip; chunk naming; 10-part limit.
- CFG round-trip preserving unknown keys and order; CRLF; limits.
- ZSO header/index/block round-trip; corrupt blocks.
- VMC superblock and exact size; contiguity check where possible.
- CHT parser limits and master-code detection.
- Path policy: traversal, symlinks, absolute paths, Unicode, OPL buffer lengths.
- Fuzzing for every parser (already in yaoplm).

### Fixtures

- Tiny ISO with `SYSTEM.CNF` and a boot ELF.
- ISO without `BOOT2`.
- CD and DVD variants.
- Sparse >4 GiB ISO for `ul` splitting.
- BIN/CUE single and multi-track.
- ZIP/7z/CHD samples.
- FAT32 and exFAT filesystem images for fragmentation/limits.
- OPL 1.1.0-style library (92-byte `games.bin`, title 64, SMB name 87, contiguous files) and
  OPL 1.2.0-style library (188-byte `games.bin`, title 160, SMB name 159).
- Synthetic `ul.cfg` with multiple games and edge-case names.
- Art of each type and wrong sizes/bit depth.
- Never use real commercial games in the repo.

### Integration and e2e

- Temp directories; no real removable drives in automated tests.
- Optional Samba container for SMB path tests.
- Playwright against the Docker container: login, scan, plan, confirm, rename, delete, recover,
  upload resume, read-only enforcement.
- Fault injection: kill during copy, full disk, permission denied, CIFS rename failure.

### Real-console checklist

1. OPL 1.1.0 and 1.2.0 beta see the same library.
2. FAT32 CD game, DVD game <= 4 GiB, and `ul` game > 4 GiB boot.
3. exFAT + ZSO boots on 1.2.0 beta.
4. SMB share on the NAS is visible and boots.
5. MX4SIO card works.
6. ART, CFG compatibility modes, and VMC work.
7. Rename keeps CFG/ART/VMC associations.
8. Delete leaves no orphaned files that break OPL.
9. `games.bin` is regenerated correctly.
10. Fragmentation warnings match reality.

---

## 9. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Data loss during write | catastrophic | plan/confirm, atomic writes, journal, backups, trash, read-only mode, fault-injection tests |
| Format drift in OPL betas | games disappear/corrupt | profile system, tests against OPL source, real-console checklist |
| Upstream yaoplm churn | merge pain | pin commit, minimal divergence, upstream PRs, vendor if needed |
| GPL obligations | legal | GPL-3.0-or-later, NOTICE, source offer, no proprietary bundling |
| SMBv1 insecurity | network exposure | document, no built-in SMB1 server, isolated share, optional Samba sidecar with warning |
| Docker permissions/UID | write failures | document UID/GID, init container, test on NAS, clear errors |
| CIFS semantics | atomic rename/fsync failures | detect FS, fallback copy+rename, warn, test |
| Large uploads | timeouts/memory | streaming, chunking, resume, quotas, proxy config |
| Art source licensing | legal | opt-in, attribution, no bundling, review each source |
| Multi-user concurrency | conflicts | one writer per root, job queue, optimistic locking, audit |
| Browser security | account takeover | HttpOnly cookies, CSRF, CSP, rate limit, TLS, no token in URLs |
| Pre-alpha upstream | instability | treat as beta, pin, test, have wrapper fallback |
| Clean-room legal risk (if chosen) | legal | true clean-room process, OPL spec only, no GPL code reading during implementation |

---

## 10. Decisions and remaining questions

### Resolved (2026-10-25)

| Question | Decision |
|---|---|
| License and base | Fork yaoplm; GPL-3.0-or-later (D1) |
| Upload | Resumable browser upload in v0.1 (D2) |
| Deployment | NAS Docker with a local mount (D3) |
| MVP scope | Dashboard + safe library ops; no UL/ZSO/VMC/PS1 in v0.1 (D4) |

### Still open

1. **Branding/repo:** repository owner, image name (`ghcr.io/<org>/oplmdash`), and whether the
   project is a public fork or a separate repo with yaoplm as a git remote.
2. **OPL versions:** read both 1.1.0 and 1.2.0 beta. Verified differences: 1.1.0 has a 64-byte
   title limit, 87-byte SMB filename limit, 92-byte `games.bin`, PNG/JPG/BMP art, no exFAT/ZSO,
   and requires contiguous files. Decide whether v0.1 detects the profile and enforces these
   limits automatically (recommended), or asks the user to choose a profile.
3. **Auth model:** password + session cookie (recommended) vs long-lived token only; whether to add
   a `viewer` role in v0.1.
4. **Upload limits:** maximum file size, concurrent uploads, per-user quota, staging retention.
5. **Delete policy:** trash with retention (recommended) vs immediate delete after confirmation.
6. **Art fetching:** enable inherited `FetchArt` in v0.1 or keep it hidden until source licensing is
   reviewed.
7. **Inherited features:** hide yaoplm's loader install/update and Redump verification in the
   `oplmdash` UI, or keep them as advanced features.
8. **CI/registry:** GitHub Actions + GHCR vs another registry; image signing tool (cosign).

### Phase 0 next actions

1. [x] Create the fork and pin the upstream commit; add `FORK.md`, `NOTICE`, keep `LICENSE`.
2. [ ] Build `yaoplm-server` locally and run it against a fixture OPL library.
3. [x] Add `packaging/docker/Dockerfile` + `compose.yaml` for the NAS mount model.
4. [x] Prototype the upload endpoint and verify `PlanImport` can consume the staged path.
5. [ ] Evaluate the inherited SPA: trim vs replace. Record the decision in `docs/oplmdash/DECISIONS.md`.
6. [x] Write `docs/SPIKE.md` with results and a go/no-go for M1.

---

## 11. Sources

Primary:

- OPL source: <https://github.com/ps2homebrew/Open-PS2-Loader>
- OPL reviewed commit: `b3a2199795fe89ba7e2dd7b843354472dff2c3eb` (2026-10-12).
  Key files: `src/supportbase.c`, `src/system.c`, `src/ps2cnf.c`, `src/config.c`,
  `src/util.c`, `src/textures.c`, `src/themes.c`, `src/bdmsupport.c`, `src/ethsupport.c`,
  `src/appsupport.c`, `src/opl.c`, `include/supportbase.h`, `include/config.h`,
  `include/mcemu.h`, `modules/isofs/zso.c`, `modules/isofs/zso.h`,
  `modules/vmc/genvmc/genvmc.c`, `modules/iopcore/common/cdvd_config.h`,
  `modules/iopcore/cdvdman/device-smb.c`, `pc/iso2opl/*`, `pc/ziso.py`.
- OPL Manager docs: <https://oplmanager.com/site/?docs>
- yaoplm: <https://github.com/belomaxorka/yaoplm>
- PyOPLM: <https://github.com/edisnord/PyOPLM>
- libopl: <https://github.com/Nold360/libopl>
- OPL PC Tools: <https://github.com/brainstream/OPL-PC-Tools>
- OrbitPS2 Manager: <https://github.com/Luden02/OrbitPS2-Manager>
- opl-linux-toolbox: <https://github.com/lucasonline0/opl-linux-toolbox>
- ps2-game-manager: <https://github.com/dheison0/ps2-game-manager>
- opl-manager-next: <https://github.com/N0NamedGuy/opl-manager-next>
- docker-smb-opl: <https://github.com/milouk/docker-smb-opl>

yaoplm research used for format facts:

- `docs/research/02-opl-formats.md` (OPL on-disk formats)
- `docs/research/01-competitors.md` (competitive analysis)
- `docs/research/04-stack.md` (stack research)
- `docs/research/08-format-verification.md` (USBA CRC test vectors)
- `docs/research/15-cfg-vmc-prior-art.md` (CFG/VMC prior art)

Community/format references:

- POPStarter documentation: <https://nathanneurotic.github.io/POPSTARTERINFO/>
- Redump: <https://redump.org/>
- PCSX2 patch docs: <https://pcsx2.net/docs/advanced/writing-patches>
- OPL art backups on archive.org: `OPLM_ART_2024_09` (no stated license; do not bundle)

---

## Appendix A - OPL source verification and compatibility guardrails

Reviewed against `ps2homebrew/Open-PS2-Loader` at commit
`b3a2199795fe89ba7e2dd7b843354472dff2c3eb` (2026-10-12), the same commit used by
yaoplm's format research. The source is AFL-3.0; it was read as a specification only.
No OPL code is copied into `oplmdash`.

### A.1 Verified constants and behaviours

| Area | OPL source | Verified behaviour | Our rule |
|---|---|---|---|
| Folders | `src/supportbase.c` `sbCreateFolders()` | Creates `CFG THM LNG ART VMC CHT APPS`, plus `CD DVD` on image devices | Create the same; never invent required folders |
| ISO names | `src/supportbase.c` `isValidIsoName()` | `.iso`/`.zso` case-insensitive; old format needs len >= 17 and `_` at 4, `.` at 8, `.` at 11; new format len >= 5; title > 160 is skipped | Match exactly; ASCII-only on write |
| Game ID | `src/ps2cnf.c` `ps2cnfGetBootFile()` + `src/supportbase.c` `GetStartupExecName()` | Reads <= 1023 bytes; `BOOT2` case-sensitive; value is printable ASCII up to whitespace; skip to first `:`, skip `\`, stop at `;`; max 11 chars; boot path buffer 64 | Parse exactly; warn when `BOOT2` is missing or odd |
| CD vs DVD | `src/supportbase.c` `sbReadList()` | Media comes from the `CD/` or `DVD/` folder, not from the image size | Folder is authoritative; size is only a UI hint |
| `games.bin` | `include/supportbase.h` `base_game_info_t`; `src/supportbase.c` `scanForISO()` | 188-byte records in current OPL; 92-byte records in 1.0.0/1.1.0/old betas; keyed by `name + extension`; stale entries survive a same-name replacement | Delete after changing `CD/` or `DVD/`; detect generation before parsing |
| `ul.cfg` | `include/supportbase.h` `USBExtreme_game_entry_t`; `src/supportbase.c` `sbReadList()`/`sbRebuildULCfg()` | 64-byte records; `name[32]`, `"ul."` at 0x20, `startup[12]` at 0x23, `parts` 0x2F, `media` 0x30, `0x08` at 0x35; `size / 64` records; no header | Write exact 64-byte records; zero reserved bytes; `0x08` at 0x35 |
| `ul` chunks | `src/supportbase.c` `sbCreatePath_name()`; `modules/iopcore/cdvdman/device-smb.c` | `ul.%08X.%s.%02x`; uppercase CRC, lowercase hex part; 1 GiB (0x80000 sectors) except last; SMB max 10 parts | Write lowercase hex; verify sizes; enforce 10 parts on SMB |
| `USBA_crc32` | `src/system.c` `USBA_crc32()` | Custom CRC, reversed table, seed 0, hashes the NUL, up to 33 bytes, no final XOR; signed `char` makes bytes >= 0x80 undefined | Use the RiptOPL/unsigned result; require 7-bit ASCII titles |
| CFG | `src/config.c`; `src/util.c` `readFileBuffer()` | `#` only in column 0 is a comment; no trimming; key <= 31, value <= 255; CRLF on write; `name:` prefix for indented lines; repeated key keeps first position and last value; OPL writes resolved keys and drops `#`/comments | Preserve unknown keys and order; byte-preserve untouched files; validate limits |
| ART | `src/bdmsupport.c` `bdmGetImage()`; `src/textures.c` `texDiscoverLoad()`/`texSizeValidate()`; `src/themes.c` | Path `ART/<ID>_<SUFFIX>.png`; current master appends `.png` only; width/height <= 1024; texture <= 720*512*4; default theme uses `COV`, `ICO`, `BG`, `SCR`, `SCR2`; custom themes may use any suffix | PNG only; <= 1024; correct suffix; do not bundle art |
| VMC | `src/system.c` `sysCheckVMC()`; `modules/vmc/genvmc/genvmc.c` | `VMC/<name>.bin`; magic starts with `Sony PS2 Memory Card Format`, type 2, exact `pages_per_cluster * clusters_per_card * page_size`, whole MiB; UI offers 8/16/32/64; BDM requires one contiguous chain | Offer 8/16/32/64; write a formatted card; check contiguity on BDM |
| CHT | `src/supportbase.c` `sbLoadCheats()`; `src/cheatman.c` | Path buffer 64; `CHT/<ID>.cht`; 16 hex digits per code; `//` or `#` comments anywhere; mastercode names; 250 codes, 5 hooks | Raw codes only; validate limits; keep path short |
| APPS | `src/opl.c` `oplScanApps()`; `src/appsupport.c` | `APPS/<folder>/title.cfg`; `title` and `boot` required (lowercase keys); title <= 128, boot <= 64, argv1 <= 128, path <= 128; memory card root folders need `_`; legacy `conf_apps.cfg` | Validate `title.cfg`; `boot` is a file name, not a path; keep names short |
| ZSO | `modules/isofs/zso.c`/`.h`; `pc/ziso.py` | 24-byte header `ZISO`; 2048-byte blocks; index at 24; bit 31 plain; raw LZ4 blocks; `align <= 11`; 1.2.0 beta only | Read/write exact; never write ZSO for 1.1.0 |
| SMB | `modules/iopcore/common/cdvd_config.h`; `device-smb.c` | SMBv1 only; filename buffer 160; `ISO_MAX_PARTS` 10; 0x80000-sector part step; prefix/share/user/pass buffers 32 | Keep image names <= 159 bytes; <= 10 parts; warn on long SMB settings |
| Fragmentation | `src/bdmsupport.c` `bdmLaunchGame()` | All parts share one 64-entry fragment array; per-part check, total budget 64; 1.1.0 wants each file contiguous | Check total fragments, not just per part |

#### A.1.1 Profile differences: OPL 1.1.0 vs current 1.2.0 beta

| Item | OPL 1.1.0 (`v1.1.0` tag) | Current 1.2.0 beta (`b3a2199`) | Our rule |
|---|---|---|---|
| ISO title limit | 64 bytes (`ISO_GAME_NAME_MAX 64`) | 160 bytes | Use the active profile's limit; warn when 1.1.0 and title > 64 bytes |
| SMB image filename buffer | 88 bytes (`filename[88]`, max 87) | 160 bytes (max 159) | Enforce 87 for 1.1.0 SMB, 159 for 1.2 |
| `games.bin` record | 92 bytes | 188 bytes | Detect generation; delete rather than rewrite when unsure |
| Image formats | PNG, JPG, BMP | PNG only | Always write PNG; never write JPG/BMP |
| exFAT | no | yes (rev1880+) | Refuse exFAT for the 1.1.0 profile |
| ZSO | no | yes | Refuse ZSO for the 1.1.0 profile |
| Fragmentation | each file contiguous (`USBMASS_IOCTL_CHECK_CHAIN`) | <= 64 fragments per game total | Check per profile |
| Art | PNG, JPG, BMP | PNG only | Always write PNG |
| VMC | same format/size rules | same format/size rules | Same |

The 1.1.0 values come from `git show v1.1.0:include/supportbase.h` and
`git show v1.1.0:modules/iopcore/common/cdvd_config.h`. The plan must never assume 1.2 limits
when the user's profile is 1.1.0.

### A.2 Corrections and clarifications to the plan

1. **Byte lengths, not Unicode characters.** OPL uses `strlen` and byte positions. yaoplm's
   `iso_name::classify` uses `chars()`. For ASCII names they agree; for non-ASCII they can
   diverge. Our rule: enforce ASCII-only names on write; when scanning existing names, count
   bytes to mirror OPL, and warn on non-ASCII. Consider changing `iso_name` to byte semantics
   as part of the fork.
2. **`ul` part index format.** OPL reads `%02x` (lowercase hex); `pc/iso2opl` writes `%02d`
   (decimal). They agree only for parts 0-9. SMB allows 10 parts and DVD-9 needs at most 9,
   so real libraries are safe; we write lowercase hex and enforce <= 10 parts on SMB.
3. **`ul.cfg` live reload.** OPL reloads `ul.cfg` when its size changes (`sbIsSameSize`).
   A rewrite with the same size (for example a title-only rename) may not be noticed until
   OPL refreshes or reboots. Our UI must say "refresh or restart OPL" after `ul.cfg` edits.
4. **VMC validation is narrower than "formatted".** `sysCheckVMC` checks magic, type 2, exact
   size and whole MiB; it does not parse the FAT or directory. A zero-filled file fails the
   magic and OPL will offer to create a card via genvmc. Our manager should still write a
   fully formatted card so the console does not have to, and so save browsing works.
5. **VMC sizes.** OPL's menu offers 8/16/32/64 MiB; `sysCheckVMC` accepts any whole-MiB size.
   We offer the four UI sizes and reject others unless explicitly allowed.
6. **`games.bin` old generation.** 188-byte records are current (2017+); 92-byte records are
   from 1.0.0/1.1.0/old betas. Detect by the extension offset and delete rather than rewrite
   when unsure.
7. **Config write semantics.** OPL's `configWrite` flattens prefixes and drops comments/`#`
   keys, writing only `key=value\r\n`. If we want to preserve comments/prefixes, keep the
   original bytes and only rewrite when edited; after an edit, match OPL's output for changed
   keys while preserving untouched lines where possible. yaoplm's `CfgDocument` already does
   the byte-preserving approach; keep it.
8. **Path buffers.** OPL uses fixed buffers: 256 for most paths, 64 for `CHT/` and the `APPS`
   root, 128 for an app folder/path, 32 for `ElfPath`, 32 for VMC names, 32 for SMB
   prefix/share/user/pass. Warn or refuse names that would overflow these on the console side
   (relative to the OPL device prefix, not the host mount path).
9. **`AltStartup` length.** `ElfPath[32]` with `cdrom0:\<name>;1` means the filename part must
   be <= 21 chars. Warn if longer.
10. **`BOOT2` parser quirks.** `CNFGetToken` matches `BOOT2` as a prefix; `BOOT2X` then fails
    at `=`. `CNFGetKey` copies printable chars only; a non-printable byte stops the value but
    the loop may spin to 64. Our parser should mirror OPL for scanning and warn on odd cases;
    do not "fix" them silently.
11. **Non-ASCII `ul` titles are dangerous.** OPL's `USBA_crc32` uses a signed `char` index;
    bytes >= 0x80 produce undefined behaviour. Enforce 7-bit ASCII for `ul` titles. yaoplm
    already does.
12. **`iso2opl` is not authoritative for part naming.** Use OPL's `%02x` reader, not
    iso2opl's `%02d`, even though they agree for 0-9.

### A.3 What would break OPL (guardrails)

| Never do this | Why it breaks OPL | Guardrail |
|---|---|---|
| Write a partial or non-64-byte `ul.cfg` record | OPL reads `size / 64` and parses garbage | Validate `size % 64 == 0`; atomic rewrite |
| Write chunk names with uppercase part hex | OPL builds lowercase names; case-sensitive SMB may not find them | Use `%02x` |
| Write chunks that are not exactly 1 GiB except the last | OPL maps LSN by 0x80000 sectors | Enforce exact sizes; verify before writing `ul.cfg` |
| Write more than 10 parts on SMB | `FIDs[ISO_MAX_PARTS]` is 10 | Enforce 10; warn/refuse |
| Write `ul.cfg` with missing or wrong-sized chunks | Game fails to boot | Verify every part exists and matches before writing the record |
| Write a non-ASCII `ul` title | `USBA_crc32` indexes the table with a negative value | Enforce 7-bit ASCII |
| Write an ISO name OPL does not recognize | OPL skips or hides the file | Match `isValidIsoName`; ASCII; <= 159 bytes on SMB |
| Write an image without `BOOT2` | OPL silently hides it | Detect and warn/refuse unless the user overrides |
| Leave stale `games.bin` | OPL uses the old ID for a replaced image | Delete after changing `CD/` or `DVD/` |
| Write CFG with `#` keys, key > 31, or value > 255 | OPL ignores or truncates them | Validate; never write `#` keys |
| Flatten or drop unknown CFG keys | User settings are lost | Preserve unknown keys and order; only touch changed keys |
| Write ART as JPG/BMP or > 1024 px | OPL 1.2 ignores or rejects it | PNG only; <= 1024; correct suffix |
| Write a zero-filled or wrong-size VMC | OPL rejects it and may recreate it | Write a formatted card; exact MiB; contiguous on BDM |
| Write encrypted CHT codes | OPL treats them as names or drops them | Raw 8+8 hex; mastercode names; <= 250 codes |
| Write `title.cfg` without `title`/`boot` or with a path in `boot` | App is hidden | Validate; `boot` is a file name inside the folder; limits |
| Delete a shared VMC or CFG/ART used by another game | Data loss | Check references; never delete VMC automatically |
| Rename without updating CFG/ART/CHT/VMC references | Settings/art are lost | Move by ID; for UL, rename chunks and rebuild `ul.cfg` |
| Change media without moving the folder or updating the UL media byte | Wrong media type and read speed | Update the folder and `ul.cfg` together |
| Write paths that overflow OPL's buffers | OPL truncates the path | Check relative path lengths; warn |
| Write while OPL is reading | OPL sees an inconsistent folder | Atomic writes; tell the user to refresh/reboot; no locking is possible |

### A.4 Verification tests to keep and add

- Keep yaoplm's `crates/yaoplm-formats/tests/opl_oracles.rs`. It extracts `USBA_crc32` from
  OPL source at test time and compares 5000 random names against OPL's C function.
- Run oracle tests in CI with a pinned OPL checkout (`YAOPLM_OPL_SRC`) and a C compiler:
  `cargo test -p yaoplm-formats --test opl_oracles -- --ignored`.
- Add oracle tests for:
  - `SYSTEM.CNF` parsing against `ps2cnfGetBootFile()` + `GetStartupExecName()`.
  - `isValidIsoName()` classification for old/new names, lengths and extensions.
  - `configReadFileBuffer()` / `configWrite()` for comments, prefixes, duplicates and CRLF.
  - `ul.cfg` read/write against `sbReadList()` / `sbRebuildULCfg()`.
  - ZSO round-trip against `pc/ziso.py` (Python + `lz4`).
  - VMC bytes against genvmc for 8/16/32/64 MiB, or at least superblock/FAT/root checks.
- Add a read-only scan test against a real OPL folder produced by OPL Manager, if a legally
  distributable fixture can be obtained. Never commit commercial games.
- Add a "fault injection" test that kills a write mid-way and verifies the journal restores
  the previous `ul.cfg`/CFG state.

### A.5 Verified `USBA_crc32` vectors

Computed with a direct Python port of OPL's `src/system.c` and cross-checked against
yaoplm's known vectors (which are checked against OPL's C function by the oracle test):

| Title | CRC32 |
|---|---|
| `""` | `00000000` |
| `A` | `935FAD90` |
| `AG` | `017A9F71` |
| `Grand Theft Auto III` | `10D00014` |
| `Metal Gear Solid 2 Substance` | `CE32D6E8` |
| `Test Game` | `1C9B6AE0` |
| `Z` x 32 | `C5B91150` |
| `Z` x 33 | `92A46C81` |
| `f` | `8433E5CC` |
| `F` | `A490D3EA` |
| `f` + `o` x 31 | `84BA9D95` |
| `F` + `O` x 31 | `8CAF0142` |
