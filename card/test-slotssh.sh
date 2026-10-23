#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/System/slotssh.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
FAILS=0

fail() {
	echo "FAIL: $*"
	FAILS=$((FAILS + 1))
}

make_fake() {
	name="$1"
	out="$2"
	shift 2
	{
		echo '#!/bin/sh'
		echo "printf '%s\\n' \"\$*\" >> \"$out\""
		for line in "$@"; do echo "$line"; done
		echo 'exit 0'
	} > "$TMP/bin/$name"
	chmod 755 "$TMP/bin/$name"
}

reset() {
	rm -rf "$TMP/run" "$TMP/data" "$TMP/bin" "$TMP"/*.log "$TMP/shadow"
	mkdir -p "$TMP/run" "$TMP/data" "$TMP/bin"

	# Mock dropbear: spawns background sleep and writes its PID as instructed by -P
	cat > "$TMP/bin/dropbear" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >> "$AGS_RUN/dropbear.log"
while [ $# -gt 0 ]; do
	case "$1" in
	-P)
		sleep 60 &
		echo "$!" > "$2"
		shift 2
		;;
	*)
		shift
		;;
	esac
done
exit 0
EOF
	chmod 755 "$TMP/bin/dropbear"

	make_fake dropbearkey "$TMP/run/dropbearkey.log"
	make_fake pidof "$TMP/run/pidof.log" "exit 1"
	make_fake killall "$TMP/run/killall.log"

	cat > "$TMP/bin/mkpasswd" <<'EOF'
#!/bin/sh
echo '$6$fakehash$'
EOF
	chmod 755 "$TMP/bin/mkpasswd"

	cat > "$TMP/shadow" <<'EOF'
root:$6$oldhash$:19000:0:99999:7:::
daemon:*:19000:0:99999:7:::
EOF
}

run() {
	PATH="$TMP/bin:$PATH" \
		AGS_DROPBEAR="$TMP/bin/dropbear" \
		AGS_DROPBEARKEY="$TMP/bin/dropbearkey" \
		AGS_PIDOF="$TMP/bin/pidof" \
		AGS_KILLALL="$TMP/bin/killall" \
		AGS_RUN="$TMP/run" \
		AGS_DATA="$TMP/data" \
		AGS_SHADOW="$TMP/shadow" \
		AGS_MKPASSWD="$TMP/bin/mkpasswd" \
		sh "$SCRIPT" "$@"
}

reset
run on && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "on exited $rc"
[ -f "$TMP/run/dropbear.pid" ] || fail "dropbear did not write pidfile"
grep -q -- '-t rsa' "$TMP/run/dropbearkey.log" || fail "rsa key was not generated"
grep -q -- '-t ed25519' "$TMP/run/dropbearkey.log" || fail "ed25519 key was not generated"

out="$(run status)"
[ "$out" = "running" ] || fail "status expected running, got $out"

run off && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "off exited $rc"
[ ! -f "$TMP/run/dropbear.pid" ] || fail "off left pidfile behind"

out="$(run status)" && sc=0 || sc=$?
[ "$out" = "stopped" ] || fail "status expected stopped, got $out"
[ "$sc" = 1 ] || fail "status on stopped expected rc 1, got $sc"

# Test setting password
run password "hunter2" && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "password exited $rc"
grep -q '^root:\$6\$fakehash\$:' "$TMP/shadow" || fail "shadow was not updated with new password"
grep -q '^daemon:\*:' "$TMP/shadow" || fail "other accounts in shadow were corrupted"

if [ "$FAILS" -gt 0 ]; then
	echo "$FAILS failure(s)"
	exit 1
fi
echo "slotssh: ok"
