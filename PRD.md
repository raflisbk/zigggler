# PRD — zzzzziglerrr (Personal Tool)

## 1. Executive Summary

- **Problem**: Jiggler yang reliable di Wayland itu jarang (xdotool mati di Wayland), dan belum ada satu tool yang jalan konsisten lintas Linux (KWin/Mutter/Sway)/macOS/Windows dengan UI yang gak kaku — gw butuh gerakan mouse asli buat ngelewatin deteksi "away" di app kayak Teams/Slack, bukan cuma cegah screen lock.
- **Solution**: Tray app Rust+Tauri, mouse injection & idle detection per-platform (uinput/Wayland protocol di Linux, CGEvent di macOS, SendInput di Windows), UI settings persegi 1-window dengan 11 tema terkurasi + 3 bahasa slang (ID/EN/WB), fitur scheduler + idle detection biar gerakannya gak asal nyala terus.
- **Platform scope**: Linux distro populer (Ubuntu, Fedora, Debian, Arch) di Wayland — KWin, Mutter, Sway; macOS; Windows. Detail breakdown teknis per-platform ada di [[goals/goals.md]].
- **Success Criteria**:
  1. Jalan ≥ 8 jam nonstop background tanpa crash.
  2. RAM idle < 30MB, CPU idle ~0% (cuma spike pas jiggle event).
  3. App lain (Teams/Slack) ke-detect "active" selama jiggler nyala — verifikasi manual.
  4. Toggle on/off dan ganti interval dari tray tanpa restart app.
  5. Autostart login tanpa interaksi manual.

## 2. User Experience & Functionality

- **Persona**: Single user (gw sendiri), daily driver KDE Plasma Wayland, kerja WFH, butuh status "active" pas lagi fokus tanpa gerak mouse (meeting, reading, dll).

- **User Stories**:
  - Sebagai user, gw mau toggle jiggler on/off dari tray icon, biar gak perlu buka window tiap kali.
  - Sebagai user, gw mau atur interval gerakan (detik/menit) dari window settings, biar bisa nyesuain sama threshold "away" tiap app.
  - Sebagai user, gw mau set scheduler (jam mulai-selesai, misal 09:00-17:00) biar otomatis aktif pas jam kerja tanpa gw nyalain manual.
  - Sebagai user, gw mau jiggler cuma jalan kalau gw bener idle (idle detection), biar gak interfere pas lagi aktif pakai mouse.
  - Sebagai user, gw mau icon tray berubah visual (warna/animasi) biar tau status jiggler (aktif/idle-wait/off) tanpa buka window.
  - Sebagai user, gw mau app auto-start pas login, nempel di tray, zero manual launch.
  - Sebagai user, gw mau pilih tema window settings (15 arah dari eksplorasi: pixelated + chrome/metal/water), biar bisa nyesuain mood/selera tanpa update app.
  - Sebagai user, gw mau ganti bahasa UI (Indonesia/English/Jepang) langsung dari titlebar, copy-nya slang kayak ngomong ke bestie — bukan bahasa formal produk korporat.
  - Sebagai user, gw mau window settings-nya persegi & ringkas, gak ada scroll/overflow — semua kontrol kelihatan sekali pandang.

- **Acceptance Criteria**:
  - Tray icon punya 3 state visual beda: OFF, ON-waiting (idle belum tercapai / di luar jadwal), ON-jiggling.
  - Window settings bisa diakses dari klik tray icon, nutup ke tray (gak exit process) kalau di-close.
  - Perubahan interval/scheduler/idle-threshold tersimpan persist ke config file, ke-load lagi pas restart.
  - Idle detection pakai idle time asli dari compositor (bukan polling posisi mouse sendiri, karena itu bakal ke-trigger oleh jiggler sendiri → infinite loop).
  - Scheduler overlap dengan idle detection: jiggler cuma jalan kalau DUA kondisi true (dalam jadwal DAN user idle).
  - Theme picker: 11 pilihan terkurasi, urutan tetap — Game Boy, NES Arcade, DOS Phosphor, Mercury, Gold, Obsidian Glass, Liquid Glass, Holographic, Sketchbook, Pastel Blob, Comic Outline. Diakses lewat ikon bulat kecil di titlebar (bukan panel terpisah) — klik buka grid tema di dalam window yang sama, pilih satu → otomatis balik ke view utama. Ganti tema langsung apply tanpa restart, pilihan tersimpan persist.
  - Language picker: 3 pilihan — Indonesia (indikator "ID"), English ("EN"), Jepang ("WB") — ditampilkan sebagai chip kecil di titlebar, selalu kelihatan tanpa buka sub-view. Semua nada slang/compact. Ganti bahasa langsung apply ke semua label/copy tanpa restart, pilihan tersimpan persist.
  - Window settings berbentuk persegi (300×300), `overflow: hidden` strict — gak ada scrollbar dalam kondisi apapun, konten didesain fit di satu layar window.

- **Non-Goals**:
  - Bukan multi-user / multi-device sync.
  - Bukan dukungan X11 primer (fokus Wayland; X11 boleh jalan sebagai bonus fallback via xdotool-equivalent, bukan target testing utama).
  - Bukan distribusi publik ke store resmi (Microsoft Store/Mac App Store) — cukup installer manual (.deb/AppImage, .dmg, .msi) + GitHub Releases.
  - Bukan "anti-detection" canggih (gak perlu randomize pattern buat ngelewatin anti-cheat/anti-fraud system — ini cuma buat status presence app chat).

