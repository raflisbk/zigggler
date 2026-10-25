# zzzzziglerrr

![release](https://img.shields.io/github/v/release/raflisbk/zigggler?label=release)
![CI](https://github.com/raflisbk/zigggler/actions/workflows/ci.yml/badge.svg)
![platform](https://img.shields.io/badge/platform-Linux%20%7C%20KDE%2FKWin-blue)
![license](https://img.shields.io/github/license/raflisbk/zigggler)

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

### Windows idle diagnostics (opt-in)

The `windows-diagnostics` Cargo feature records the real app's idle gate and
`SendInput` results. It is disabled in normal builds. In PowerShell on the
Windows desktop, build and test it with:

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml --features windows-diagnostics
cargo build --locked --manifest-path src-tauri/Cargo.toml --features windows-diagnostics
```

Quit the installed app from its tray menu before launching this build. Keep
ActivityWatch running in the same interactive Windows session; launching the app
directly inside an SSH session is not equivalent to launching it on the desktop.

```powershell
$env:ZZZZZIGLERRR_DIAGNOSTICS_LOG = Join-Path $env:TEMP ("jiggler-{0}.jsonl" -f (Get-Date -Format yyyyMMdd-HHmmss))
try {
    & .\src-tauri\target\debug\zzzzziglerrr.exe
} finally {
    Remove-Item Env:ZZZZZIGLERRR_DIAGNOSTICS_LOG -ErrorAction SilentlyContinue
}
```

Use the app's controls to enable jiggling and select settings. For idle-only
testing, use a 5-second interval, enable idle-only, disable random breaks, and
leave the desktop untouched for 90 seconds. Review repeated closed gates below
8 seconds, open gates at or above 8 seconds, and resets following each injection.
A reset is supported by the raw last-input timestamp changing and idle time
dropping; `sent == requested` alone is insufficient. Input from another source
confounds attribution. A 90-second test alone cannot establish ActivityWatch
coverage beyond its normal 180-second AFK timeout.

Logs contain process/session IDs, effective loop settings, input timestamps,
API results and per-injection samples. Each injection's follow-up sampling adds
about one second to the configured interval. The first loop can still use
settings captured before startup activation; analyze the actual loop settings.

For automated trials, `ZZZZZIGLERRR_DIAGNOSTICS_ENABLE=1` enables the real toggle.
`ZZZZZIGLERRR_DIAGNOSTICS_CONFIG` optionally names a JSON config applied through
the normal settings handler: **it overwrites the persistent configuration** and
can update the autostart entry to the diagnostic executable. Back up and restore
both the config and original autostart value around every automated trial. Do
not set these variables globally. Restarting the installed app starts jiggling
OFF; autostart does not automatically enable it.

This instrumentation does not change the input API or establish the cause of
Task #14's historical prolonged-AFK report. Controlled trials demonstrate idle
resets with idle-only disabled. A user-run 90-second Windows desktop trial of
diagnostic commit `234e39c` also demonstrated repeated idle-only gating with a
5-second interval and random breaks disabled: gates closed at 5-6 seconds of
idle, reopened at 10-11 seconds, and all seven matching injection cycles sent
2/2 inputs and immediately reset the sampled idle time to zero. One initial
cycle retained the previous random-break setting and was excluded; additional
input-timestamp changes during that initial cycle remain unattributed. The
runner reported the installed app alive more than 130 seconds after restart.
This trial does not establish ActivityWatch coverage beyond its normal
180-second timeout or explain the earlier prolonged-AFK interval.

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

[MIT](LICENSE)
