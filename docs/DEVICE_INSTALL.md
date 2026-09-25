# Install E-Ink Chess on a Kobo over USB

These steps use the Cobalt SDK revision pinned by this app. They install Cobalt
and E-Ink Chess together. The reader needs NickelMenu, a charged battery, and
a data-capable USB cable. No Wi-Fi or SSH is needed.

Cobalt remains the hardware/runtime layer, but the installed owner-facing path
is direct:

```text
NickelMenu -> E-Ink Chess -> chessboard
```

The Cobalt application launcher is skipped.

If you already have the earlier `NickelMenu -> Cobalt -> Chess` build installed,
use [Update an existing Kobo to direct E-Ink Chess launch](DEVICE_UPDATE_DIRECT_LAUNCH.md).

## One-time setup on this Mac

The Cobalt checkout belongs beside this repository:

```sh
cd /Users/aba/dev
git clone https://github.com/abavelski/Cobalt.git
cd Cobalt
git checkout b9f46f21af1e209af3c4030e7af90a20c722395c
```

If `Cobalt` is already there at that revision, skip the clone. Install the ARM
build tools from inside the Cobalt directory (its Rust toolchain may differ from
the chess app's toolchain):

```sh
rustup target add armv7-unknown-linux-musleabihf
brew install messense/macos-cross-toolchains/armv7-unknown-linux-musleabihf
```

Prepare the local Cobalt checkout. This copies the current chess source into
its workspace, registers Chess in its device package, points Cobalt's packaged
panel session at Chess instead of the Cobalt launcher, and changes the generated
NickelMenu entry to **E-Ink Chess**:

```sh
cd /Users/aba/dev/eink-chess-app
python3 scripts/prepare_cobalt.py ../Cobalt
cd ../Cobalt
cargo test -p kobo-eink-chess
cargo install --path crates/kobo-cli --force
```

`cargo install` puts the patched `kobo` command in `~/.cargo/bin`. If a terminal
cannot find it, reopen Terminal or add that directory to your `PATH`.

## Install on the connected reader

On the Kobo, accept the **Connect** prompt. Check that `/Volumes/KOBOeReader`
appears on the Mac. The reader should show its USB connection screen.

From the prepared Cobalt checkout, preview the install:

```sh
cd /Users/aba/dev/Cobalt
kobo setup --volume /Volumes/KOBOeReader --source --dry-run
```

Check that it names your model and firmware. Then install:

```sh
kobo setup --volume /Volumes/KOBOeReader --source --no-eject --no-sample
```

Confirm the prompt. This builds ARM binaries, writes Cobalt and Chess to
`.adds/cobalt` on the reader, verifies the copied files, and writes an
**E-Ink Chess** NickelMenu entry. It preserves existing unrelated NickelMenu
entries and KOReader. This revision's device package still contains Cobalt's
bundled apps; they are simply not in the normal Chess launch path.

Cobalt setup also enables the reader's `ForceWifiOn` developer setting and sets
its automatic sleep timer to 90 minutes. It leaves the firmware SSH server
disabled. You can change the sleep timer later in the reader's Energy saving
settings.

When setup reports success, eject the volume from Finder or run:

```sh
diskutil eject /Volumes/KOBOeReader
```

Unplug the cable. Power the Kobo completely off and back on, then leave it at
the home screen for about one minute so NickelMenu completes its startup.

Open NickelMenu (usually the bottom-right menu) and choose **E-Ink Chess**.
Cobalt may briefly show its takeover splash while it stops Nickel, then the
chessboard should open directly. There is no Cobalt launcher selection step.

To leave the app, tap **Return to Kobo reader** at the bottom of the chess
screen. Closing the root Chess app ends the Cobalt panel session and restores
Nickel.

## Install an updated version of this app later

Because the preparation script patches one exact Cobalt source revision, reset
the local Cobalt checkout before each new device package:

```sh
cd /Users/aba/dev/Cobalt
git reset --hard b9f46f21af1e209af3c4030e7af90a20c722395c
```

Then:

```sh
cd /Users/aba/dev/eink-chess-app
python3 scripts/prepare_cobalt.py ../Cobalt

cd ../Cobalt
cargo test -p kobo-eink-chess
cargo install --path crates/kobo-cli --force

kobo setup --volume /Volumes/KOBOeReader --source --no-eject --no-sample
diskutil eject /Volumes/KOBOeReader
```

Unplug and restart the reader.

See [Update an existing Kobo to direct E-Ink Chess launch](DEVICE_UPDATE_DIRECT_LAUNCH.md)
for a more detailed step-by-step update/checklist.

## If the E-Ink Chess menu entry is missing

First check that NickelMenu itself still works. If a firmware update removed
its plugin, reconnect the reader and run the setup command with `--menu` to
stage NickelMenu again, then eject and restart. See Cobalt's `docs/INSTALL.md`
in the sibling checkout for recovery details.

If NickelMenu works but still shows **Cobalt** instead of **E-Ink Chess**, make
sure the Cobalt checkout was reset to the pinned commit and then prepared with
this repository's `scripts/prepare_cobalt.py` before reinstalling the `kobo`
CLI and running `kobo setup --source`.
