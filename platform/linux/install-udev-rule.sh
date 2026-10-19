#!/usr/bin/env bash
set -euo pipefail

RULE_SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/60-uinput.rules"
RULE_DST="/etc/udev/rules.d/60-uinput.rules"

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Perlu root. Jalankan ulang: sudo $0" >&2
  exit 1
fi

cp "$RULE_SRC" "$RULE_DST"
udevadm control --reload-rules
udevadm trigger --subsystem-match=misc --action=add

echo "Udev rule terpasang: $RULE_DST"
echo "Akses /dev/uinput lewat logind ACL (uaccess) — scoped ke device uinput doang,"
echo "gak ngasih akses /dev/input/event* (keyboard/mouse asli)."
echo "Login session aktif (seat) langsung kebaca akses barunya, gak perlu relogin."
