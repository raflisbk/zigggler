# Goals — zzzzziglerrr

Dokumen ini adalah source of truth buat eksekusi. Role agent WAJIB update bagian **Status & Log** tiap kali: fitur selesai, nemu bug, atau ambil keputusan arsitektur. Jangan tunggu sampai semua kerjaan kelar — update tiap increment.

> PRD lengkap: [[../PRD.md]]
> Mockup UI final (disetujui): [[../mockups/03-theme-language.html]] — ini source of truth tampilan, jangan desain ulang dari nol.

---

## 0. Prioritas Platform (WAJIB dibaca dulu)

**P0 — harus jalan duluan, di device ini:**
- OS: Debian 13 (trixie), KDE Plasma 6.3.6, kernel 6.12
- Compositor: **KWin (Wayland)**
- Ini target pertama. Semua fitur MVP diverifikasi jalan di sini SEBELUM ngerjain platform lain.

**P1 — abstraksi Linux lain (lanjut setelah P0 jalan):**
- Compositor: GNOME/Mutter, Sway
- Distro populer lain: Ubuntu, Fedora, Arch (cek packaging/udev rule beda-beda)

**P2 — cross-platform (lanjut setelah P0+P1 stabil):**
- macOS
- Windows

Jangan lompat ke P1/P2 kalau P0 belum lolos acceptance criteria MVP di bawah.

---

## 1. Role & Tab Herdr

| Role | Tab herdr | Scope |
|---|---|---|
| **rust-core** | `rust-core` | Mouse injection + idle detection abstraction (trait `MouseInjector`/`IdleWatcher`), implementasi Linux/uinput + Wayland `ext-idle-notify-v1` dulu (P0), scheduler, config persistence |
| **frontend-ui** | `frontend-ui` | Implementasi window Tauri dari mockup `03-theme-language.html` jadi functional (bukan statis) — tray, 2-view (main/theme-picker), i18n ID/EN/WB, theme CSS, wiring ke backend Rust via Tauri command/event |
| **platform-compat** | `platform-compat` | Testing lintas compositor (KWin→Mutter→Sway) dan lintas OS (macOS/Windows) pas udah masuk fase P1/P2. Dari awal: setup udev rule, dokumentasiin gotcha per environment |
| **qa-release** | `qa-release` | CI (GitHub Actions build check), packaging (.deb/AppImage dulu buat P0), review PR sebelum merge, pastiin checklist acceptance criteria per fitur sebelum approve |

PM (founder, di tab ini — `w1:t1`) koordinasi via `herdr pane run`, gak langsung coding.

---

## 2. Git Workflow (WAJIB diikuti semua role)

0. **WAJIB pakai `git worktree`, jangan `git checkout` di direktori utama.** Direktori project ini dipakai bareng-bareng (PM + 4 role). `git checkout <branch>` di direktori utama bisa numpuk/ngerusak kerjaan role lain yang kebetulan lagi pakai direktori yang sama (udah pernah kejadian — edit PM ketimpa). Sebelum mulai fitur baru: `git worktree add ../mouse-jiggler-<role>-<fitur> -b feat/<role>-<fitur> main`, kerja di path worktree itu, bukan di direktori utama.
1. **Satu fitur = satu branch.** Branch dari `main`, nama: `feat/<role>-<fitur-singkat>` (contoh: `feat/rust-core-uinput-jiggle`, `feat/frontend-ui-theme-picker`).
2. Kerja di branch itu (lewat worktree-nya) sampai fitur selesai + lolos acceptance criteria di goals ini. Abis PR di-merge, hapus worktree-nya (`git worktree remove ../mouse-jiggler-<role>-<fitur>`).
3. Commit dengan pesan jelas (conventional commits: `feat:`, `fix:`, `docs:`, dst).
4. `git push -u origin <branch>`.
5. Buka PR ke `main` via `gh pr create` — isi deskripsi: apa yang berubah, cara test manual, checklist acceptance criteria yang kecentang.
6. **qa-release** review dulu sebelum merge (kalau qa-release belum spawn/belum sempat, PM review).
7. Merge ke `main` (squash atau merge commit, konsisten), delete branch abis merge.
8. **Update `goals/goals.md`** (bagian Status & Log di bawah) — tambah baris baru, JANGAN timpa history lama.
9. Kalau nemu bug blocking di tengah kerja: jangan diemin, catat di Status & Log dengan tag `[BUG]`, lanjut kerja atau tanya PM via herdr kalau butuh keputusan.

