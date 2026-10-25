#!/bin/sh
SD="${SLOT_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
RUN="${AGS_RUN:-/run}"
LOGS="$SD/Logs"
SESSION_MAX=65536
export SLOT_ROOT="$SD"
export SLOT_RUN="$RUN"

mkdir -p "$LOGS" 2>/dev/null
for f in slot.log slot.log.1 slot-session.log slot-session.log.1 slot-card.log ags-shutdown-trace.log input-trace.log; do
	[ -f "$SD/$f" ] && mv -f "$SD/$f" "$LOGS/$f"
done
[ -d "$SD/crash" ] && [ ! -e "$LOGS/crash" ] && mv "$SD/crash" "$LOGS/crash"

[ $(wc -c < "$LOGS/slot-session.log" 2>/dev/null || echo 0) -gt "$SESSION_MAX" ] \
	&& mv -f "$LOGS/slot-session.log" "$LOGS/slot-session.log.1"

log() {
	echo "$(date '+%F %T') $*" >> "$LOGS/slot-session.log" 2>/dev/null
}

# --- OTA Update Handover ---
if [ -f "$SYS/slot.new" ]; then
	log "Applying OTA update: staging slot.new -> slot"
	chmod +x "$SYS/slot.new" 2>/dev/null
	cp -f "$SYS/slot" "$SYS/slot.bak" 2>/dev/null
	mv -f "$SYS/slot.new" "$SYS/slot"
	for newcore in "$SYS"/*.so.new; do
		[ -f "$newcore" ] || continue
		base="$(basename "$newcore" .new)"
		mv -f "$newcore" "$SYS/$base"
	done
	sync
fi

# --- Rollback Safeguard ---
CRASH_COUNT_FILE="$RUN/slot-crash-count"
if [ -f "$SYS/slot.bak" ]; then
	CRASHES=$(cat "$CRASH_COUNT_FILE" 2>/dev/null || echo 0)
	if [ "$CRASHES" -ge 3 ]; then
		log "Rollback safeguard: restoring slot.bak due to repeated crashes ($CRASHES)"
		mv -f "$SYS/slot.bak" "$SYS/slot"
		rm -f "$CRASH_COUNT_FILE"
		sync
	fi
fi

pid="$(cat "$RUN/slot-led.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/led.sh" once
	sh "$SYS/led.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-led.pid"
fi

pid="$(cat "$RUN/slot-lid.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/lid.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-lid.pid"
fi

echo "|/bin/sh $SYS/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null

[ -f "$LOGS/slot.log" ] && mv -f "$LOGS/slot.log" "$LOGS/slot.log.1"
log "exec $SYS/slot"

CRASHES=$(( $(cat "$CRASH_COUNT_FILE" 2>/dev/null || echo 0) + 1 ))
echo "$CRASHES" > "$CRASH_COUNT_FILE" 2>/dev/null
( sleep 15 && rm -f "$CRASH_COUNT_FILE" ) > /dev/null 2>&1 &

exec /lib/ld-linux-aarch64.so.1 "$SYS/slot" > "$LOGS/slot.log" 2>&1
