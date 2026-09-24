# Install E-Ink Chess on a Kobo over USB

These steps use the Cobalt SDK revision pinned by this app. They install Cobalt
and E-Ink Chess together. The reader needs NickelMenu, a charged battery, and
a data-capable USB cable. No Wi-Fi or SSH is needed.

## One-time setup on this Mac

The Cobalt checkout belongs beside this repository:

```sh
cd /Users/aba/dev
git clone https://github.com/BandarLabs/Cobalt.git
cd Cobalt
git checkout 026ac5561add0157109dd98592272ce9c6eb9343
```

If `Cobalt` is already there at that revision, skip the clone. Install the ARM
build tools from inside the Cobalt directory (its Rust toolchain may differ from
the chess app's toolchain):

```sh
rustup target add armv7-unknown-linux-musleabihf
brew install messense/macos-cross-toolchains/armv7-unknown-linux-musleabihf
```

Prepare the local Cobalt checkout. This copies the current chess source into
its workspace and registers Chess in its device package and launcher metadata:

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

From the Cobalt checkout, preview the install:

```sh
cd /Users/aba/dev/Cobalt
kobo setup --volume /Volumes/KOBOeReader --source --dry-run
```

Check that it names your model and firmware. Then install:

```sh
kobo setup --volume /Volumes/KOBOeReader --source --no-eject --no-sample
```

Confirm the prompt. This builds ARM binaries, writes Cobalt and Chess to
`.adds/cobalt` on the reader, verifies the copied files, and adds a Cobalt
NickelMenu entry. It preserves the existing NickelMenu entries and KOReader.
This revision's device package also includes Cobalt's bundled apps. The first
build can take several minutes.

Cobalt setup also enables the reader's `ForceWifiOn` developer setting and sets
its automatic sleep timer to 90 minutes. It leaves the firmware SSH server
disabled. You can change the sleep timer later in the reader's Energy saving
settings.

When setup reports success, eject the volume from Finder or run:

```sh
diskutil eject /Volumes/KOBOeReader
```

Unplug the cable. Power the Kobo completely off and back on, then leave it at
the home screen for about one minute so NickelMenu completes its startup. Open
NickelMenu (usually the bottom-right menu), choose **Cobalt**, then **Chess**.

## Install an updated version of this app later

Connect the reader again, then from this repository:

```sh
python3 scripts/prepare_cobalt.py ../Cobalt
cd ../Cobalt
kobo setup --volume /Volumes/KOBOeReader --source --no-eject --no-sample
diskutil eject /Volumes/KOBOeReader
```

Unplug and restart the reader. The preparation script is safe to run again: it
refreshes the copied Rust files without adding duplicate registry entries.
The Cobalt checkout must remain at the pinned commit. A normal Cobalt platform
update may replace this custom device package, so repeat these steps if Chess
disappears after updating Cobalt.

## If the Cobalt menu entry is missing

First check that NickelMenu itself still works. If a firmware update removed
its plugin, reconnect the reader and run the setup command with `--menu` to
stage NickelMenu again, then eject and restart. See Cobalt's
`docs/INSTALL.md` in the sibling checkout for recovery details.
