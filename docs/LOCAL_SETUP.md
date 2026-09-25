# Local development setup

This guide is for a developer who has **never used Rust before**.

The shortest path is:

```text
Mac or Windows/WSL
        |
        v
      Rust
        |
        v
  eink-chess-app
        |
        v
   Cobalt SDK
        |
        v
 browser simulator
        |
        v
 Kobo later
```

For the current MVP you do **not** need Slint, a chess engine, PGN support, or
an ARM cross-compiler. The app is intentionally just a Rust board model plus a
thin Cobalt UI.

## 1. What is being installed?

A few names appear frequently in Rust projects:

- **Rust** is the programming language.
- **rustup** installs and manages Rust toolchains.
- **rustc** is the Rust compiler.
- **Cargo** is Rust's build tool and dependency manager. Think of it as a
  combination of npm, Maven/Gradle, and a build command.
- **crate** is Rust's word for a package/library/application.
- **Cobalt** is the Kobo application SDK/runtime used by this prototype.
- **rust-analyzer** is the language server used by editors for completion,
  navigation, errors, and refactoring.

This repository pins Rust in `rust-toolchain.toml`, so after Rust is installed
you normally do not need to choose a compiler version manually.

---

# macOS setup

These instructions work on both Apple Silicon and Intel Macs.

## 2. Install Apple's command-line developer tools

Open **Terminal.app** and run:

```sh
xcode-select --install
```

macOS will show an installer dialog. Complete it.

This gives Rust a native linker/compiler and also installs useful command-line
development tools.

Check that Git is available:

```sh
git --version
```

You should see a version number.

## 3. Install Rust

Rust's recommended installer is `rustup`.

In Terminal:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Choose the default installation when asked.

When it finishes, either open a new Terminal window or load Cargo into the
current shell:

```sh
source "$HOME/.cargo/env"
```

Verify the installation:

```sh
rustup --version
rustc --version
cargo --version
```

All three commands should print version information.

Do not worry if your globally installed Rust version is newer than the version
used by this repository. `rustup` reads `rust-toolchain.toml` when you enter
the project and selects the repository's pinned toolchain.

## 4. Optional but recommended: install an editor

VS Code is a convenient starting point:

https://code.visualstudio.com/

Install these extensions:

- **rust-analyzer**
- **Even Better TOML** (optional, useful for `Cargo.toml`)

You do not need a special Rust IDE. VS Code + rust-analyzer is enough.

## 5. Clone the chess app

Choose a directory where you keep source code:

```sh
mkdir -p ~/src
cd ~/src

git clone https://github.com/abavelski/eink-chess-app.git
cd eink-chess-app
```

Now confirm which Rust toolchain the repository selected:

```sh
rustup show
```

Then compile and run the tests:

```sh
cargo test
```

The first build is slower because Cargo downloads and compiles dependencies.
Later builds are incremental and much faster.

If `cargo test` succeeds, the basic Rust setup is working.

## 6. Useful Rust commands

From the repository root:

```sh
# Compile and run tests
cargo test

# Check the code without producing a final executable
cargo check

# Format the source code
cargo fmt

# Check formatting without changing files
cargo fmt --all -- --check

# Run Rust's additional static analysis
cargo clippy
```

A normal edit/test loop is simply:

```text
edit code
   |
cargo test
   |
repeat
```

For this project, start by reading:

```text
src/board.rs
```

It is deliberately plain Rust and contains no Kobo-specific code.

Then read:

```text
src/main.rs
```

That is the thin adapter between our board model and Cobalt.

---

# Cobalt setup

## 7. Why Cobalt is separate

The current Kobo prototype uses the Cobalt SDK.

The application depends on Cobalt as a Rust dependency, but Cobalt also
provides a command-line tool named `kobo` which runs the browser simulator and
later helps with device development.

Keep Cobalt in a separate checkout beside this repository:

```text
~/src/
├── Cobalt/
└── eink-chess-app/
```

## 8. Clone the matching Cobalt version

This project currently pins the Cobalt SDK to:

```text
6737d128b3110a79a8b25216c58995b5ddc4b37c
```

For the smoothest development setup, install the CLI from the same revision:

