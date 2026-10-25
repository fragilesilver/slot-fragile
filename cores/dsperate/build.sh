#!/bin/sh
# Fragile: builds the DSperate Nintendo DS libretro core
set -eu

device_cflags="-mcpu=cortex-a53 -mtune=cortex-a53"
cxx_ldflags="-static-libstdc++ -static-libgcc"

if [ -n "${CORE_HOST_CFLAGS:-}" ]; then
	device_cflags="$CORE_HOST_CFLAGS"
	cxx_ldflags=""
fi

usage() {
	echo "usage: $0 stamp COMMIT | build COMMIT WORKDIR OUT" >&2
	exit 2
}

stamp() {
	echo "commit=$1"
	echo "source=https://github.com/beebono/DSperate"
	echo "device_cflags=$device_cflags"
}

build() {
	commit="$1"
	work="$2"
	out="$3"

	# Locate DSperate source tree (sibling project or vendored)
	src="/home/fragkicul/Work/MustardOS/projects/DSperate"
	if [ ! -d "$src" ]; then
		echo "DSperate source directory not found at $src" >&2
		exit 1
	fi

	mkdir -p "$work"
	mkdir -p "$(dirname "$out")"

	if ! command -v cmake >/dev/null 2>&1; then
		if command -v apt-get >/dev/null 2>&1; then
			sed -i '/bullseye-security/d' /etc/apt/sources.list 2>/dev/null || true
			apt-get -o Acquire::Check-Valid-Until=false update -qq
			apt-get install -y -qq --no-install-recommends cmake ninja-build >/dev/null
		fi
	fi

	generator="Unix Makefiles"
	if command -v ninja >/dev/null 2>&1; then
		generator="Ninja"
	fi

	cmake -S "$src" -B "$work" -G "$generator" \
		-DCMAKE_BUILD_TYPE=Release \
		-DDSPERATE_TESTS=OFF \
		-DDSPERATE_SDL=OFF \
		-DDSPERATE_WAYLAND=OFF \
		-DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null

	cmake --build "$work" --target dsperate_libretro --parallel "$(getconf _NPROCESSORS_ONLN)" >/dev/null

	artifact="$work/src/frontend/libretro/dsperate_libretro.so"
	if [ ! -f "$artifact" ]; then
		echo "build finished without producing $artifact" >&2
		exit 1
	fi

	cp "$artifact" "$out"
	stamp "$commit" >"$out.meta"
	echo "built $out"
}

case "${1:-}" in
stamp)
	[ $# -eq 2 ] && [ -n "$2" ] || usage
	stamp "$2"
	;;
build)
	[ $# -eq 4 ] && [ -n "$2" ] && [ -n "$3" ] && [ -n "$4" ] || usage
	build "$2" "$3" "$4"
	;;
*)
	usage
	;;
esac
