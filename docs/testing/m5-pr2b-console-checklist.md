# M5, PR 2b — checklist: game settings and memory cards (VMC)

The milestone M5's second PR, part "b" (branch `m5/cfg-vmc`: CFG-2, VMC-1, ADR-0036). Closes items 2 and 3 of the M5 check in ROADMAP: "compatibility modes are applied" and "VMC works: a save is created and read". The owner checks it on their PS2. Tick items as you go; report issues in separate small PRs. General test-data rules — in [TESTING.md](../TESTING.md).

Intended for stick E: (FAT32, volume serial `7E1CD2C5`), OPL 1.2.0 beta and OPL 1.1.0 from the FreeMcBoot card. Every write to E: goes through a plan; confirm only those. The internet is not needed.

**Be careful with the memory card.** In wLaunchELF launched from OPL as a game, write nothing to `mc0:` and `mc1:`: it is unknown whether OPL substitutes a VMC for the card for it, and a write could land on the real card with FreeMcBoot. Saves are checked on your own game (item 15).

## Setup

- [ ] 1. Run `pnpm tauri dev` from `apps/desktop` (or the program built from the branch). "Settings": profile `OPL 1.2.0 beta`, "Simple mode" on (the default).
- [ ] 2. The test files are already in `D:\yaoplm-testdata\out\m5-pr2b\` (prepared by the agent, everything generated, nothing commercial; the recipe is `README.txt` there):
  - `YAOP_000.08.CFG test.iso` and `YAOP_000.09.GSM test.iso` — wLaunchELF with IDs `YAOP_000.08` and `YAOP_000.09`;
  - `CFG\YAOP_000.08.cfg` — a "foreign" program's CFG: the keys `Title`, `Genre`, `Description`, which OPL does not write itself, and `$Compatibility=1` (mode 1).
- [ ] 3. "Devices" → E: → "Open library" → "Add games" → both images → plan → confirm. "CFG test" and "GSM test" appeared in the library.
- [ ] 4. Copy `CFG\YAOP_000.08.cfg` to `E:\CFG\` with the file manager (this is the "foreign" file that YAOPLM must edit carefully). In YAOPLM "Refresh": the "CFG test" card in "Game settings" shows "Mode 1" ticked.

## Simple mode (on the PC)

- [ ] 5. The "CFG test" card → "Game settings": modes 1–6 with explanations and "VMC slot 1" are visible; slot 2, GSM, cheats and PADEMU are not. Untick "Mode 1", tick "Mode 6: disable IGR" → "Save…". The plan: "Changes in CFG: `$Compatibility: 1 → 32`, `$ConfigSource: (none) → 1`", nothing more. "Confirm and execute".
- [ ] 6. "VMC slot 1" → "Create": the list shows "YAOP_000.08_0 (new, 8 MiB)". "Save…" → the plan: "`$VMC_0: (none) → YAOP_000.08_0`". Confirm. The "done" notice without warnings. If the warning "… lies on the device not in one piece" appears — write down its text: OPL will not take such a card.
- [ ] 7. File manager: `E:\VMC\YAOP_000.08_0.bin` is exactly 8,388,608 bytes. `E:\CFG\YAOP_000.08.cfg` in Notepad: `Title`, `Genre`, `Description` in place, below `$Compatibility=32`, `$ConfigSource=1`, `$VMC_0=YAOP_000.08_0`. `E:\.yaoplm\backup\` holds copies of the CFG from before the edits.

## Advanced mode (on the PC)

- [ ] 8. "Settings" → turn off "Simple mode". The "GSM test" card → "Game settings":
  - "GSM: video mode" → "Custom for this game" → "Enable custom video mode" → a video mode your TV definitely shows, but not the one the console runs in: for a PAL console "PAL 60 Hz", for an NTSC console "NTSC, non-interlaced";
  - "Cheats" → "Custom for this game" → "Cheat engine" → "All codes".
  "Save…": the plan has `$GSMSource`, `$EnableGSM`, `$GSMVMode`, `$CheatsSource`, `$EnableCheat`, `$ConfigSource`. Confirm.
- [ ] 9. "VMC slot 2" on "GSM test" → "Create…" → name `generic_1`, size 16 MiB → "Take the new card" → "Save…" → confirm. `E:\VMC\generic_1.bin` — 16,777,216 bytes.

## On the console: OPL 1.2.0 beta, USB

- [ ] 10. "CFG test" → Triangle → game menu → compatibility settings: only mode 6 is ticked, the text "Customized Settings" is present (this is `$ConfigSource=1`).
- [ ] 11. There "Configure VMC": "VMC Slot 1" — `YAOP_000.08_0`, state "VMC file exists", size 8 Mb. **Do not change anything and do not press Create/Modify** (OPL would recreate the card); go back.
- [ ] 12. Square on "CFG test" → the details page: "YAOPLM CFG test" and the genre "Homebrew" (if the theme shows them) — the keys YAOPLM did not touch are in place.
- [ ] 13. Cross on "CFG test": **no** VMC questions ("Error accessing VMC…" or "VMC … is fragmented…"), wLaunchELF launches. Press L1+L2+R1+R2+Start+Select: it does **not** return to OPL (mode 6 disabled IGR). Restart the console with the button.
- [ ] 14. Cross on "GSM test": before launching OPL says "No cheats found." (cheats are enabled but there is no cheat file — this is expected), then wLaunchELF in a different video mode (GSM). L1+L2+R1+R2+Start+Select returns to OPL. If the screen goes black — restart the console with the button and write which mode you chose.

## Save on your own game

- [ ] 15. On the PC: your GTA San Andreas card (`SLUS_209.46`, ul parts on E:) → "VMC slot 1" → "Create" → "Save…" → confirm (card `SLUS_209.46_0`, 8 MiB). On the console launch the game: it sees the memory card without offering to format it. Save, exit, turn off the console. Turn on, launch again — load the save. On the PC "Game settings" → the slot list shows `SLUS_209.46_0 (8 MiB)`; `yaoplm vmc ls E:\` (optional) shows "1 saves".

## OPL 1.1.0

- [ ] 16. "CFG test": in the settings mode 6 is ticked, in "Configure VMC" slot 1 is `YAOP_000.08_0`; launching without VMC questions.

## After OPL

- [ ] 17. In OPL 1.2.0 on "CFG test" tick mode 2 as well and save ("Save Changes"). On the PC "Refresh": modes 2 and 6 are ticked on the card; the CFG still has `Title`, `Genre`, `Description` (YAOPLM reads the file that OPL wrote).

## Cleanup

- [ ] 18. Delete "CFG test" and "GSM test" together with related files (a deletion plan with a separate confirmation). YAOPLM never deletes VMC cards (ADR-0020): delete `E:\VMC\YAOP_000.08_0.bin` and `E:\VMC\generic_1.bin` with the file manager. Leave the `SLUS_209.46_0` card if the save is needed.

## What to report

A screen photo of OPL for items 10–14 and 16; the result of item 15 (did the save load); plan screenshots from items 5, 6 and 8; the text of any warning or error from the plan, a notice or OPL.