**Tidak boleh**: commit langsung ke `main`, force push ke `main`, skip PR.

---

## 3. Platform Implementation Matrix

| Concern | Linux (P0: KWin) | Linux (P1: Mutter/Sway) | macOS (P2) | Windows (P2) |
|---|---|---|---|---|
| Mouse inject | `uinput` crate, butuh akses `/dev/uinput` (udev rule) | sama (uinput OS-level, gak beda per compositor) | `CGEventCreateMouseEvent`+`CGEventPost` (`core-graphics` crate) | `SendInput` (`windows` crate, `winuser`) |
| Idle detection | `ext-idle-notify-v1` via `wayland-client`, KWin 6+ native support | Mutter 44+ native; **Sway: cek versi wlroots, mungkin gak support protokol ini — siapkan fallback dbus `org.freedesktop.ScreenSaver`** | `CGEventSourceSecondsSinceLastEventType` | `GetLastInputInfo` (`user32`) |
| Tray | Tauri built-in tray API (cross-platform) | sama | sama, icon asset beda style (template image) | sama |
| Autostart | `.desktop` di `~/.config/autostart/` | sama | LaunchAgent plist `~/Library/LaunchAgents/` | shortcut di `shell:startup` atau registry `Run` |
| Config dir | `~/.config/zzzzziglerrr/` (`dirs::config_dir()`) | sama | `~/Library/Application Support/zzzzziglerrr/` | `%APPDATA%\zzzzziglerrr\` |
| Permission setup | udev rule (`/etc/udev/rules.d/99-uinput.rules`) + grup `input`, sekali seumur hidup | sama | Accessibility permission (System Settings → Privacy) buat CGEvent | biasanya gak perlu permission khusus buat SendInput |
| Packaging | `.deb` + `.AppImage` | sama, tapi tes di tiap distro populer | `.dmg` via `tauri build` | `.msi`/`.exe` via `tauri build` |

Implementasi kode: pakai trait abstraction (`trait MouseInjector { fn jiggle(&self); }`, `trait IdleWatcher { fn idle_seconds(&self) -> u64; }`) dengan `#[cfg(target_os = "linux")]` dst per modul, biar nambah platform baru gak ubah core logic.

---

## 4. MVP Task Breakdown (P0 — Linux/KWin, device ini)

### rust-core
- [ ] Scaffold `cargo create-tauri-app` (Rust backend + minimal frontend placeholder).
- [ ] Trait `MouseInjector` + implementasi Linux `UinputInjector` — relative move kecil (misal ±2px) lalu balik, biar gak keliatan geser beneran.
- [ ] Udev rule file `99-uinput.rules` + dokumentasi setup (`usermod -aG input $USER` + reboot/relogin).
- [ ] Trait `IdleWatcher` + implementasi Linux Wayland `ext-idle-notify-v1` via `wayland-client`/`smithay-client-toolkit`, test di KWin session ini.
- [ ] Scheduler: thread/tokio interval, baca jam aktif dari config, gate jiggle loop.
- [ ] Config struct (`theme`, `locale`, `interval_secs`, `schedule_start`, `schedule_end`, `idle_only`) + `serde_json` load/save ke `~/.config/zzzzziglerrr/config.json`.
- [ ] Tauri command buat expose state ke frontend (`get_config`, `set_config`, `toggle_jiggle`).
- **Acceptance**: jiggle beneran gerakin mouse di KWin Wayland session ini, app lain (coba test manual pakai status Teams/Slack kalau ada) ke-detect aktif. Idle detection gak infinite-loop (gak ke-trigger sama jigglenya sendiri).