```sh
cd ~/src

git clone https://github.com/abavelski/Cobalt.git
cd Cobalt

git checkout 6737d128b3110a79a8b25216c58995b5ddc4b37c
```

Install Cobalt's `kobo` command:

```sh
cargo install --path crates/kobo-cli --force
```

Verify it:

```sh
kobo --help
```

If your shell says `kobo: command not found`, restart Terminal or run:

```sh
source "$HOME/.cargo/env"
```

## 9. Run the app in the Cobalt simulator

Return to this project:

```sh
cd ~/src/eink-chess-app
```

First make sure the Rust model still works:

```sh
cargo test
```

Then start Cobalt development mode:

```sh
kobo dev
```

Cobalt's simulator runs locally and presents the e-ink UI in a browser.

The first useful manual test is intentionally simple:

1. confirm that the full 8×8 board is visible;
2. tap the white pawn on e2;
3. confirm the square is selected;
4. tap e4;
5. confirm the pawn moved from e2 to e4;
6. tap **Reset** and confirm the starting position returns.

There are no chess rules yet. Moving a rook diagonally or putting one piece on
top of another is currently allowed by design.

## 10. If `kobo dev` and the app use different Cobalt revisions

Cobalt is evolving quickly. If the SDK protocol changes, using a newer CLI
against this repository's older pinned SDK can cause confusing errors.

The safest rule is:

> Use the Cobalt CLI from the same commit that is pinned in `Cargo.toml`.

You can see the currently pinned revision with:

```sh
grep kobo-sdk Cargo.toml
```

If the project later updates Cobalt, update the Cobalt checkout too:

```sh
cd ~/src/Cobalt
git fetch
git checkout <new-revision>
cargo install --path crates/kobo-cli --force
```

---

# Windows setup

Rust itself works very well on native Windows. Cobalt's current host builds,
however, are tested and released for macOS and Linux rather than native
Windows.

For this project the recommended Windows setup is therefore **WSL2 + Ubuntu**.

That gives you almost the same development environment as Linux and avoids
having two separate sets of Cobalt instructions.

## 11. Install WSL2

Open **PowerShell as Administrator**:

```powershell
wsl --install -d Ubuntu
```

Restart Windows if requested.

Launch **Ubuntu** from the Start menu and complete the initial username/password
setup.

From this point onward, run the Linux commands inside the Ubuntu terminal, not
PowerShell.

## 12. Install Linux development prerequisites

Inside Ubuntu/WSL:

```sh
sudo apt update
sudo apt install -y build-essential git curl pkg-config
```

Then install Rust:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Verify it:

```sh
rustup --version
rustc --version
cargo --version
git --version
```

## 13. Clone the project inside the Linux filesystem

Prefer:

```text
~/src/eink-chess-app
```

inside WSL rather than a directory under `/mnt/c/`.

Rust builds involve many small filesystem operations and are generally simpler
and faster in WSL's Linux filesystem.

Run:

```sh
mkdir -p ~/src
cd ~/src

git clone https://github.com/abavelski/eink-chess-app.git
cd eink-chess-app

cargo test
```

## 14. Install Cobalt in WSL

Still inside Ubuntu/WSL:

```sh
cd ~/src

git clone https://github.com/abavelski/Cobalt.git
cd Cobalt
git checkout 6737d128b3110a79a8b25216c58995b5ddc4b37c

cargo install --path crates/kobo-cli --force
```

Then:

```sh
cd ~/src/eink-chess-app
kobo dev
```

Modern WSL forwards localhost services to Windows, so the simulator can be
viewed from your normal Windows browser.

For VS Code on Windows, install Microsoft's **WSL** extension and open the
project from the WSL terminal with:

```sh
code .
```

Install rust-analyzer in the WSL VS Code environment when VS Code offers to do
so.

## 15. Native Windows alternative

If you only want to learn Rust or run the pure board tests, native Windows is
fine.

Install Rust from:

https://www.rust-lang.org/tools/install

The installer may ask for the Microsoft Visual Studio C++ Build Tools. Install
them when prompted.

Then native PowerShell can run:

