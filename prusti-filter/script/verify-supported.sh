#!/bin/bash

set -eo pipefail

info() { echo -e "[-] ${*}"; }
error() { echo -e "[!] ${*}"; }

cargoclean() {
	# Clean the artifacts of this project ("bin" or "lib"), but not those of the dependencies
	names="$(cargo metadata --format-version 1 | jq -r '.packages[].targets[] | select( .kind | map(. == "bin" or . == "lib") | any ) | select ( .src_path | contains(".cargo/registry") | . != true ) | .name')"
	for name in $names; do
		cargo clean -p "$name" || cargo clean
	done
}

# Get the directory in which this script is contained
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null && pwd )"

# Get the root directory of the crate, which is the first argument or the current folder
CRATE_ROOT="$(cd "${1:-.}" && pwd)"
cd "$CRATE_ROOT"

if [[ ! -r "$CRATE_ROOT/Cargo.toml" ]]; then
	error "Path '$CRATE_ROOT' does not look like the source of a crate"
	exit 1
fi


EVALUATION_TIMEOUT="${EVALUATION_TIMEOUT:-900}"
info "Using EVALUATION_TIMEOUT=$EVALUATION_TIMEOUT seconds"

FORCE_PRUSTI_FILTER="${FORCE_PRUSTI_FILTER:-false}"
info "Using FORCE_PRUSTI_FILTER=$FORCE_PRUSTI_FILTER"

info "Run standard compilation"

# Make sure that the "standard" compilation uses the same compiler flags as Prusti uses
export RUSTFLAGS="-Zborrowck=mir -Zpolonius -Znll-facts"
export POLONIUS_ALGORITHM="Naive"
exit_status="0"
cargo metadata --format-version 1 > /dev/null || exit_status="$?"
if [[ "$exit_status" != "0" ]]; then
	info "The crate does not compile (cargo metadata). Skip verification."
	exit 42
fi
cargo clean || exit_status="$?"
if [[ "$exit_status" != "0" ]]; then
	info "The crate does not compile (cargo clean). Skip verification."
	exit 42
fi
# Timeout in seconds
timeout -k 10 $EVALUATION_TIMEOUT cargo build || exit_status="$?"
if [[ "$exit_status" != "0" ]]; then
	info "The crate does not compile (cargo build). Skip verification."
	exit 42
fi

info "Filter supported procedures"

if [[ ! -r "$CRATE_ROOT/prusti-filter-results.json" ]] || [[ "$FORCE_PRUSTI_FILTER" == "true" ]] ; then
	rm -f "$CRATE_ROOT/prusti-filter-results.json"
	export RUSTC="$DIR/rustc.sh"
	export RUST_BACKTRACE=1
	exit_status="0"
	cargoclean
	# Timeout in seconds
	timeout -k 10 $EVALUATION_TIMEOUT cargo build -j 1 || exit_status="$?" && true
	unset RUSTC
	unset RUST_BACKTRACE
	if [[ "$exit_status" != "0" ]]; then
		info "The automatic filtering of verifiable functions failed."
		exit 43
	fi
fi

supported_procedures="$(jq '.functions[] | select(.procedure.restrictions | length == 0) | .node_path' "$CRATE_ROOT/prusti-filter-results.json")"

info "Prepare whitelist ($(echo "$supported_procedures" | grep . | wc -l) items)"

(
	echo "CHECK_PANICS = false"
	echo "ENABLE_WHITELIST = true"
	echo "WHITELIST = ["
	echo "$supported_procedures" | sed 's/$/,/' | sed '$ s/.$//'
	echo "]"
) > "$CRATE_ROOT/Prusti.toml"

# Sometimes a dependecy is compiled somewhere else. So, make sure that the whitelist is always enabled.
export PRUSTI_ENABLE_WHITELIST=true
export PRUSTI_CHECK_PANICS=false

info "Start verification"

# Save disk space
rm -rf log/ nll-facts/
# This is important! Without this, NLL facts are not recomputed and dumped to nll-facts.
rm -rf target/*/incremental/
export PRUSTI_FULL_COMPILATION=true
export RUSTC="$DIR/../../docker/prusti"
export RUST_BACKTRACE=1
cargoclean
# Timeout in seconds
timeout -k 10 $EVALUATION_TIMEOUT cargo build -j 1
