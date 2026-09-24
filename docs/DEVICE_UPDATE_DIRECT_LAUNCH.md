# Update an existing Kobo to direct E-Ink Chess launch

This guide is for a Kobo Libra H2O that already has the previous version of this
project installed, where the normal launch path is:

```text
NickelMenu -> Cobalt -> Chess
```

After this update the path becomes:

```text
NickelMenu -> E-Ink Chess -> chessboard
```

Cobalt remains installed underneath as the runtime, but its launcher is skipped.
The chess screen also gains **Return to Kobo reader**, which closes the app and
hands the panel back to Nickel.

These instructions use USB and the Cobalt revision pinned by the project.

## 1. Update the project checkout on the Mac

Open Terminal:

```sh
cd /Users/aba/dev/eink-chess-app
git pull
```

Confirm the new files are present:

```sh
ls docs/KOBO_ARCHITECTURE.md
ls docs/DEVICE_UPDATE_DIRECT_LAUNCH.md
```

Run the normal Rust tests:

```sh
cargo test
```

## 2. Reset the local Cobalt checkout to the pinned revision

The preparation script modifies the Cobalt checkout on purpose. Before preparing
a new package, reset that checkout so the script always starts from the exact
known source.

```sh
cd /Users/aba/dev/Cobalt
git reset --hard 026ac5561add0157109dd98592272ce9c6eb9343
```

This command discards local changes inside the Cobalt checkout. Do not run it
there if you have unrelated Cobalt work you want to keep.

Verify:

```sh
git rev-parse HEAD
```

It should print:

```text
026ac5561add0157109dd98592272ce9c6eb9343
```

## 3. Prepare Cobalt with E-Ink Chess as the root app

```sh
cd /Users/aba/dev/eink-chess-app
python3 scripts/prepare_cobalt.py ../Cobalt
```

The final line should say that the package was prepared for direct NickelMenu
launch through Cobalt.

Build/test the copied application:

```sh
cd ../Cobalt
cargo test -p kobo-eink-chess
```

Reinstall the `kobo` CLI from this prepared checkout, because the USB package
builder is part of the modified Cobalt source:

```sh
cargo install --path crates/kobo-cli --force
```

## 4. Connect the Kobo over USB

On the Kobo, accept the **Connect** prompt.

On the Mac, verify the mounted volume exists:

```sh
ls /Volumes/KOBOeReader
```

If that path does not exist, stop here and check the cable/Connect prompt.

## 5. Preview the installation

From the prepared Cobalt checkout:

```sh
cd /Users/aba/dev/Cobalt
kobo setup --volume /Volumes/KOBOeReader --source --dry-run
```

Check that the detected model and firmware are the expected reader.

## 6. Install the new package

```sh
kobo setup --volume /Volumes/KOBOeReader --source --no-eject --no-sample
```

This refreshes the Cobalt runtime and the packaged Chess binary. The generated
NickelMenu configuration now contains **E-Ink Chess** instead of **Cobalt**.

The existing Cobalt directory is still used because Cobalt is the runtime. The
visible launcher step is what has been removed.

## 7. Eject and restart

```sh
diskutil eject /Volumes/KOBOeReader
```

Unplug the cable.

Power the Kobo completely off and back on. After Nickel reaches the home screen,
leave it alone for about a minute so NickelMenu finishes its normal startup and
failsafe sequence.

## 8. Verify the new launch path

Open NickelMenu.

You should now see:

```text
E-Ink Chess
```

rather than the old Cobalt entry for this installation.

Tap **E-Ink Chess**.

Expected sequence:

1. Cobalt may briefly show its takeover splash while it stops Nickel.
2. The chessboard opens directly.
3. There is no Cobalt application launcher in between.
4. Move a piece, for example `e2 -> e4`.
5. Tap **Return to Kobo reader** at the bottom.
6. The chess session closes and Nickel returns.

Nickel restoration can take noticeably longer than closing a normal Nickel
screen because the runtime is returning hardware ownership to the stock reader.

## 9. If the old Cobalt menu entry is still shown

Reconnect USB and inspect the Cobalt-owned NickelMenu file:

```sh
cat /Volumes/KOBOeReader/.adds/nm/cobalt
```

For this project it should contain an `E-Ink Chess` menu item.

If the file is correct but the device still shows the old entry, eject and do a
full power-off/power-on again, then give NickelMenu a minute after the home
screen appears.

If the file still contains `Cobalt`, make sure you ran both:

```sh
python3 scripts/prepare_cobalt.py ../Cobalt
cd ../Cobalt
cargo install --path crates/kobo-cli --force
```

before `kobo setup --source`.

## 10. Future app updates

For later E-Ink Chess code changes, use the same sequence:

```sh
cd /Users/aba/dev/eink-chess-app
git pull
cargo test

cd /Users/aba/dev/Cobalt
git reset --hard 026ac5561add0157109dd98592272ce9c6eb9343

cd /Users/aba/dev/eink-chess-app
python3 scripts/prepare_cobalt.py ../Cobalt

cd ../Cobalt
cargo test -p kobo-eink-chess
cargo install --path crates/kobo-cli --force

kobo setup --volume /Volumes/KOBOeReader --source --no-eject --no-sample
diskutil eject /Volumes/KOBOeReader
```

Then restart the reader.

The reset step matters because `prepare_cobalt.py` is an integration patch
against one exact Cobalt revision. Starting from that exact revision makes the
result reproducible.
