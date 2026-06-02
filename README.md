> [!IMPORTANT]
> Install directly from GitHub Actions here:
>
> <a href="https://github.com/cmss13-devs/launcher/releases/tag/v0.19.6">
>  <img src="https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Windows download link"/>
> </a>

# SS13 Launcher ![Steam Build](https://img.shields.io/github/actions/workflow/status/cmss13-devs/cm-launcher/steam.yml?style=for-the-badge&label=STEAM%20BUILD) ![GitHub Build](https://img.shields.io/github/actions/workflow/status/cmss13-devs/cm-launcher/build.yml?style=for-the-badge&label=GITHUB%20BUILD) ![Tests](https://img.shields.io/github/actions/workflow/status/cmss13-devs/cm-launcher/build.yml?style=for-the-badge&label=TESTS)

A launcher for Space Station 13 servers, using [Tauri](https://v2.tauri.app/) and managing BYOND versions internally.

## Screenshots

| CM-SS13 Game Servers                                                                                                                               | Authentication Options (Steam only available in Steam builds)                                                                                      | Automatic Relay Selection                                                                                                                          |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| <img width="1992" height="1188" alt="VcnBDvrlqS7Tfryu@2x" src="https://github.com/user-attachments/assets/d8b5ac37-e818-45cb-b020-5fd96dc64f50" /> | <img width="1981" height="1179" alt="0SR6wKmNaPuefRBK@2x" src="https://github.com/user-attachments/assets/e196bac1-f134-42da-9990-4e4864c24129" /> | <img width="1996" height="1200" alt="6whuDKXeRfZD5E3f@2x" src="https://github.com/user-attachments/assets/b4f08132-6740-4b50-bb91-8f527e2aab5f" /> |

## Features

### BYOND

- Automatically installs the correct version for the game server you are connecting to.
- Private WebView2 install location to avoid conflicts with system BYOND.

### Authentication

- CM-SS13 Authentication via web browser authentication flow
  - Handles tokens refresh to stay logged in indefinitely
- BYOND Authentication via pager
- Steam Authentication via Authentication ticket flow/Authentik backend

### Rich Presence

- Supports Steam and Discord rich presence
- Displays currently launched server, as well as the number of players online
- Allows friends to join directly from the friends list

### CI/CD

- Automatically deploys tagged versions to GitHub Releases and Steam
- Steam releases are pushed to a `latest` branch for manual deployment to `default`.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://rustup.rs/) (stable)
- Platform-specific dependencies (see below)

Install frontend dependencies first:

```bash
npm install
```

### Build Variants

The launcher has two variants controlled by Cargo features and Tauri config overlays:

| Variant | Feature flag | Config overlay | Description |
|---------|-------------|----------------|-------------|
| **SS13 (generic)** | _(none)_ | _(none)_ | Generic SS13 server browser |
| **CM-SS13** | `cm_ss13` | `--config src-tauri/tauri.cm.conf.json` | CM-SS13 specific launcher |

Both variants can optionally include Steam support by adding the `steam` feature.

### Dev Mode (with hot-reloading)

```bash
# SS13 (generic)
npm run tauri -- dev

# CM-SS13
npm run tauri -- dev --features cm_ss13 --config src-tauri/tauri.cm.conf.json

# Either variant with Steam support
npm run tauri -- dev --features steam
npm run tauri -- dev --features cm_ss13,steam --config src-tauri/tauri.cm.conf.json
```

To run a Steam build in development, place a file named `steam_appid.txt` in `src-tauri/` containing `4313790`. Otherwise, the app will immediately close and attempt to reopen via Steam.

### Production Builds

```bash
# SS13 (generic)
npm run tauri -- build

# CM-SS13
npm run tauri -- build --features cm_ss13 --config src-tauri/tauri.cm.conf.json
```

### Windows Setup

Download the WebView2 fixed runtime (bundled into the installer so users don't need system WebView2):

```powershell
./scripts/download-webview2.ps1
```

No other platform-specific setup is needed on Windows.

### Linux Setup

Install system dependencies (Ubuntu 24.04):

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-0 libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-0 libjavascriptcoregtk-4.1-dev \
  gir1.2-javascriptcoregtk-4.1 gir1.2-webkit2-4.1 \
  libappindicator3-dev librsvg2-dev \
  patchelf musl-tools cabextract
```

Linux builds require bundled sidecars (Wine, WebView2, cabextract) for running BYOND.

1. Download Wine, winetricks, and cabextract (bundles Wine so Linux users can run BYOND via Wine):

   ```bash
   ./scripts/download-wine.sh
   ```

2. Download the WebView2 fixed runtime:

   ```bash
   ./scripts/download-webview2.sh
   ```

3. Prepare the sidecar directory for AppImage bundling. The directory name must match the `productName` in tauri.conf.json:

   ```bash
   # For SS13 (generic) — productName is "SS13 Launcher"
   mkdir -p "/tmp/lib/SS13 Launcher"
   cp src-tauri/wine.tar.zst "/tmp/lib/SS13 Launcher/"
   cp src-tauri/winetricks "/tmp/lib/SS13 Launcher/"
   cp src-tauri/cabextract "/tmp/lib/SS13 Launcher/"
   chmod +x "/tmp/lib/SS13 Launcher/winetricks" "/tmp/lib/SS13 Launcher/cabextract"
   cp -r src-tauri/webview2-runtime "/tmp/lib/SS13 Launcher/webview2-runtime"

   # For CM-SS13 — productName is "CM-SS13 Launcher"
   mkdir -p "/tmp/lib/CM-SS13 Launcher"
   cp src-tauri/wine.tar.zst "/tmp/lib/CM-SS13 Launcher/"
   cp src-tauri/winetricks "/tmp/lib/CM-SS13 Launcher/"
   cp src-tauri/cabextract "/tmp/lib/CM-SS13 Launcher/"
   chmod +x "/tmp/lib/CM-SS13 Launcher/winetricks" "/tmp/lib/CM-SS13 Launcher/cabextract"
   cp -r src-tauri/webview2-runtime "/tmp/lib/CM-SS13 Launcher/webview2-runtime"
   ```

4. Linux AppImage builds also require a custom tauri-cli fork and the linux config overlay:

   ```bash
   cargo install tauri-cli --git https://github.com/tauri-apps/tauri --branch feat/truly-portable-appimage --force
   ```

5. Build with the linux config overlay and the `ADD_DIR` env var:

   ```bash
   # SS13 (generic)
   ADD_DIR="/tmp/lib/SS13 Launcher" cargo tauri build --config src-tauri/tauri.linux.conf.json

   # CM-SS13
   ADD_DIR="/tmp/lib/CM-SS13 Launcher" cargo tauri build --features cm_ss13 \
     --config src-tauri/tauri.cm.conf.json --config src-tauri/tauri.linux.conf.json
   ```

### Regenerating TypeScript Bindings

When Tauri commands or types change, regenerate the TS bindings:

```bash
cd src-tauri && cargo test --features steam --lib export_bindings
```

The `steam` feature is required so that Steam-specific bindings are included.

### Releasing

Use `tools/release.sh [semver]` to change the version in `Cargo.toml`, create a commit changing the version, and tag that commit with the semver. When this is pushed, GitHub Actions will push new builds to both GitHub Releases and Steam.

Manually download the `.msi` and `.exe` and upload these to [Microsoft](https://www.microsoft.com/en-us/wdsi/filesubmission) to /try/ and avoid SmartScreen when installed via GitHub Releases.

### To-Do

See issues tagged with https://github.com/cmss13-devs/cm-launcher/labels/feature-request or https://github.com/cmss13-devs/cm-launcher/labels/bug as an easy place to start contributing.
