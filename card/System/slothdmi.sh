#!/bin/sh
# Fragile: HDMI display and audio control for the Anbernic H700 on BaseOS.
# Manages Sunxi disp2 output routing through debugfs (/sys/kernel/debug/dispdbg)
# and checks cable hotplug state via extcon (/sys/class/extcon/hdmi/state).
set -eu

EXTCON="${AGS_EXTCON_HDMI_STATE:-/sys/class/extcon/hdmi/state}"
EXTCON_DIR="$(dirname "$EXTCON")"
DISPDBG="${AGS_DISPDBG_DIR:-/sys/kernel/debug/dispdbg}"
BACKLIGHT_DIR="${AGS_BACKLIGHT_DIR:-/sys/class/backlight}"

dispdbg_write() {
	name="$1"
	cmd="$2"
	param="$3"
	if [ -d "$DISPDBG" ]; then
		[ -w "$DISPDBG/name" ] && printf '%s\n' "$name" > "$DISPDBG/name" 2>/dev/null || true
		[ -w "$DISPDBG/command" ] && printf '%s\n' "$cmd" > "$DISPDBG/command" 2>/dev/null || true
		[ -w "$DISPDBG/param" ] && printf '%s\n' "$param" > "$DISPDBG/param" 2>/dev/null || true
		[ -w "$DISPDBG/start" ] && printf '1\n' > "$DISPDBG/start" 2>/dev/null || true
	fi
}

set_backlight() {
	val="$1"
	if [ -d "$BACKLIGHT_DIR" ]; then
		for b in "$BACKLIGHT_DIR"/*; do
			if [ -w "$b/brightness" ]; then
				if [ "$val" = "max" ] && [ -r "$b/max_brightness" ]; then
					cat "$b/max_brightness" > "$b/brightness" 2>/dev/null || true
				else
					printf '%s\n' "$val" > "$b/brightness" 2>/dev/null || true
				fi
			fi
		done
	fi
}

is_connected() {
	if [ -r "$EXTCON" ]; then
		case "$(cat "$EXTCON" 2>/dev/null || echo 0)" in
			*1*|*[Cc]onnected*) return 0 ;;
			*) return 1 ;;
		esac
	fi
	return 1
}

case "${1:-}" in
probe)
	# Check whether HDMI hardware exists on this device
	if [ -d "$EXTCON_DIR" ] || [ -r "$EXTCON" ]; then
		echo "present"
		exit 0
	else
		echo "absent"
		exit 1
	fi
	;;
status)
	if is_connected; then
		echo "connected"
		exit 0
	else
		echo "disconnected"
		exit 1
	fi
	;;
on)
	mode="${2:-720p}"
	case "$mode" in
		1080p|1080)
			# Sunxi disp2 parameter for 1080p60 HDMI
			dispdbg_write "disp0" "switch1" "4 9 0 0 0x4 0x101 0 0 0 8"
			;;
		*)
			# Sunxi disp2 parameter for 720p60 HDMI (safe default)
			dispdbg_write "disp0" "switch1" "4 10 0 0 0x4 0x101 0 0 0 8"
			;;
	esac
	# Dim or turn off internal panel backlight when outputting to TV
	set_backlight 0
	echo "hdmi: on ($mode)"
	;;
off)
	# Switch disp0 back to LCD
	dispdbg_write "disp0" "switch1" "1 0"
	# Restore internal panel backlight
	set_backlight 128
	echo "hdmi: off"
	;;
rescue)
	# Emergency rescue: force internal LCD and restore backlight
	dispdbg_write "disp0" "switch1" "1 0"
	set_backlight 128
	extras_file="${2:-}"
	if [ -n "$extras_file" ] && [ -f "$extras_file" ]; then
		sed -i '/^hdmi_mode=/d' "$extras_file" 2>/dev/null || true
		echo "hdmi_mode=off" >> "$extras_file"
	fi
	echo "hdmi: rescued"
	;;
*)
	echo "usage: slothdmi.sh probe | status | on [720p|1080p] | off | rescue [extras.state]" >&2
	exit 2
	;;
esac
