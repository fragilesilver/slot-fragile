#!/bin/sh
# Fragile: builds the cores of the systems past the Game Boy family (Phase 12) from their pinned
# source archives, each with its own makefile's recipe. The archive ships beside the core as its
# corresponding source.
set -eu

device_cflags="-mcpu=cortex-a53 -mtune=cortex-a53"
# The C++ cores carry their own C++ runtime rather than lean on the device's.
cxx_ldflags="-static-libstdc++ -static-libgcc"
# A build for this computer, for the tests, names its own flags.
if [ -n "${CORE_HOST_CFLAGS:-}" ]; then
	device_cflags="$CORE_HOST_CFLAGS" cxx_ldflags=""
fi

usage() {
	echo "usage: $0 stamp NAME COMMIT | build NAME COMMIT TARBALL WORKDIR OUT" >&2
	echo "NAME is one of fceumm snes9x supafaust pcsx_rearmed" >&2
	exit 2
}

# NAME -> the repository under github.com/libretro, the folder in its archive, how to make it,
# the library it makes, and whether it is C++.
recipe() {
	case "$1" in
	fceumm)
		repo=libretro-fceumm make_args="-f Makefile.libretro platform=unix"
		artifact=fceumm_libretro.so cxx=0
		;;
	snes9x)
		repo=snes9x make_args="-C libretro platform=unix"
		artifact=libretro/snes9x_libretro.so cxx=1
		;;
	supafaust)
		repo=supafaust make_args="platform=unix"
		artifact=mednafen_supafaust_libretro.so cxx=1
		;;
	pcsx_rearmed)
		repo=pcsx_rearmed make_args="-f Makefile.libretro platform=unix"
		artifact=pcsx_rearmed_libretro.so cxx=0
		;;
	*)
		usage
		;;
	esac
}

stamp() {
	recipe "$1"
	echo "commit=$2"
	echo "source=https://github.com/libretro/$repo/archive/$2.tar.gz"
	echo "recipe=make $make_args"
	echo "device_cflags=$device_cflags"
	if [ "$cxx" = 1 ]; then echo "ldflags=$cxx_ldflags"; fi
}

build() {
	name="$1" commit="$2" tarball="$3" work="$4" out="$5"
	recipe "$name"
	src="$work/$repo-$commit"

	rm -rf "$work"
	mkdir -p "$work"
	tar -xzf "$tarball" -C "$work"
	if [ ! -d "$src" ]; then
		echo "$tarball does not hold $repo-$commit" >&2
		exit 1
	fi
	# The work folder sits inside Fragile's own checkout: keep the cores' makefiles from
	# reading Fragile's commit as their version.
	GIT_CEILING_DIRECTORIES="$(cd "$work" && pwd)"
	export GIT_CEILING_DIRECTORIES

	ldflags=""
	if [ "$cxx" = 1 ]; then ldflags="$cxx_ldflags"; fi
	# shellcheck disable=SC2086
	CFLAGS="$device_cflags" CXXFLAGS="$device_cflags" LDFLAGS="$ldflags" \
		make -C "$src" $make_args -j"$(getconf _NPROCESSORS_ONLN)"

	if [ ! -f "$src/$artifact" ]; then
		echo "make finished without producing $artifact" >&2
		exit 1
	fi
	mkdir -p "$(dirname "$out")"
	cp "$src/$artifact" "$out"
	stamp "$name" "$commit" >"$out.meta"
}

case "${1:-}" in
stamp)
	[ $# -eq 3 ] && [ -n "$2" ] && [ -n "$3" ] || usage
	stamp "$2" "$3"
	;;
build)
	[ $# -eq 6 ] && [ -n "$2" ] && [ -n "$3" ] && [ -n "$4" ] && [ -n "$5" ] && [ -n "$6" ] || usage
	build "$2" "$3" "$4" "$5" "$6"
	;;
*)
	usage
	;;
esac
