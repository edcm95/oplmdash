# M5: automatic loader profile detection — check (LIB-1)

What we check: YAOPLM understands on its own which OPL a device is prepared for, shows it on the device card and in the library header, and the "for this device" choice is remembered and applies to everything (scanning, adding, conversions, game settings). Details — ADR-0039, research/16, SPEC §5.1.1.

The console is needed only in step 5. The rest is checked in the program: it writes nothing to the device except what you add yourself in the steps below.

## What is needed

- Stick E: (or any test FAT32).
- The official OPL 1.1.0 archive from GitHub (`OPNPS2LD-v1.1.0.7z`, release v1.1.0) — for step 3. It is already downloaded to `D:\yaoplm-testdata\downloads\opl-v1.1.0\`.

## Steps

1. **Stick without a loader** (as E: is now: OPL launches from the memory card).
   - "Devices": on the E: card the row "Loader: OPL 1.2.0 beta — no signs of a loader on the device".
   - The same in the library header next to the game count, in simple and in advanced mode.
   - `yaoplm device E:\` → `Loader: OPL 1.2.0 beta: no signs of a loader on the device (auto)`.
2. **Choice for the device.** In the library header "Change" → "OPL 1.1.0".
   - The row: "OPL 1.1.0 — chosen for this device"; the advice on the device card — for 1.1.0.
   - Unplug and plug the stick back in (possibly under another letter): the choice is saved, it is bound to the volume serial number.
   - "Change" → "Auto (by device)" restores detection.
3. **OPL 1.1.0 on the stick.** Put `OPNPS2LD-v1.1.0.ELF` from the archive on the stick as `UPDT\OPL.ELF` (the FMCB 1.966 path).
   - The row: "OPL 1.1.0 — OPL v1.1.0 in UPDT/OPL.ELF, identified by checksum".
   - If the stick has ZSO, there is a warning below that OPL 1.1.0 does not see them.
4. **Traces of OPL 1.2.** Launch any OPL 1.2 beta (or YAOPL) from this stick and let it open the game list — it will write `CD/games.bin` / `DVD/games.bin` with 188-byte records.
   - The profile stays by checksum (OPL 1.1.0), but a notice appears: "The games on this device were last listed by an OPL 1.2 beta from build 2017 or a fork…".
   - Delete `UPDT\OPL.ELF`: now "OPL 1.2.0 beta — DVD/games.bin was last written by a beta from build 2017 or an OPL 1.2 fork".
5. **Console (optional).** OPL 1.1.0 from step 3, launched from the stick, must show the games. ZSO is not shown by 1.1.0: the program warned about this in step 3.
6. **Clean up.** Delete `UPDT\OPL.ELF` if it is still there and return "Auto".

## What to report

- For each step: "as expected" or what happened instead (a photo or screenshot is fine).
- The output of `yaoplm device E:\` after steps 3 and 4.
