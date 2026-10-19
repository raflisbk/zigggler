#!/usr/bin/env bash
set -euo pipefail

RULE_SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/99-uinput.rules"
RULE_DST="/etc/udev/rules.d/99-uinput.rules"

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Perlu root. Jalankan ulang: sudo $0" >&2
  exit 1
fi

cp "$RULE_SRC" "$RULE_DST"
udevadm control --reload-rules
udevadm trigger --subsystem-match=misc --action=add

TARGET_USER="${SUDO_USER:-$USER}"
usermod -aG input "$TARGET_USER"

echo "Udev rule terpasang: $RULE_DST"
echo "User '$TARGET_USER' ditambah ke grup 'input'."
echo "Butuh LOGOUT/LOGIN ulang (atau reboot) biar membership grup kebaca sesi aktif."
