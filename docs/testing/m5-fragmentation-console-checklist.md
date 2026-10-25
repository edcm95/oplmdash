# M5: fragmentation check on the console (IMP-7b, LIB-4)

What we check: YAOPLM counts file pieces the same way OPL does. A game over the limit does not launch in OPL, and YAOPLM warns about it in advance in the library and after a write. Details — ADR-0037, research/06 §1.11.

## What is needed

- Test FAT32 stick (E: will do) with about 2.5 GB free space on it.
- OPL 1.2.0 beta (the latest build). If available — OPL 1.1.0 for step 4.
- Test homebrew images from `D:\yaoplm-testdata\out\`: `YAOP_000.01.wLaunchELF.iso` (530 KiB) and `YAOP_000.06.wLaunchELF 1000MiB.iso` (1 GiB). Both launch wLaunchELF, no commercial games needed.
- A tool for playing the game "in pieces" (it writes only `--out` and its own temporary `<out>.filler` to the stick, which it deletes itself; you run it):

```
cargo run -p yaoplm-test-support --example make_fragmented -- --image <image> --out <path on the stick> --pieces <N>
```

It prints how many pieces Windows ultimately laid the file out in. YAOPLM must show the same number.

## Steps

1. **Normal write.** Add `YAOP_000.01.wLaunchELF.iso` to the stick in YAOPLM.
   - The write result has no piece warning.
   - In the game card (advanced mode, "Details") — "Pieces on the device: 1 (OPL loads a game from no more than 64)".
   - In OPL 1.2 the game launches (wLaunchELF appears).
2. **A game in 70 pieces.** Run:
   `cargo run -p yaoplm-test-support --example make_fragmented -- --image "D:\yaoplm-testdata\out\YAOP_000.06.wLaunchELF 1000MiB.iso" --out "E:\DVD\YAOP_000.06.Fragmented.iso" --pieces 70`
   - The tool prints "… in 70 pieces" (or a similar number greater than 64).
   - In YAOPLM after refreshing the library the game has the warning "The game lies on the device in pieces (parts: 70)…", and the "Only games with warnings" filter shows it.
   - `yaoplm scan E:\` shows the same number in the `PIECES` column.
   - In OPL 1.2 the game **does not launch**: OPL says the game is fragmented. Write down the exact message text.
3. **Exactly at the boundary.** Delete the game from step 2 through YAOPLM and repeat the command with `--pieces 64`.
   - There is no warning, the card says "64 (…no more than 64)".
   - In OPL 1.2 the game launches.
4. **OPL 1.1.0** (if available). In YAOPLM settings select the "OPL 1.1.0" profile. Delete the game from step 3 and repeat the command with `--pieces 2`.
   - YAOPLM warns: "…OPL 1.1.0 launches a game from USB, MX4SIO and iLink only if every file is one piece…".
   - OPL 1.1.0 does not launch the game, OPL 1.2 does.
5. **Clean up.** Delete the test games through YAOPLM (with confirmation) and restore the profile if you changed it.

## What to report

- For each step: "as expected" or what happened instead.
- The OPL message text from steps 2 and 4 (a screen photo is fine).
- The output of `yaoplm scan E:\` after step 2.
