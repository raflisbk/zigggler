# Linux Platform Notes — P0 (Debian 13 trixie, KDE Plasma 6.3.6, KWin Wayland, kernel 6.12)

## 1. `/dev/uinput` access (udev rule)

Default state: `/dev/uinput` is `crw------- root:root` (mode 0600) — unprivileged users have
no access, writes fail with `EACCES`.

Fix: install `60-uinput.rules`, which uses **`TAG+="uaccess"` only** — systemd-logind grants
an ACL on `/dev/uinput` scoped to the active seat session's user. No `GROUP="input"` +
`usermod -aG input` — that was the first draft and got flagged in security review: group
`input` owns every `/dev/input/event*` node (real keyboard, real mouse, every HID), so
adding the user to it is a broad unrelated grant for a feature that only needs
`/dev/uinput`. `uaccess` scopes the grant to this one device node and nothing else.

**Rule filename matters**: must be numbered lower than `73-seat-late.rules` (the systemd
rule that runs the `uaccess` builtin on `TAG=="uaccess"`). A `99-*.rules` file sets the tag
*after* 73 already ran — ACL never applied. `60-uinput.rules` runs before 73, so the tag is
visible when logind's builtin checks it. Confirmed by testing both orderings on this
device: `99-` left `/dev/uinput` with no ACL (`crw-rw----`, still not writable by the
unprivileged user); `60-` produced `crw-rw----+` with `user:<name>:rw-` in `getfacl`.

```
sudo platform/linux/install-udev-rule.sh
```

What it does:
1. Copies `60-uinput.rules` to `/etc/udev/rules.d/`.
2. Reloads/triggers udev rules — logind applies the ACL to the active seat session
   immediately, **no relogin needed**.

Verify:
```
getfacl /dev/uinput   # should show user:<you>:rw-
test -w /dev/uinput && echo writable
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