```powershell
git clone https://github.com/abavelski/eink-chess-app.git
cd eink-chess-app
cargo test
```

For Cobalt/Kobo development, use the WSL path above instead.

---

# What you do NOT need yet

A new developer can easily install too much. For the current milestone, avoid
that.

## Slint

**Not needed for the current Kobo prototype.**

Slint is the UI toolkit we are considering for the future desktop/Kindle
frontend. Cobalt currently supplies the Kobo UI.

When we add a Slint frontend later, it will normally arrive as Cargo
dependencies in the project. There is no need to install a separate Slint SDK
today.

## Chess engine

Not needed.

Stockfish belongs much later.

## Chess rules library

Not needed for the physical-board MVP.

The current `Board::tap()` intentionally lets any piece move to any square.

## PGN parser

Not needed yet.

## ARM cross-compiler

Not needed for `cargo test` or `kobo dev`.

Only install device cross-compilation tools when we are ready to build/deploy
to the real Kobo. Cobalt's device build currently targets:

```text
armv7-unknown-linux-musleabihf
```

At that point start with:

```sh
rustup target add armv7-unknown-linux-musleabihf
```

Cobalt's cross-compilation requirements can change as native dependencies
change, so follow the device-build instructions from the **same pinned Cobalt
revision** instead of copying an old cross-compiler recipe.

---

# A five-minute Rust orientation

You do not need to learn all of Rust before changing this project.

This is enough to begin.

A variable:

```rust
let square = 12;
```

A mutable variable:

```rust
let mut selected = None;
selected = Some(12);
```

A struct:

```rust
struct Piece {
    color: Color,
    kind: PieceKind,
}
```

An enum:

```rust
enum Color {
    White,
    Black,
}
```

An optional value:

```rust
Option<Piece>
```

means either:

```rust
Some(piece)
```

or:

```rust
None
```

A method:

```rust
impl Board {
    fn reset(&mut self) {
        *self = Self::starting_position();
    }
}
```

A test:

```rust
#[test]
fn reset_restores_the_position() {
    let mut board = Board::default();
    board.reset();
}
```

Run all tests with:

```sh
cargo test
```

The compiler is deliberately strict. When Rust refuses to compile something,
read the complete error message: Rust's diagnostics usually explain both the
problem and where to look next.

---

# Recommended first exercises

Before adding features, make a few harmless changes locally:

1. Change the window/app title in `src/main.rs`.
2. Change the starting location of one piece in `src/board.rs`.
3. Run `cargo test` and see which test notices.
4. Add a new unit test.
5. Restore the normal starting position.
6. Run `cargo fmt`.
7. Run `kobo dev` and move a few pieces.

After that, a good first real feature is **Flip Board**.

It is small enough to learn the codebase without introducing chess rules,
files, persistence, or networking.

---

# Troubleshooting

## `cargo: command not found`

Restart the terminal, or:

```sh
source "$HOME/.cargo/env"
```

## macOS linker/compiler errors

Make sure Apple's command-line tools are installed:

```sh
xcode-select --install
```

Check:

```sh
xcode-select -p
```

## Wrong Rust version

From the project directory:

```sh
rustup show
```

The repository's `rust-toolchain.toml` should control the selected toolchain.

If the pinned toolchain has not yet been downloaded:

```sh
rustup install 1.85.1
```

## `kobo: command not found`

Reinstall the Cobalt CLI:

```sh
cd ~/src/Cobalt
cargo install --path crates/kobo-cli --force
source "$HOME/.cargo/env"
```

## Cobalt protocol/version errors

Make sure the Cobalt checkout matches the revision in this repository's
`Cargo.toml`.

## Something fails after updating Cobalt

Do not immediately update every dependency.

First return to the pinned revision and verify that the existing project still
works. Then update Cobalt as a separate intentional change.

---

# Useful references

- Rust getting started: https://www.rust-lang.org/learn/get-started
- The Rust Book: https://doc.rust-lang.org/book/
- Cargo Book: https://doc.rust-lang.org/cargo/
- rust-analyzer: https://rust-analyzer.github.io/
- Cobalt: https://github.com/BandarLabs/Cobalt
- This project: https://github.com/abavelski/eink-chess-app
