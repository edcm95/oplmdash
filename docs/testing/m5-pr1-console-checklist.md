# M5, PR 1 — checklist: dump verification, conversions, import from archives and CHD

The milestone M5's first PR (branch `m5/dumps-and-conversions`: GAME-4, GAME-5, META-2, IMP-1b). The owner checks it on their PS2. Tick items as you go; report issues in separate small PRs. General test-data rules — in [TESTING.md](../TESTING.md).

Intended for stick E: (FAT32, volume serial `7E1CD2C5`), OPL 1.2.0 beta and OPL 1.1.0 from the FreeMcBoot card. The bootable test disc — wLaunchELF (`YAOP_000.01`), packed in every way from IMP-1b, so the whole "archive or CHD → stick → launch" path is checked without commercial images. Every write to E: goes through a plan; confirm only those.

## Setup

- [ ] 1. Run `pnpm tauri dev` from `apps/desktop` (there is no release build for M5 yet). "Settings → Library": profile `OPL 1.2.0 beta`, names `ID.Title.iso`.
- [ ] 2. Test files on D: (prepared by the agent; if the folder is missing — the commands below):
   - `D:\yaoplm-testdata\out\m5-packed\`: `YAOP_000.01.wLaunchELF.zip`, `.7z`, `(DVD).chd`, `(CD).chd` (with an audio track), `.cue` with `(Track 1).bin` and `(Track 2).bin`. Command: `cargo run -p yaoplm-test-support --example pack -- D:\yaoplm-testdata\out\YAOP_000.01.wLaunchELF.iso D:\yaoplm-testdata\out\m5-packed`;
   - `D:\yaoplm-testdata\out\YAOP_000.05.wLaunchELF 4300MiB.iso` (larger than 4 GiB, boots).
- [ ] 3. Optional, for Redump verification: your own archive with a game (zip or 7z with an ISO inside), downloaded earlier.
- [ ] 4. "Devices" → the E: card → "Open library". If `YAOP_000.01` is already on the stick, delete that game (its ID is needed below).

## Dump verification (on the PC)

- [ ] 5. "Settings → Dump verification (Redump)" → "Download from Redump" (the internet is needed). Expected: "Downloaded, records: …" (about 11,800) and the row "Redump DAT <date>, records: …". The "Check for update" button becomes inactive until a week from now.
- [ ] 6. Library → "Verify dumps" → "All games" → "Verify". In the log one line per game; fixtures and wLaunchELF are "unknown" with the reason "Not in Redump" (it is homebrew). A shield icon appears on the cards in the library.
- [ ] 7. "Verify dumps" → "All games" again: verification is almost instant, the rows say "Hashes from cache".
- [ ] 8. If you have your own game (item 3): import it (item 9), then "Verify dump" in the game card. Expected "Verified" and the Redump title with a link. "Bad dump" or "unknown" — send the game title and region.

## Import from archives and CHD

- [ ] 9. "Add games" → "Add files…": the dialog filter shows `.chd`, `.zip`, `.7z`. Select `m5-packed\YAOP_000.01.wLaunchELF.7z` → "Show plan". Expected: the entry `CD/YAOP_000.01.wLaunchELF.iso` with the note "Read directly from the archive, without unpacking to the computer". "Write to device".
- [ ] 10. Console, OPL 1.2.0 beta, USB: wLaunchELF is in the list and launches.
- [ ] 11. PC: the game card → "Verify dump" → "Hashes": note the SHA-1. Then "Delete" → "Show delete plan" → confirm.
- [ ] 12. Repeat items 9 and 11 one by one for `(CD).chd`, `.zip`, `.cue` and `(DVD).chd`, deleting the game after each. Each SHA-1 is the same as in item 11. For `(CD).chd` and `.cue` the plan warns "Audio tracks are not carried over…".
- [ ] 13. Enable "Move instead of copy" and add the `.7z`: the plan has the note "The archive the game was taken from is not deleted"; after the write the `.7z` remains in the folder.
- [ ] 14. Drag the `.zip` from the file manager into the "Add games" window: it appears in the source list and the plan is built.

## Conversions

- [ ] 15. The wLaunchELF card (ISO) → "Convert…" → ZSO → "Show plan". The plan says "Compress to CD/….zso" and deletes the `.iso`; the button is unavailable until "I understand that files will be deleted" is ticked. Confirm. The format in the card is ZSO, the size is smaller.
- [ ] 16. Console, OPL 1.2.0 beta: wLaunchELF launches from ZSO. OPL 1.1.0: the game is not in the list or does not launch (1.1.0 does not read ZSO, this is expected).
- [ ] 17. "Convert…" → ul: on the stick `ul.XXXXXXXX.YAOP_000.01.00` and an entry in `ul.cfg`, the ZSO is deleted. Console (1.2.0 and 1.1.0): it launches.
- [ ] 18. "Convert…" → ISO: again `CD/YAOP_000.01.wLaunchELF.iso`, no ul parts and no `ul.cfg` entry. Console: it launches.
- [ ] 19. Add `YAOP_000.05.wLaunchELF 4300MiB.iso`: on FAT32 the plan splits it into 5 ul parts. Then "Convert…" → ZSO: the plan warns "FAT32 holds files up to 4 GiB…"; the ZSO comes out small (the image is almost all zeros). Console, OPL 1.2.0 beta: it launches from ZSO.
- [ ] 20. "Compress to ZSO" on the add screen, source `.7z`: the plan says "Compress to ….zso", the game is immediately in ZSO. Console: it launches.
- [ ] 21. Library → filter "ZSO" → "Convert…" → ISO: the dialog says "Will be converted: N of N", the plan covers all shown games.

## Cancel and failure (optional)

- [ ] 22. Convert a 4300 MiB ul game to ZSO → "Cancel" in the job halfway: the game remains in ul parts, there is no `*.yaoplm-part` on the stick.
- [ ] 23. The same, but pull the stick out halfway. After reconnecting: the banner "1 operation did not finish" → "Roll back" → the game is in ul parts again, there are no temporary files.

## What to report

A screen photo of OPL for items 10, 16, 17 and 19 with the OPL version; plan screenshots from items 9, 12 (CD CHD), 15 and 19; the output of `dir /a E:\` after item 18; the result of item 8 if you checked your own game; the text of any error from the plan or job dialog.
