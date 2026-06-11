# culex — HiveGame mobile

iOS + Android shell hosting the existing hivegame.com Leptos frontend. The
public app name is **HiveGame**; `culex` is the crate/directory name,
chosen to parallel `apis` (Latin for bee — the web crate) with the genus
name of the mosquito (Hive piece that copies adjacent pieces' moves, which
mirrors how this shell hosts the existing web frontend wholesale).

## Phase 0 status

Spike: webview loads `https://hivegame.com` directly. Production target later in Phase 0 / early Phase 1 is a **bundled CSR Leptos build** that talks to the backend over its public API.

## One-time setup

### Tooling split

- **nix dev shell** (`nix develop`) owns the Rust/Leptos stack.
- **Homebrew** owns the iOS tooling. Tauri's iOS prereq checks (`xcodegen`, `cocoapods`, `libimobiledevice`, …) are brew-centric — let brew handle them rather than mixing nixpkgs in. We tried nix-first and hit nixpkgs darwin build failures (e.g. `libplist` test segfault) and Tauri's brew-specific check logic.
- **Android Studio** owns the Android SDK + NDK.

### iOS prereqs

```bash
# 1. Install Homebrew if you don't have it.
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
# Then follow the post-install hints to put brew on your PATH
# (Apple Silicon: eval "$(/opt/homebrew/bin/brew shellenv)").

# 2. Install the Tauri 2 CLI (cargo binstall is faster if you have it).
cargo install tauri-cli --version "^2" --locked

# 3. Let Tauri auto-install its iOS deps via brew (xcodegen, cocoapods,
#    libimobiledevice, etc.) on first init.
cd culex
cargo tauri ios init
```

### Android prereqs

Install Android Studio. From there install the SDK + NDK, then set:

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/<version>"   # ls $ANDROID_HOME/ndk to see version
```

Then:

```bash
cd culex
cargo tauri android init
```

## Run

```bash
# Android emulator (start an AVD first via Android Studio).
cargo tauri android dev

# iOS simulator. Use the `tauri-ios` wrapper — it sets DEVELOPER_DIR to point
# at the real Xcode just for this command. DON'T set DEVELOPER_DIR globally:
# it breaks nix's clang wrapper for macOS host build scripts (which Android
# cross-compile invokes), causing libSystem link failures.
tauri-ios dev
```

## Layout

- `src-tauri/` — Rust Tauri shell (this is the native app).
- `dist/` — frontend bundle that gets packaged into the app. For the spike this is a placeholder; the webview redirects to `https://hivegame.com` on launch. Once we move to the bundled CSR build, `cargo leptos` (or equivalent) will produce the contents of `dist/`.

## Icons

Reuse from `apis/assets/`:
- `android-chrome-192x192.png` — Android launcher
- `android-chrome-512x512.png` — Play Store + adaptive icon
- `apple-touch-icon.png` — iOS

These get copied into `src-tauri/icons/` and `src-tauri/gen/{android,apple}/` during `cargo tauri {android,ios} init`. Re-run `cargo tauri icon ../path/to/source.png` if we ever want to regenerate the full set.
