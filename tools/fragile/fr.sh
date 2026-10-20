#!/usr/bin/env bash
# Run a command at the root of the Fragile tree with the Rust toolchain on PATH,
# eg tools/fragile/fr.sh task test
source "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"
cd "$(dirname "$0")/../.." || exit 1
exec "$@"
