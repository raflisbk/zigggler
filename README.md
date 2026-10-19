# zzzzziglerrr

Mouse jiggler buat Linux (Wayland), dibuat pake Rust + Tauri. Gerakin mouse dikit tiap interval biar status "active" di app chat (Teams/Slack) gak jadi "away" pas lo lagi gak megang mouse — plus scheduler, idle detection, 11 tema, dan bahasa ID/EN/WB.

## Status

Masih in-development. Target platform pertama: Linux KDE Plasma/KWin (Wayland).

## Jalanin lokal

```
cargo tauri dev
```

Butuh Rust toolchain + dependency Tauri (webkit2gtk dkk) keinstal duluan.

## Setup `/dev/uinput`

Lihat `platform/linux/README.md` buat instalasi udev rule yang dibutuhin fitur jiggle.