### frontend-ui
- [ ] Port `mockups/03-theme-language.html` jadi `src/index.html` beneran di project Tauri — struktur sama, tapi switch/slider/tema/bahasa connect ke Tauri command (bukan cuma `state` JS lokal).
- [ ] Tray icon 3 state (off/waiting/jiggling) — bisa reuse warna dari tema aktif.
- [ ] Window close → hide ke tray (gak exit process).
- [ ] i18n: load locale dari config saat startup, bukan hardcode default "id".
- **Acceptance**: buka app, window persegi tanpa scroll (udah tervalidasi di mockup), ganti tema/bahasa dari titlebar icon beneran persist ke config.json, toggle/slider beneran ngontrol backend Rust (bukan simulasi visual doang).

### platform-compat (P0 scope dulu)
- [ ] Setup & dokumentasiin udev rule installer (script atau instruksi README).
- [ ] Verifikasi versi KWin di Plasma 6.3.6 beneran support `ext-idle-notify-v1` (test langsung, bukan asumsi dari PRD).
- [ ] Catat gotcha spesifik device ini (scaling 1749x983, dll) kalau ada yang mempengaruhi render webview.

### qa-release
- [ ] Setup GitHub Actions: build check tiap PR (`cargo build`, `cargo test` kalau ada).
- [ ] Review tiap PR dari rust-core/frontend-ui terhadap acceptance criteria di atas sebelum approve merge.
- [ ] Setelah P0 MVP semua acceptance checklist hijau: buat `.deb`/`.AppImage` build pertama, taruh di GitHub Release draft.

---

## 5. Status & Log

> Format tiap entry: `YYYY-MM-DD HH:MM [role] [tag] pesan — branch/PR kalau ada`
> Tag: `[DONE]` fitur selesai, `[BUG]` nemu masalah, `[DECISION]` keputusan arsitektur, `[BLOCKED]` butuh keputusan PM.

- 2026-10-19 [pm] [DECISION] Scope resmi: P0 Linux/KWin device ini dulu, P1 compositor Linux lain, P2 macOS/Windows. Role di-spawn via herdr tab terpisah (rust-core, frontend-ui, platform-compat, qa-release).
- 2026-10-19 [pm] [DECISION] Git workflow: branch per fitur (`feat/<role>-<fitur>`), commit → push → PR → review qa-release → merge main → update goals.md.
- 2026-10-19 [pm] [DECISION] Remote origin URL kena typo (`https:/` → `https://`), udah di-fix sebelum role mulai push.
- 2026-10-20 [platform-compat] [DONE] Udev rule installer (`platform/linux/99-uinput.rules` + `install-udev-rule.sh`) buat akses `/dev/uinput` (grup `input` + `uaccess`). Verifikasi `ext-idle-notify-v1` di KWin 6.3.6 device ini via `wayland-info` (interface `ext_idle_notifier_v1` version 1 tampil, bukan asumsi) — gak perlu fallback dbus buat P0. Gotcha scaling dicatat: scale 1.1, logical 1746x982 (deket sama 1749x983 di PRD), WebKitGTK rawan sub-pixel rounding di fractional scale — perlu dites khusus pas QA tema pixel-art. — PR #2 (`feat/platform-compat-udev-idle-docs`)

- 2026-10-20 [qa-release] [DONE] Review + merge PR #2 (platform-compat): acceptance criteria udev installer, idle protocol verification, scaling gotcha — semua lolos. Direktori utama ke-checkout ke branch rust-core pas review (numpuk kerjaan role lain), dibalikin ke `main` sesuai rule #0 worktree. qa-release pakai worktree terpisah mulai sekarang buat tiap review/kerjaan.

<!-- Role: tambah entry baru di bawah ini, jangan edit/hapus entry lama -->
