# Pre-release interface consistency check

The product owner's decision of 2026-10-25. Before every release tag (`v0.N.0`) and after major changes to the device card, the library header or the storage doctor (DEV-2), the interface goes through one end-to-end check: everything the program says and offers about one device and one library must add up to one consistent story. Individual PRs test their own feature; contradictions between features are visible only during such a walkthrough.

## Why this is a separate task

Examples from the 2026-10-25 session (profile detection, PR #38 and #40):
- **Contradictory signals on one card.** A folder on the system disk simultaneously got "Not suitable for OPL", "An operating system is installed on this volume" and the question "Which OPL do you launch from this device?". Each part was correct by its own logic; together — nonsense. Fixed in PR #40: the question is hidden if the doctor considers the device unsuitable. The root of the problem (folder mode for SMB) is not solved — see "Open questions".
- **A global setting that silently overrides the device.** An explicit profile in settings disabled file-based detection on all devices, while the interface showed "by device" as the default. Removed by ADR-0040.
- **A hidden accessibility defect.** The contrast of `kbd` on the settings page had been below WCAG AA for a long time, but axe did not see it until the element entered the visible part of the page. The profile row disappeared, the "Keyboard" card moved up, and the E2E failed in CI.
- **The fixture does not show part of the interface.** The E2E library lives in a temporary folder on the system disk; the doctor considers it unsuitable, and the profile question in E2E on Windows is not visible at all, while on Linux CI it is. The behaviour had to be checked with component tests, and the E2E adjusted to the verdict.

## When

1. Before a release tag, after feature freeze: a full walkthrough of the rules and matrix below.
2. After PRs that change the device card, the library header, the storage doctor, the "What next" hints (UI-6), profiles and questions about them, or dialogs when a device is connected: a walkthrough of the affected matrix rows.

## Rules (what must hold)

Each rule is checked on all matrix rows. A violation is a finding (see "Walkthrough procedure").

1. **One story per device.** The doctor's verdict, the "Loader" row, the "What next" hint, questions, notices and buttons do not contradict each other. If a device is "Not suitable for OPL", it is not offered anything that only makes sense for a suitable one: installing and updating a loader, the OPL version question, "Download pictures" as the next step.
2. **An action only where it is possible.** Install, update, replace and format buttons — per SPEC §5.10.1 and §5.11.2 tables. No buttons that lead to a refusal "not allowed on this device".
3. **One main button per state** (UI-6). Priority order when several fit: unfinished write (journal) → unsuitable device → setup or install → profile question → add games → pictures → game warnings. The profile question and "Install OPL…" are not shown simultaneously as the first two steps.
4. **Why is visible.** Every value the program chose itself (profile, disc type, file name, FS during setup) shows the reason ("by …"). The user's choice is visible as a choice ("chosen for this device", "version unknown"), reversible with one action ("Return to "Auto"") and not overridden by a hidden setting (ADR-0040).
5. **The same everywhere.** The device card, the library header, the write plan, the write result and `yaoplm device` name one profile with one reason. Request caches account for the user's choice (keys with `overrides`) and are refreshed after a write: for example, YAOPLM deletes `games.bin` after changes, and the OPL generation sign disappears until the next view on the console.
6. **Notices without duplicates and with the right severity.** One notice is not shown twice on one screen. The level (error, warning, information) matches the consequences. Simple mode shows only what explains the verdict and the next step. The texts of two notices do not argue (for example, "the games were last listed by 1.2" next to a user-selected 1.1.0 — appropriate, but the wording must not sound like a program error).
7. **Dialogs only when needed.** Modal dialogs — write confirmation and explicit user actions. When a device is connected there are no dialogs, except "Update available" per SPEC §5.10.1 with "Not now". Questions — as a row on the card, not as a dialog (ADR-0041).
8. **Device mode.** A stick or SD card for PS2, a folder OPL reads over the network (SMB), server roots (Docker) — verdicts, questions and hints match how OPL will read this device. Currently this rule is not met for folders on the computer's disk (see "Open questions").
9. **Texts.** The glossary `docs/I18N.md`: one term per concept on all screens (device, stick, folder, volume). Typography, no keys that are no longer used anywhere. Core phrases render from the English catalogue without a reload.
10. **Accessibility.** axe without WCAG 2.2 AA violations on every screen and in every state: both modes, both themes, open menus and dialogs, questions and warnings on cards, scrolled areas. A layout shift opens elements that axe did not check before. Keyboard navigation: every question and every button is reachable with Tab and works with Enter.
11. **Layout.** Long paths and titles, a narrow window (1024 px), the "Loader" row wrapping together with its neighbours in the library header, dropdown menus over warnings, the dark theme, 3000+ games.
12. **Loading and errors.** Until a device has responded, the interface does not show conclusions as final (`ready=false`). A read error does not turn into "no signs". Scanning waits for an active profile.
13. **Speed.** Loader detection reads the whole ELF (up to 64 MiB) and unpacks LZMA: a list of five or more devices, a slow stick and a network folder do not freeze the screen; there is a waiting indicator.
14. **Desktop, browser, server.** Differences that exist by design (formatting only on the desktop, the server shows only its own roots, no folder selection dialog in the browser) are explained by a hint, not by a refusal without a reason.

## State matrix

Rows that need to be seen with your own eyes (English screenshots, light and dark theme, simple and advanced mode):

| Device | Loader and traces | Library | Profile |
|---|---|---|---|
| FAT32 stick (test F:) | none | empty | default |
| FAT32 stick | none, OPL on the memory card | games, `games.bin` 1.1.0 and 1.2 | by `games.bin`; a question if there is no `games.bin` |
| FAT32 stick | ELF 1.1.0, 1.2, YAOPL, RiptOPL, wOPL, NHDDL | games | by hash or the version in the ELF |
| FAT32 stick | ELF + leftovers of another fork | games | by the ELF + a notice about the leftovers |
| FAT32 stick | corrupted ELF | games | by name + a notice |
| FAT32 stick | two ELF files of different generations | games | below the exact level + "several loaders" |
| exFAT stick | none and ELF 1.1.0 | games with ZSO | not 1.1.0; a conflict with ELF 1.1.0 as a device problem |
| NTFS stick, GPT without partitions, sector ≠ 512, read-only | any | any | "Not suitable" + "Prepare the device…" without version questions |
| SD in MX4SIO; emulator SD card (MMCE) | as on a stick; the card's service folders | games | as on a stick; service folders are not touched |
| folder on the system disk | none | games | see "Open questions" |
| folder on a non-system disk (for SMB) | none | games | see "Open questions" |
| network folder mounted by the OS | none | games | SMB advice |
| server root (Docker) | none | games | as for an SMB folder |

For each row additionally: the user's choice matches the files / contradicts them / "Don't know"; an unfinished write (journal); after a write (`games.bin` deleted).

## Fixtures and tests

- The E2E fixture `make_library` lives on the system disk, so everything that depends on the "suitable" verdict is not visible in E2E on Windows. Before the walkthrough a "removable device" fixture is needed: for example, a server environment variable that marks the root as an external FAT32 volume, or a separate FAT32 image. Until then, states are checked by component tests (`LoaderProfileHint.test.tsx` — an example) and manually on the test stick.
- Screenshots of each matrix row go to the `pr-assets` branch (`release-<version>/`), as for PRs.
- Known flaky tests (as of 2026-10-25): `ConnectionStatus.test.tsx` sometimes times out under load; the CLI test `devices_and_device_doctor` once failed during a full run with a shared `target/`. Before the walkthrough they need to be run several times and fixed, not restarted until green.
- Real loader builds (`scripts/loader-corpus.py`, the `loader_elfs` and `loader_corpus` tests) — refresh the sample before the walkthrough so the cards show current versions.

## Walkthrough procedure

1. Update the clones and the loader sample; build the server and desktop from the release branch.
2. Prepare the matrix devices on test stick F: (formatting only through a plan), the SMB folder and the server root.
3. Walk the matrix rows in simple and advanced mode, English, light and dark theme; take screenshots; run axe on every screen.
4. Record each rule violation in `docs/plans/release-<version>-ui-findings.md`: rule, matrix row, screenshot, proposal. Fix with separate small PRs; decisions that change SPEC — through an ADR.
5. Repeat E2E, axe and the walkthrough of the fixed rows.
6. Update the SPEC §5.11.2 hint tables and SPEC §5.10.1 state tables if the behaviour changed.

## Open questions (as of 2026-10-25)

1. **"Device for PS2" and "folder for SMB" modes** (product owner, 2026-10-25). Currently a folder on the computer's disk is evaluated as a volume for connecting to a PS2: on the system disk — "Not suitable for OPL", although OPL can read it over the network. Need to decide:
   - how to determine the mode: a removable volume — a device for PS2; a folder on the internal disk and a network folder — for SMB; server roots — for SMB; whether it can be switched manually and where that is remembered (like the profile choice, by device key);
   - what changes in the doctor in SMB mode: no "system disk" error (writing still only inside the selected folder); instead of FS advice — SMB rules (SMBv1, names up to 159 bytes, no more than 10 `ul` parts, no fragment limit and no 4 GiB limit on NTFS);
   - the profile question is needed in SMB mode: the OPL version on the console cannot be learned from a folder, except from `games.bin` and `conf_opl.cfg`;
   - what to show on the card of the system disk opened as a whole.
2. **The profile question and the beginner's path.** In SPEC §5.11.2 the state "OPL folders exist, no loader" has an "OPL on the memory card" button. The version question (ADR-0041) should become its continuation, not a separate block next to the "What next" hint.
