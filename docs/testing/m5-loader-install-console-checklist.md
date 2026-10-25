# M5: loader installation — console check (LDR-3, BOOT-1)

What we check: YAOPLM installs official OPL on a stick in one plan — folders, the ELF where the standard launch method will find it, a backup of the previous file, a manifest — and after installation says how to launch OPL on the PS2. The program does not touch configs on the memory card. Details — ADR-0042, ADR-0044, SPEC §5.10.1.

We write only to test stick F: (it can be formatted). Before every write the program shows a plan; you confirm.

## What is needed

- Stick F: (FAT32), connected to the PC, then to the PS2.
- FreeMcBoot 1.966 on the memory card (your FMCB card) and, if available, PS2BBL.
- Network on the PC: the program will download OPL 1.2.0 beta from GitHub (tag `latest`, about 1.4 MB).

## Steps

1. **Wizard.** "Devices" → the F: card → "Install loader…".
   - Where the device will work: "USB port". Launch method: "FreeMcBoot from the memory card" (`UPDT/OPL.ELF`). Build: "OPL 1.2.0 beta" marked "Recommended".
   - Under the builds: version `v1.2.0-Beta-…`, date, size, AFL-3.0 licence, links to the release and commit, "What's new".
   - Advanced mode: tick "OPL menu language: English" and "Starter settings".
2. **Plan.** "Download and install" → the plan: "Create OPL folders", "OPL 1.2.0 beta in UPDT/OPL.ELF", language, "Remember the installed loader", starter settings with a warning about `conf_opl.cfg` on the card. Confirm.
   - On F: `UPDT\OPL.ELF` appeared, the folders `CD`, `DVD`, `CFG`, `ART`, `VMC`, `CHT`, `THM`, `LNG`, `APPS`, in `LNG\` — `lang_English.lng` and the font, `conf_opl.cfg` with `usb_mode=2`, `.yaoplm\manifest.json`.
   - The "Loader installed" dialog: "FreeMcBoot — nothing needs configuring: the standard config launches mass:/UPDT/OPL.ELF" and a reminder to enable USB in the OPL settings.
   - The F: card's "Loader" row names OPL 1.2.0 beta, its version and `UPDT/OPL.ELF` (by the version inside the file).
3. **Console, FreeMcBoot.** Insert F: into the PS2 and launch the OPL item in the FMCB menu.
   - If the memory card already has a `conf_opl.cfg`, OPL will take the settings from there — this is expected; then enable USB manually ("Auto") and save.
   - Without its own settings on the card: USB is enabled automatically, the menu is in English (if the language is not selected in OPL — select it in the interface settings).
4. **"Don't know" and a foreign file.** On the PC put any other ELF on F: (for example wLaunchELF) as `APPS\OPNPS2LD.ELF`. The wizard → "Don't know".
   - The plan is not executed: "APPS/OPNPS2LD.ELF contains another program (size, SHA-256)…". At the bottom "Replace this file (a backup will be kept)".
   - `UPDT/OPL.ELF` already has this build: "this exact build is already there".
   - Tick "Replace…" and confirm: the previous file is in `.yaoplm\backup\loader\<time>\APPS_OPNPS2LD.ELF`, byte for byte.
   - The dialog after installation: FreeMcBoot and PS2BBL, both "nothing needs configuring".
5. **Console, PS2BBL (if available).** Launch PS2BBL with F: inserted — the standard config launches `mass:/APPS/OPNPS2LD.ELF`, that is, OPL.
6. **Three copies.** Repeat step 4 three more times, each time putting another ELF into `APPS\OPNPS2LD.ELF`: `.yaoplm\backup\loader\` keeps the last three copies of this path, and the plan shows the oldest being deleted.
7. **MX4SIO (if you will check it).** SD card in the PC card reader, wizard → "MX4SIO": methods "PS2BBL-MX4SIO or OSDMenu" (`APPS/OPL/OPNPS2LD.ELF` with `title.cfg`) and "OPL already on the memory card" (folders only); no starter settings. After installation — the lines `LK_AUTO_E1 = massX:/APPS/OPL/OPNPS2LD.ELF` for `PS2BBL.INI` and `name_OSDSYS_ITEM_N` / `path1_OSDSYS_ITEM_N` for `OSDMENU.CNF`. If you add the line to the config on the console — does OPL launch from MX4SIO.
8. **CLI (optional).** `yaoplm loader install F:\ opl-beta --preset ps2bbl` prints the plan, and after "y" — the launch lines.

## What to report

- For each step: "as expected" or what happened instead (a photo of the PS2 screen or a screenshot).
- Whether OPL launched from FMCB (step 3) and from PS2BBL (step 5), and whether it sees USB without manual enabling.
- The contents of `F:\.yaoplm\manifest.json` after step 4.
