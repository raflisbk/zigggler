# Linux Platform Notes — P0 (Debian 13 trixie, KDE Plasma 6.3.6, KWin Wayland, kernel 6.12)

## 1. `/dev/uinput` access (udev rule)

Default state: `/dev/uinput` is `crw------- root:root` (mode 0600) — unprivileged users have
no access, writes fail with `EACCES`.

Fix: install `99-uinput.rules`, which grants group `input` read/write plus `uaccess` tag
(logind ACL, covers seat-based access too).

```
sudo platform/linux/install-udev-rule.sh
```

What it does:
1. Copies `99-uinput.rules` to `/etc/udev/rules.d/`.
2. Reloads/triggers udev rules.
3. Adds the invoking user to group `input`.

**Group membership only takes effect on next login session** (new `id`/`groups` output) —
log out/in or reboot after running the script, before the app can open `/dev/uinput`.

Verify after relogin:
```
groups        # should list "input"
ls -l /dev/uinput   # should show group "input", mode 660
```

## 2. Idle detection protocol — verified, not assumed

Checked directly on this device via `wayland-info`, not inferred from docs:

```
interface: 'ext_idle_notifier_v1', version: 1, name: 20
interface: 'org_kde_kwin_idle',   version: 1, name: 18   (legacy KDE-specific, fallback only)
```

`kwin --version` → `kwin 6.3.6`. **Confirmed: this KWin build natively advertises
`ext_idle_notifier_v1`.** No dbus `org.freedesktop.ScreenSaver` fallback needed for P0 —
implement `IdleWatcher` directly against `ext_idle_notifier_v1` via `wayland-client`.

`org_kde_kwin_idle` is also present; keep as a secondary fallback path only if a future
non-KDE Wayland target (P1 Sway) lacks `ext_idle_notifier_v1`.

## 3. Device-specific gotcha: fractional scaling

```
kscreen-doctor -o:
  Modes: 1920x1080@60 (native)
  Geometry: 0,0 1746x982   (logical, post-scale)
  Scale: 1.1
```

Physical panel is 1920x1080, but KWin reports **scale 1.1**, giving a logical/effective
size of **1746x982** (PRD referenced this as ~1749x983 — same fractional-scale rounding,
not a typo elsewhere).

Impact on frontend-ui: the fixed 300×300 `overflow:hidden` window target is specified in
logical px, but WebKitGTK (Tauri's Linux webview) renders through this 1.1 fractional
scale — sub-pixel rounding at non-integer scale factors is a known WebKitGTK rough edge
(blurry edges, 1px seams on borders/pixel-art theme assets). **Test the pixel-art themes
(Game Boy, NES Arcade, DOS Phosphor) specifically at this scale factor** before signing
off frontend-ui acceptance — don't rely on testing at scale 1.0/2.0 only.

## 4. Summary for acceptance criteria

| Check | Result |
|---|---|
| `/dev/uinput` installer works, documented | done — see `install-udev-rule.sh` |
| `ext-idle-notify-v1` supported on this KWin | confirmed via `wayland-info`, not assumed |
| Device gotchas documented | fractional scale 1.1 (1746x982 logical) noted for webview QA |
