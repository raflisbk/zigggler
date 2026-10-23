# zzzzziglerrr

A reliable mouse jiggler for Linux (Wayland), built with Rust + Tauri.

Most jigglers rely on `xdotool`, which doesn't work on Wayland. `zzzzziglerrr` moves the
cursor through a real `uinput` virtual device and, on KDE Plasma/KWin, also signals genuine
seat activity via the `org_kde_kwin_idle` protocol — so presence-tracking tools like
ActivityWatch see a real activity signal, not just a twitching cursor.

*Baca dalam Bahasa Indonesia: [README.id.md](README.id.md)*

## Features

- Toggle jiggling on/off from the tray icon or the settings window (state stays in sync)
- Configurable jiggle interval
- Idle-gated jiggling — only fires once you're actually idle, so it never interferes while you're working
- Tray icon reflects live state: off / waiting for idle / jiggling
- Screensaver/DPMS inhibited while jiggling is active (`org.freedesktop.ScreenSaver.Inhibit`)
- 11 curated themes, 3 languages (Indonesian / English / Japanese), switchable without restart
- Closing the window hides it to the tray — the app keeps running until you quit it from the tray menu

## Platform support

| Platform | Status |
|---|---|
| Linux — KDE Plasma / KWin (Wayland) | ✅ Supported, verified |
| Linux — other Wayland compositors (GNOME/Mutter, Sway) | 🚧 Planned |
| macOS | 🚧 Planned |
| Windows | 🚧 Planned |

## Install (Debian-based Linux)

Pre-built `.deb` packages are published on the [Releases page](https://github.com/raflisbk/zigggler/releases/latest).

```bash
# Download the latest .deb from the Releases page, then:
sudo dpkg -i zzzzziglerrr_*.deb
# If dpkg complains about missing dependencies:
sudo apt --fix-broken install
```

### Required one-time setup: `/dev/uinput` access

The jiggle feature writes synthetic input events through `/dev/uinput`, which is root-owned
by default. A udev rule grants your user session access without adding you to any broad
device group:

```bash
sudo platform/linux/install-udev-rule.sh
```

This takes effect immediately for your current session — no reboot or re-login needed.
Verify it worked:

```bash
getfacl /dev/uinput   # should show user:<you>:rw-
```

See [`platform/linux/README.md`](platform/linux/README.md) for the full rationale and
troubleshooting (including a common udev rule-ordering pitfall on some distros).

After installing, launch `zzzzziglerrr` from your application launcher — it starts in the
system tray.

## Build from source

Requirements: Rust toolchain, Node.js (for the Tauri CLI), and Tauri's system dependencies
(`libwebkit2gtk-4.1-dev`, `libudev-dev`, and friends — see your distro's Tauri prerequisites).

```bash
npm install
npm run tauri dev              # run in development
npm run tauri build -- --bundles deb   # produce a release .deb
```

No frontend build step — the UI is plain HTML/CSS/JS served as-is, no bundler involved.

## Known limitations

- Idle-detection and the activity-reset mechanism (`org_kde_kwin_idle.simulate_user_activity`)
  are KDE/KWin-specific. On other compositors, jiggling still moves the cursor via `uinput`,
  but presence-tracking tools may still report you as away.
- Not tested against tools other than [ActivityWatch](https://activitywatch.net/). It should
  not be assumed to work identically with every presence/AFK tracker.
- X11 is not a primary target; Wayland is the focus.

## Status

Linux/KWin core feature set (P0) is complete and verified, including confirmed non-AFK
behavior in ActivityWatch during active jiggling. Autostart-on-login and support for other
compositors/platforms are in progress — see [`docs/goals.md`](docs/goals.md) if you have
local access to it (it's gitignored and not part of this repo's public history).

## License

No license file has been added yet — this is currently a personal tool, not a published
open-source release. Treat the source as "all rights reserved" until a license is added.
