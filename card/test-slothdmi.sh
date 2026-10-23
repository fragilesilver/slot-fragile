#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/System/slothdmi.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
FAILS=0

fail() {
	echo "FAIL: $*"
	FAILS=$((FAILS + 1))
}

reset() {
	rm -rf "$TMP"/*
	mkdir -p "$TMP/extcon/hdmi" "$TMP/dispdbg" "$TMP/backlight/panel"
	touch "$TMP/dispdbg/name" "$TMP/dispdbg/command" "$TMP/dispdbg/param" "$TMP/dispdbg/start"
	echo "128" > "$TMP/backlight/panel/brightness"
}

run() {
	AGS_EXTCON_HDMI_STATE="$TMP/extcon/hdmi/state" \
	AGS_DISPDBG_DIR="$TMP/dispdbg" \
	AGS_BACKLIGHT_DIR="$TMP/backlight" \
	sh "$SCRIPT" "$@"
}

# 1. Probe when present vs absent
reset
echo "0" > "$TMP/extcon/hdmi/state"
run probe && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "probe should succeed when extcon exists"

rm -rf "$TMP/extcon"
run probe && rc=0 || rc=$?
[ "$rc" = 1 ] || fail "probe should fail when extcon is missing"

# 2. Status when disconnected
reset
echo "0" > "$TMP/extcon/hdmi/state"
status_out="$(run status 2>&1 || true)"
[ "$status_out" = "disconnected" ] || fail "status expected disconnected, got $status_out"

# 3. Status when connected
reset
echo "HDMI=1" > "$TMP/extcon/hdmi/state"
status_out="$(run status 2>&1 || true)"
[ "$status_out" = "connected" ] || fail "status expected connected, got $status_out"

# 4. on (default 720p)
reset
echo "HDMI=1" > "$TMP/extcon/hdmi/state"
run on 720p && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "on 720p failed with $rc"
grep -q "disp0" "$TMP/dispdbg/name" || fail "dispdbg name not disp0"
grep -q "switch1" "$TMP/dispdbg/command" || fail "dispdbg command not switch1"
grep -q "4 10" "$TMP/dispdbg/param" || fail "dispdbg param not 4 10"
grep -q "1" "$TMP/dispdbg/start" || fail "dispdbg start not 1"
grep -q "0" "$TMP/backlight/panel/brightness" || fail "backlight not zeroed"

# 5. on 1080p
reset
echo "HDMI=1" > "$TMP/extcon/hdmi/state"
run on 1080p && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "on 1080p failed with $rc"
grep -q "4 9" "$TMP/dispdbg/param" || fail "dispdbg param not 4 9 for 1080p"

# 6. off
reset
run off && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "off failed with $rc"
grep -q "1 0" "$TMP/dispdbg/param" || fail "dispdbg param not 1 0"
grep -q "128" "$TMP/backlight/panel/brightness" || fail "backlight not restored"

# 7. rescue
reset
echo "hdmi_mode=auto" > "$TMP/extras.state"
run rescue "$TMP/extras.state" && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "rescue failed with $rc"
grep -q "1 0" "$TMP/dispdbg/param" || fail "rescue did not switch to lcd0"
grep -q "128" "$TMP/backlight/panel/brightness" || fail "rescue did not restore backlight"
grep -q "hdmi_mode=off" "$TMP/extras.state" || fail "rescue did not set hdmi_mode=off"

# 8. unknown command exits 2
reset
run invalid && rc=0 || rc=$?
[ "$rc" = 2 ] || fail "invalid command should exit 2, got $rc"

if [ "$FAILS" -gt 0 ]; then
	echo "$FAILS failure(s)"
	exit 1
fi
echo "slothdmi: ok"
