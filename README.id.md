# zzzzziglerrr

![release](https://img.shields.io/github/v/release/raflisbk/zigggler?label=release)
![CI](https://github.com/raflisbk/zigggler/actions/workflows/ci.yml/badge.svg)
![platform](https://img.shields.io/badge/platform-Linux%20%7C%20KDE%2FKWin-blue)
![license](https://img.shields.io/badge/license-unreleased-lightgrey)

**Gerakkan kursor seperti aktivitas asli. Dibuat untuk KDE Wayland.**

[Releases](https://github.com/raflisbk/zigggler/releases/latest) · [Instalasi](#instalasi-linux-berbasis-debian) · [Build dari source](#build-dari-source) · [Batasan](#batasan-yang-diketahui) · [Status](#status)

*Read in English: [README.md](README.md)*

---

Kebanyakan mouse jiggler mengandalkan `xdotool`, yang tidak berfungsi di Wayland.
`zzzzziglerrr` menggerakkan kursor melalui perangkat virtual `uinput`, dan di KDE
Plasma/KWin juga mengirim sinyal aktivitas seat yang sesungguhnya lewat protokol
`org_kde_kwin_idle` — sehingga alat pemantau presensi seperti ActivityWatch membaca sinyal
aktivitas asli, bukan sekadar kursor yang bergetar.

Terkonfirmasi di produksi: ActivityWatch tetap non-AFK selama dua sesi tes independen
60 menit dan 31 menit, tanpa input manusia sama sekali. Lihat [Status](#status).

## Fitur

- Toggle jiggle on/off dari ikon tray atau window settings — status selalu sinkron
- Interval jiggle dapat diatur
- Jiggle digerbang oleh status idle — hanya aktif saat benar-benar idle, tidak mengganggu saat sedang bekerja
- Ikon tray merefleksikan status real-time: off / menunggu idle / sedang jiggle
- Screensaver/DPMS dicegah selama jiggle aktif (`org.freedesktop.ScreenSaver.Inhibit`)
- 11 tema terkurasi, 3 bahasa (Indonesia / Inggris / Jepang), dapat diganti tanpa restart
- Menutup window menyembunyikannya ke tray — aplikasi tetap berjalan sampai dimatikan dari menu tray

## Dukungan Platform

|Platform|Status|
|---|---|
|Linux — KDE Plasma / KWin (Wayland)|✅ Didukung, terverifikasi|
|Linux — compositor Wayland lain (GNOME/Mutter, Sway)|🚧 Direncanakan|
|macOS|🚧 Direncanakan|
|Windows|🚧 Direncanakan|

## Instalasi (Linux berbasis Debian)

Paket `.deb` siap pakai dipublikasikan di [halaman Releases](https://github.com/raflisbk/zigggler/releases/latest).

```bash
# Unduh .deb terbaru dari halaman Releases, lalu:
sudo dpkg -i zzzzziglerrr_*.deb
# Jika dpkg mengeluhkan dependency yang kurang:
sudo apt --fix-broken install
```

### Setup sekali: akses `/dev/uinput`

Fitur jiggle menulis event input sintetis melalui `/dev/uinput`, yang secara default dimiliki
root. Sebuah udev rule memberi akses ke sesi user Anda tanpa menambahkan ke grup device yang luas:

```bash
sudo platform/linux/install-udev-rule.sh
```

Langsung berlaku untuk sesi Anda saat ini — tidak perlu reboot atau login ulang. Verifikasi:

```bash
getfacl /dev/uinput   # harus menunjukkan user:<anda>:rw-
```

Lihat [`platform/linux/README.md`](platform/linux/README.md) untuk penjelasan lengkap dan
troubleshooting, termasuk gotcha urutan nomor udev rule yang umum terjadi di beberapa distro.

Setelah terinstal, jalankan `zzzzziglerrr` dari application launcher — aplikasi akan langsung
masuk ke system tray.

## Build dari source

Dibutuhkan: Rust toolchain, Node.js (untuk Tauri CLI), dan dependency sistem Tauri
(`libwebkit2gtk-4.1-dev`, `libudev-dev`, dkk — cek prasyarat Tauri untuk distro Anda).

```bash
npm install
npm run tauri dev                      # jalankan versi development
npm run tauri build -- --bundles deb   # buat .deb release
```

Tidak ada build step untuk frontend — UI berupa HTML/CSS/JS polos yang disajikan langsung,
tanpa bundler.

## Kenapa zzzzziglerrr?

|Aspek|zzzzziglerrr|Jiggler berbasis `xdotool` umumnya|
|---|---|---|
|Dukungan Wayland|✅ Native, via `uinput`|❌ `xdotool` butuh X11|
|Dikenali sebagai aktivitas asli oleh ActivityWatch|✅ Terkonfirmasi via `org_kde_kwin_idle` (KDE/KWin)|❌ Input sintetis saja tidak terhitung|
|Layar tetap menyala|✅ Inhibit DPMS/screensaver eksplisit|⚠️ Mengandalkan efek samping, tidak terjamin|
|Berbasis tray, tidak ada window yang mengganggu|✅|Beragam|

## Batasan yang diketahui

- Deteksi idle dan mekanisme reset aktivitas (`org_kde_kwin_idle.simulate_user_activity`)
  khusus KDE/KWin. Di compositor lain, jiggle tetap menggerakkan kursor via `uinput`, tapi
  alat pemantau presensi mungkin masih menganggap Anda away.
- Baru diuji di [ActivityWatch](https://activitywatch.net/). Jangan asumsikan bekerja
  identik di semua tracker presensi/AFK.
- X11 bukan target utama; fokus pada Wayland.

## Status

Fitur inti Linux/KWin (P0) sudah selesai dan terverifikasi, termasuk konfirmasi perilaku
non-AFK di ActivityWatch selama jiggle aktif, direproduksi di dua sesi tes independen.
Autostart saat login dan dukungan compositor/platform lain masih dalam pengerjaan.

## Lisensi

Belum ada lisensi yang dipublikasikan — ini masih personal tool, belum rilis open-source
resmi. Anggap source code "all rights reserved" sampai lisensi ditambahkan.
