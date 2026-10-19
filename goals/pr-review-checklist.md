# PR Review Checklist — qa-release

Pakai ini pas review PR dari rust-core/frontend-ui/platform-compat. Checklist diturunin langsung dari `goals/goals.md` § 4. Isi checklist feature yang relevan, sisanya skip.

Non-negotiable per PR (semua role):
- [ ] CI hijau (`cargo build`, `cargo test`)
- [ ] Branch sesuai konvensi `feat/<role>-<fitur>`
- [ ] Commit tanpa trailer `Co-Authored-By`
- [ ] PR description ada: apa yang berubah + cara test manual
- [ ] Kalau nyentuh platform abstraction (`MouseInjector`/`IdleWatcher`): pakai trait + `#[cfg(target_os = ...)]`, gak hardcode Linux-only di core logic

---

## rust-core

### uinput jiggle
- [ ] `MouseInjector` trait + `UinputInjector` jalan, relative move kecil (±2px) lalu balik
- [ ] Udev rule `99-uinput.rules` ada + dokumentasi (`usermod -aG input $USER` + reboot/relogin)
- [ ] Test manual: jiggle beneran gerakin mouse di KWin Wayland session

### idle detection
- [ ] `IdleWatcher` trait + implementasi `ext-idle-notify-v1` via `wayland-client`/`smithay-client-toolkit`
- [ ] Test manual di KWin session: idle detection gak ke-trigger sama jiggle-nya sendiri (no infinite loop)

### scheduler + config
- [ ] Scheduler baca jam aktif dari config, gate jiggle loop
- [ ] Config struct (`theme`, `locale`, `interval_secs`, `schedule_start`, `schedule_end`, `idle_only`) + `serde_json` load/save ke `~/.config/zzzzziglerrr/config.json`
- [ ] Jiggle jalan HANYA kalau 2 gate true: dalam jadwal DAN user idle
- [ ] Tauri command (`get_config`, `set_config`, `toggle_jiggle`) expose ke frontend

---

## frontend-ui

- [ ] Port dari `mockups/03-theme-language.html` — window persegi 300×300, `overflow: hidden`, no scrollbar kondisi apapun
- [ ] Tray icon 3 state visual: OFF / ON-waiting / ON-jiggling, warna sesuai tema aktif
- [ ] Window close → hide ke tray, gak exit process
- [ ] i18n load locale dari config saat startup (gak hardcode default "id")
- [ ] Ganti tema dari titlebar icon → apply langsung tanpa restart, persist ke config.json
- [ ] Ganti bahasa dari titlebar chip → apply langsung tanpa restart, persist ke config.json
- [ ] Toggle/slider interval beneran ngontrol backend Rust (bukan simulasi visual doang)
- [ ] 2-view structure: `main` dan `theme-picker`, toggle via titlebar icon

---

## platform-compat (P0 scope)

- [ ] Udev rule installer (script/README) terdokumentasi
- [ ] Verifikasi langsung (bukan asumsi) KWin Plasma 6.3.6 support `ext-idle-notify-v1`
- [ ] Gotcha device-specific (scaling 1749x983, webview render glitch) tercatat kalau ada

---

## Sign-off

Jangan approve/merge kalau ada checklist relevan yang belum kecentang. Kalau ada checklist gak lolos tapi non-blocking, catat di komentar PR + goals.md `[BUG]` tag, boleh merge dengan follow-up issue — tapi itu keputusan PM, bukan default qa-release.
