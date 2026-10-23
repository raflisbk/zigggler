# zzzzziglerrr

Mouse jiggler yang andal untuk Linux (Wayland), dibuat dengan Rust + Tauri.

Kebanyakan jiggler mengandalkan `xdotool`, yang tidak berfungsi di Wayland. `zzzzziglerrr`
menggerakkan kursor melalui perangkat virtual `uinput`, dan di KDE Plasma/KWin juga mengirim
sinyal aktivitas seat yang sesungguhnya lewat protokol `org_kde_kwin_idle` — sehingga alat
pemantau presensi seperti ActivityWatch membaca sinyal aktivitas asli, bukan sekadar kursor
yang bergetar.

*Read in English: [README.md](README.md)*

## Fitur

- Toggle jiggle on/off dari ikon tray atau window settings (status selalu sinkron)
- Interval jiggle dapat diatur
- Jiggle digerbang oleh status idle — hanya aktif saat benar-benar idle, tidak mengganggu saat sedang bekerja
- Ikon tray merefleksikan status real-time: off / menunggu idle / sedang jiggle
- Screensaver/DPMS dicegah selama jiggle aktif (`org.freedesktop.ScreenSaver.Inhibit`)
- 11 tema terkurasi, 3 bahasa (Indonesia / Inggris / Jepang), dapat diganti tanpa restart
- Menutup window menyembunyikannya ke tray — aplikasi tetap berjalan sampai dimatikan dari menu tray

## Dukungan Platform

| Platform | Status |
|---|---|
| Linux — KDE Plasma / KWin (Wayland) | ✅ Didukung, terverifikasi |
| Linux — compositor Wayland lain (GNOME/Mutter, Sway) | 🚧 Direncanakan |
| macOS | 🚧 Direncanakan |
| Windows | 🚧 Direncanakan |

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
troubleshooting (termasuk gotcha urutan nomor udev rule yang umum terjadi di beberapa distro).

Setelah terinstal, jalankan `zzzzziglerrr` dari application launcher — aplikasi akan langsung
masuk ke system tray.

## Build dari source

Dibutuhkan: Rust toolchain, Node.js (untuk Tauri CLI), dan dependency sistem Tauri
(`libwebkit2gtk-4.1-dev`, `libudev-dev`, dkk — cek prasyarat Tauri untuk distro Anda).

```bash
npm install
npm run tauri dev              # jalankan versi development
npm run tauri build -- --bundles deb   # buat .deb release
```

Tidak ada build step untuk frontend — UI berupa HTML/CSS/JS polos yang disajikan langsung,
tanpa bundler.

## Batasan yang diketahui

- Deteksi idle dan mekanisme reset aktivitas (`org_kde_kwin_idle.simulate_user_activity`)
  khusus KDE/KWin. Di compositor lain, jiggle tetap menggerakkan kursor via `uinput`, tapi
  alat pemantau presensi mungkin masih menganggap Anda away.
- Belum diuji di alat selain [ActivityWatch](https://activitywatch.net/). Jangan asumsikan
  bekerja identik di semua tracker presensi/AFK.
- X11 bukan target utama; fokus pada Wayland.

## Status

Fitur inti Linux/KWin (P0) sudah selesai dan terverifikasi, termasuk konfirmasi perilaku
non-AFK di ActivityWatch selama jiggle aktif. Autostart saat login dan dukungan
compositor/platform lain masih dalam pengerjaan — lihat [`docs/goals.md`](docs/goals.md)
jika Anda punya akses lokal (file ini gitignored, tidak termasuk riwayat publik repo).

## Lisensi

Belum ada file lisensi — ini masih personal tool, belum rilis open-source resmi. Anggap
source code "all rights reserved" sampai lisensi ditambahkan.
