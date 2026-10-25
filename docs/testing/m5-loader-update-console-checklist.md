# M5: loader update and rollback — check (LDR-4)

What we check: the device card knows which OPL build is on the stick, says "Up to date" or "Update: …" without contacting the network, the "Update available" dialog appears once per device and build, "Not now" is remembered, an update replaces only the ELF, and "Return to …" restores the previous build. Details — ADR-0045, SPEC §5.10.1.

We write only to test stick F: (it already has OPL 1.2.0 beta `v1.2.0-Beta-2271-b3a2199` installed by YAOPLM). Before every write the program shows a plan; you confirm.

## What is needed

- Stick F: with OPL from the previous check (`UPDT\OPL.ELF` and `.yaoplm\manifest.json`).
- Network on the PC — only for step 2.
- Step 4 needs a newer beta than 2271. If upstream has not released a new one yet, this step can be done with YAOPL or skipped: then only "Up to date" is checked.

## Steps

1. **Card without network.** "Devices" → the F: card.
   - The loader row: "OPL 1.2.0 beta v1.2.0-Beta-2271 (b3a2199) UPDT/OPL.ELF".
   - If the program has already read the release (after installation from the previous check), "Up to date" and "Last check: …" are next to it. Without a network nothing changes.
   - `yaoplm device F:\` → the row `Installed:  opl-beta v1.2.0-Beta-2271-b3a2199 at UPDT/OPL.ELF (installed by YAOPLM); …`.
2. **Check for updates.** The button on the card. If there is no new beta — "This is the newest build in its channel". If there is — "Update available" opens by itself:
   - old → new version, date, size, changelog;
   - links "Release page", "Commit", "What changed since your version" (a commit comparison on GitHub).
3. **"Not now".** Close the dialog with "Not now", refresh the list with the button at the top right and restart the program: the dialog no longer appears. The card keeps the "Update: …" icon and the "Update to …" button.
4. **Update.** "Update to …" → "Update" → the plan "Update OPL 1.2.0 beta to …": only `UPDT/OPL.ELF` is replaced, the previous file goes to `.yaoplm\backup\loader\<time>\UPDT_OPL.ELF`. Folders, `conf_opl.cfg` and games are not touched. Confirm.
   - The card: the new version and "Up to date", the button "Return to v1.2.0-Beta-2271".
   - **Console:** OPL from the FreeMcBoot menu launches and shows the games.
5. **Rollback.** "Return to v1.2.0-Beta-2271" → the plan "Roll back the loader in UPDT/OPL.ELF" → confirm.
   - The card shows 2271 again and "Update: …"; the dialog does not open by itself (this pair was already offered).
   - The button is now "Return to <new version>": a repeated rollback restores the update.
   - **Console:** 2271 launches again.
6. **Setting.** "Settings" → "Loaders" → turn off "Offer loader updates automatically": the dialog does not appear either on the devices screen or after a check; the buttons on the card remain.
7. **Replacement without YAOPLM (optional).** Put another ELF manually on F: as `UPDT\OPL.ELF` (for example YAOPL): the card says "Replaced without YAOPLM" and shows the build by the version inside the file. Restore the previous file or use "Return to …".

## What to report

- For each step: "as expected" or what happened instead (a screenshot).
- Whether OPL launched on the console after the update (step 4) and after the rollback (step 5).
- The output of `yaoplm device F:\` after step 5.
