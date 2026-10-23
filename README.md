# zzzzziglerrr

![release](https://img.shields.io/github/v/release/raflisbk/zigggler?label=release)
![CI](https://github.com/raflisbk/zigggler/actions/workflows/ci.yml/badge.svg)
![platform](https://img.shields.io/badge/platform-Linux%20%7C%20KDE%2FKWin-blue)
![license](https://img.shields.io/badge/license-unreleased-lightgrey)

**Move the cursor like it's real. Built for KDE Wayland.**

[Releases](https://github.com/raflisbk/zigggler/releases/latest) · [Install](#install-debian-based-linux) · [Build from source](#build-from-source) · [Known limitations](#known-limitations) · [Status](#status)

*Baca dalam Bahasa Indonesia: [README.id.md](README.id.md)*

---

Most mouse jigglers rely on `xdotool`, which doesn't work on Wayland. `zzzzziglerrr` moves
the cursor through a real `uinput` virtual device and, on KDE Plasma/KWin, also signals
genuine seat activity through the `org_kde_kwin_idle` protocol — so presence-tracking tools
like ActivityWatch register a real activity signal, not just a twitching cursor.

Confirmed in production: ActivityWatch stayed non-AFK across two independent 60-minute and
31-minute runs with zero human input. See [Status](#status).

## Features

- Toggle jiggling on/off from the tray icon or the settings window — state always stays in sync
- Configurable jiggle interval
- Idle-gated jiggling — only fires once you're genuinely idle, never interferes while you're working
- Tray icon reflects live state: off / waiting for idle / jiggling
- Screensaver/DPMS inhibited while jiggling is active (`org.freedesktop.ScreenSaver.Inhibit`)
- 11 curated themes, 3 languages (Indonesian / English / Japanese), switchable without restart
- Closing the window hides it to the tray — the app keeps running until you quit it from the tray menu

## Platform support

|Platform|Status|
|---|---|
|Linux — KDE Plasma / KWin (Wayland)|✅ Supported, verified|
|Linux — other Wayland compositors (GNOME/Mutter, Sway)|🚧 Planned|
|macOS|🚧 Planned|
|Windows|🚧 Planned|

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
troubleshooting, including a common udev rule-ordering pitfall on some distros.

After installing, launch `zzzzziglerrr` from your application launcher — it starts in the
system tray.

## Build from source

Requirements: Rust toolchain, Node.js (for the Tauri CLI), and Tauri's system dependencies
(`libwebkit2gtk-4.1-dev`, `libudev-dev`, and friends — see your distro's Tauri prerequisites).

```bash
npm install
npm run tauri dev                      # run in development
npm run tauri build -- --bundles deb   # produce a release .deb
```

No frontend build step — the UI is plain HTML/CSS/JS served as-is, no bundler involved.

## Why zzzzziglerrr?

|Aspect|zzzzziglerrr|Typical `xdotool`-based jiggler|
|---|---|---|
|Wayland support|✅ Native, via `uinput`|❌ `xdotool` requires X11|
|Recognized as real activity by ActivityWatch|✅ Confirmed via `org_kde_kwin_idle` (KDE/KWin)|❌ Synthetic input alone isn't counted|
|Screen stays awake|✅ Explicit DPMS/screensaver inhibit|⚠️ Depends on side effects, not guaranteed|
|Tray-based, no window clutter|✅|Varies|

## Known limitations

- Idle-detection and the activity-reset mechanism (`org_kde_kwin_idle.simulate_user_activity`)
  are KDE/KWin-specific. On other compositors, jiggling still moves the cursor via `uinput`,
  but presence-tracking tools may still report you as away.
- Only tested against [ActivityWatch](https://activitywatch.net/). It should not be assumed
  to behave identically with every presence/AFK tracker.
- X11 is not a primary target; Wayland is the focus.

## Status

Linux/KWin core feature set (P0) is complete and verified, including confirmed non-AFK
behavior in ActivityWatch during active jiggling, reproduced across two independent test
runs. Autostart-on-login and support for other compositors/platforms are in progress.

## License

No license has been published yet — this is currently a personal tool, not a published
open-source release. Treat the source as "all rights reserved" until a license is added.
