#!/bin/sh
# SSH and SFTP for slot, through BaseOS's dropbear and sftp-server.
#
#   slotssh.sh on              start dropbear with host keys on /data
#   slotssh.sh off             stop dropbear
#   slotssh.sh status          check if dropbear is running
#   slotssh.sh password [PASS] set root's password in shadow
set -u

DROPBEAR="${AGS_DROPBEAR:-dropbear}"
DROPBEARKEY="${AGS_DROPBEARKEY:-dropbearkey}"
PIDOF="${AGS_PIDOF:-pidof}"
KILLALL="${AGS_KILLALL:-killall}"
RUN="${AGS_RUN:-/run}"
DATA="${AGS_DATA:-/data}"
SHADOW="${AGS_SHADOW:-/run/shadow}"
[ -f "$SHADOW" ] || SHADOW="/etc/shadow"
MKPASSWD="${AGS_MKPASSWD:-mkpasswd}"

DROPBEAR_PID="$RUN/dropbear.pid"
LOG="$RUN/slot-dropbear.log"

alive() {
	[ -f "$1" ] && kill -0 "$(cat "$1")" 2>/dev/null
}

stop_pid() {
	[ -f "$1" ] || return 0
	kill "$(cat "$1")" 2>/dev/null || true
	/bin/rm -f "$1"
}

ssh_on() {
	alive "$DROPBEAR_PID" && return 0
	$PIDOF "$(basename "$DROPBEAR")" >/dev/null 2>&1 && return 0

	/bin/mkdir -p "$DATA/dropbear"
	rsa="$DATA/dropbear/dropbear_rsa_host_key"
	ed="$DATA/dropbear/dropbear_ed25519_host_key"
	[ -f "$rsa" ] || $DROPBEARKEY -t rsa -f "$rsa" >/dev/null 2>&1 || true
	[ -f "$ed" ] || $DROPBEARKEY -t ed25519 -f "$ed" >/dev/null 2>&1 || true

	$DROPBEAR -r "$rsa" -r "$ed" -B -W 131072 -P "$DROPBEAR_PID" \
		< /dev/null > "$LOG" 2>&1 || {
		echo "dropbear did not start" >&2
		return 1
	}
	return 0
}

ssh_off() {
	if [ -f "$DROPBEAR_PID" ]; then
		stop_pid "$DROPBEAR_PID"
	fi
	$KILLALL "$(basename "$DROPBEAR")" 2>/dev/null || true
	return 0
}

ssh_status() {
	if alive "$DROPBEAR_PID" || $PIDOF "$(basename "$DROPBEAR")" >/dev/null 2>&1; then
		echo "running"
		return 0
	else
		echo "stopped"
		return 1
	fi
}

hash_password() {
	pw="$1"
	if command -v "$MKPASSWD" >/dev/null 2>&1; then
		printf '%s' "$pw" | $MKPASSWD -m sha512 -P 0 2>/dev/null && return 0
	fi
	if command -v openssl >/dev/null 2>&1; then
		openssl passwd -6 "$pw" 2>/dev/null && return 0
	fi
	if command -v python3 >/dev/null 2>&1; then
		python3 -c "import crypt; print(crypt.crypt('$pw', crypt.mksalt(crypt.METHOD_SHA512)))" 2>/dev/null && return 0
	fi
	return 1
}

set_password() {
	pw="${2:-}"
	if [ -z "$pw" ]; then
		IFS= read -r pw || [ -n "$pw" ] || { echo "no password given" >&2; return 1; }
	fi
	hash="$(hash_password "$pw")" || { echo "could not hash password" >&2; return 1; }
	[ -n "$hash" ] || return 1

	if [ ! -f "$SHADOW" ]; then
		echo "no shadow file at $SHADOW" >&2
		return 1
	fi

	awk -F: -v h="$hash" 'BEGIN { OFS = ":" } $1 == "root" { $2 = h } { print }' \
		"$SHADOW" > "$SHADOW.slot" || return 1
	cat "$SHADOW.slot" > "$SHADOW"
	/bin/rm -f "$SHADOW.slot"
	return 0
}

case "${1:-}" in
on) ssh_on ;;
off) ssh_off ;;
status) ssh_status ;;
password) set_password "$@" ;;
*)
	echo "usage: slotssh.sh on | off | status | password [PASS]" >&2
	exit 2
	;;
esac