## 3. Technical Specifications

- **Theming & Copy**:
  - 11 tema diimplementasi sebagai CSS class/token set yang di-swap via JS (Tauri webview) — bukan rebuild binary per tema.
  - Tipografi & copy disesuaikan buat nada slang/compact: contoh ID "lagi nge-jig" (bukan "sedang aktif menggerakkan mouse"), "woles dulu" (bukan "tidak aktif"), "cus balik" (bukan "kembali"). Nama brand "zzzzziglerrr" tetap gak diterjemahkan di semua locale.
  - i18n sederhana: 1 dictionary object per locale (id/en/ja), di-apply via JS `textContent` swap, gak perlu library i18n eksternal (scope cuma 3 bahasa, ~12 string).
  - Layout di dalam window dirancang 2-view: `main` (kontrol jiggle/interval/jadwal/idle/status) dan `theme-picker` (grid 11 swatch), di-toggle via tombol ikon di titlebar — bukan ditumpuk semua di satu scrollable view.

- **Tech Stack**:
  - **Bahasa**: Rust.
  - **UI**: Tauri v2 — webview (HTML/CSS/SVG) buat window settings persegi 1-window, built-in tray API (gak perlu crate tray terpisah), cross-platform by default.
  - **Mouse injection (per platform)** — abstraksi via trait `MouseInjector`, implementasi beda per OS, dipilih lewat `#[cfg(target_os = ...)]`:
    - Linux: crate `uinput` — virtual input device, independen compositor/Wayland protocol.
    - macOS: `CGEventCreateMouseEvent` + `CGEventPost` via crate `core-graphics`.
    - Windows: `SendInput` via crate `windows` (win32 `winuser`).
  - **Idle detection (per platform)** — trait `IdleWatcher`:
    - Linux Wayland: protokol `ext-idle-notify-v1` via `wayland-client`. Support beda per compositor (lihat [[goals/goals.md]] § Platform Matrix) — KWin 6+ native, Mutter/GNOME 44+ native, Sway butuh cek versi wlroots. Fallback: dbus `org.freedesktop.ScreenSaver` kalau protokol gak tersedia.
    - macOS: `CGEventSourceSecondsSinceLastEventType`.
    - Windows: `GetLastInputInfo` (user32).
  - **Scheduler**: `std::time` + background thread/tokio interval, bandingin jam sekarang ke window jadwal tersimpan. Platform-agnostic, gak perlu abstraksi khusus.
  - **Config persistence**: `serde_json` + `dirs` crate → config dir OS-native (`dirs::config_dir()` resolve otomatis ke `~/.config/zzzzziglerrr` Linux, `~/Library/Application Support/zzzzziglerrr` macOS, `%APPDATA%\zzzzziglerrr` Windows).
  - **Autostart (per platform)**: Linux `.desktop` di `~/.config/autostart/`; macOS LaunchAgent plist di `~/Library/LaunchAgents/`; Windows shortcut di `shell:startup` atau registry `Run` key.
  - **Theme + locale persistence**: disimpen bareng config lain (field `theme` dan `locale`), gak perlu storage terpisah.

- **Architecture Overview**:
  ```
  [Tray Icon] <-> [Tauri Core/State] <-> [Scheduler Thread] -> gate check
                        |                [Idle Watcher Thread] -> gate check
                        v
                  [uinput Writer] -> /dev/uinput -> kernel -> compositor -> app lain baca event
  ```
  Jiggle loop jalan tiap tick cek 2 gate (scheduler + idle) sebelum kirim event uinput.

- **Permission requirement**: akses `/dev/uinput` butuh user masuk grup `input` + udev rule custom (`TAG+="uaccess"` atau grup rule), setup sekali saat instalasi, didokumentasikan di README (bukan runtime step).

- **Security & Privacy**: gak ada data keluar device, gak ada network call, config cuma lokal. Satu-satunya privilege sensitif: akses raw input device (uinput) — scope-nya cuma inject event, gak baca input asli (gak jadi keylogger).

## 4. Risks & Roadmap

- **Phased Rollout**:
  - **MVP**: tray toggle on/off + interval, uinput jiggle, autostart.
  - **v1.1**: scheduler + idle detection (2 gate system).
  - **v1.2**: UI window settings penuh (persegi, 2-view) + tray icon state visual (warna/animasi 3 state) + theme picker (11 tema terkurasi, in-window) + language picker (ID/EN/WB slang). Mockup UI udah final & disetujui.
  - **v2.0 (open, "lebih advance")**: ide lanjutan kalau masih kepake — misal profile per-app (beda interval buat beda keperluan), stats "total jam aktif", global hotkey toggle.

- **Technical Risks**:
  - `ext-idle-notify-v1` butuh KWin versi cukup baru — kalau KWin version di Plasma 6.3.6 belum support penuh, fallback ke polling idle via dbus `org.kde.kwin` (kalau ada) atau gugurin fitur idle-detection ke v1.2+ tanpa blocking MVP.
  - Udev rule akses `/dev/uinput` beda-beda tergantung distro — perlu dites di Debian trixie spesifik punya user.
  - Tauri webview di KDE Wayland pakai WebKitGTK — perlu dites render-nya gak glitch di scaling display custom user (1749x983 effective scaling).
