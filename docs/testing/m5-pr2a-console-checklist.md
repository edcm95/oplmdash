# M5, PR 2a — checklist: all picture types, picture sets, covers and dump verification when adding

The milestone M5's second PR, part "a" (branch `m5/add-art-and-checks`: ART-2, ART-3, ART-5, ART-8, GAME-5 when adding, titles from the database META-1). The owner checks it on their PS2. Tick items as you go; report issues in separate small PRs. General test-data rules — in [TESTING.md](../TESTING.md).

Intended for stick E: (FAT32, volume serial `7E1CD2C5`), OPL 1.2.0 beta and OPL 1.1.0 from the FreeMcBoot card. Every write to E: goes through a plan; confirm only those. The internet is needed (covers, Redump DAT).

## Setup

- [ ] 1. Run `pnpm tauri dev` from `apps/desktop`. "Settings → Library": profile `OPL 1.2.0 beta`, names `ID.Title.iso`. In "Settings" "Download covers for added games", "Titles from the database when adding games" and "Verify dumps when adding games" are enabled (this is the default).
- [ ] 2. The Redump DAT is downloaded ("Settings → Dump verification (Redump)"; done in the M5 PR 1 checklist, item 5).
- [ ] 3. The test files are already in `D:\yaoplm-testdata\out\m5-pr2a\` (prepared by the agent, everything generated, nothing commercial):
   - `cover test.iso` — wLaunchELF with ID `SLUS_200.62`. The ID is taken from the built-in database ("Grand Theft Auto III") so that a title and cover are found for it; inside there is only wLaunchELF;
   - `sets\OPLM art test.zip` — a set in the OPL Manager backup layout: `PS2/SLUS_200.62/SLUS_200.62_ICO.png`, `_COV2.jpg`, `_LAB.jpg`, `_LGO.png`, `_SCR_00.jpg`, `_SCR_01.jpg`, `_SCR_02.jpg` and one picture for the game `SLES_888.88`, which is not on the stick. Each picture is a coloured card with its type name, so that the screen shows what went where;
   - `sets\art.tar` — `SLUS_200.62_BG.png` (background).
- [ ] 4. Optional: your own game on CD in Redump format (`.cue` and `.bin`, without reconversion). Only with it can you see "Verified" by the data track.

## Adding with a cover, title and verification (on the PC)

- [ ] 5. "Add games" → "Add files…" → `cover test.iso` → "Show plan". The plan has the title "Grand Theft Auto III" and the note "In the built-in database SLUS_200.62 is called "Grand Theft Auto III"". Under the plan "Searching covers for added games…" runs, the confirm button is unavailable while the plan is updating. Then a "… cover" transaction appears in the plan with `ART/SLUS_200.62_COV.png`.
- [ ] 6. Turn off "Titles from the database" on the add screen: the title in the plan becomes "cover test". Turn it back on.
- [ ] 7. "Confirm and execute". In the job log the game and its cover; the "Dump verification: 1" block with the verdict "Bad dump". This is correct: the ID belongs to a real game in Redump, and inside is wLaunchELF.
- [ ] 8. The game card → "Pictures" section: eight slots, only "Cover" is filled. For the icon, back cover, spine and background the "Download" button is unavailable (there are no open sources). For "Screenshot", "Second screenshot" and "Logo" click "Download": the dialog shows variants from libretro-thumbnails, already resized (or "No source has such a picture."). "Choose this one" → plan → confirm. The logo is not stretched: the text keeps its proportions.
- [ ] 9. "Download" for the "Cover" slot: variants — the flat xlenore cover, the 3-D xlenore cover, the libretro box (whatever is found). "Choose this one" on the 3-D → replacement plan → confirm; the old cover goes to `E:\.yaoplm\backup\`. Then restore the flat one the same way.

## Importing picture sets (on the PC)

- [ ] 10. Library → "Import pictures" → "Add archives…" → `OPLM art test.zip` and `art.tar` (you can drag them into the window). All types are ticked, "Replace pictures already on the device" is off → "Read sets".
- [ ] 11. The plan: icon, back cover, spine, logo, screenshot (from `SCR_00`), second screenshot (from `SCR_01`), background (from `art.tar`); `SCR_02` is not used. Notes "Pictures already on the device left as is: N" (the pictures from item 8) and "Pictures skipped for games not on the device: 1". Confirm.
- [ ] 12. The game card: all eight slots are filled with coloured cards labelled by type.
- [ ] 13. Repeat the import with "Replace pictures…" enabled: the plan replaces the pictures from item 8 too, and the old ones go to `E:\.yaoplm\backup\`. Confirm.

## On the console

- [ ] 14. OPL 1.2.0 beta, USB, "Cover Art" enabled in the OPL settings. In the list "Grand Theft Auto III" with a cover and a yellow "ICO" icon. Check whether the icon looks sharp: the standard theme draws it at 128×128, while YAOPLM writes 64×64, like all managers. If the icon is noticeably blurry, write to us — then for OPL 1.2 we will write 128×128.
- [ ] 15. Square on this game → the details page: the "BG" background and two screenshots "SCR" and "SCR2". The back cover, spine and logo are not shown in the standard theme (only in themes that use them).
- [ ] 16. Cross: wLaunchELF launches.
- [ ] 17. OPL 1.1.0: the cover and icon are visible.

## Checking a CD by the data track

- [ ] 18. Add `D:\yaoplm-testdata\out\m5-packed\YAOP_000.01.wLaunchELF.cue` (a CD with an audio track). The log says "Dump verification: 1", the verdict "Unknown: Not in Redump". In the card → "Verify dump" → "Hashes": there is a "Data track of the added dump" block; the image SHA-1 is the same as in the M5 PR 1 checklist, item 11.
- [ ] 19. Optional, your own game (item 4): add its `.cue`. Expected "Verified" and the explanation "Checked by the data track of the BIN or CHD the game was added from". Then "Refresh" the library: the verdict stays. If "Bad dump" or "Unknown" — send the game title and region and how many `.bin` files it has.
- [ ] 20. Turn off "Verify dumps when adding games" and add any game: the job log has no "Dump verification" block.

## Cleanup

- [ ] 21. Delete "Grand Theft Auto III" and `YAOP_000.01` together with related files (pictures go with the game) — a deletion plan with a separate confirmation.

## What to report

A screen photo of OPL for items 14, 15 and 17; plan screenshots from items 5, 11 and 13; the result of item 19 if you checked your own game; the text of any error from the plan or job dialog.
