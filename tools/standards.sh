#!/bin/sh
# Select the matching verified entry; build execution belongs to the shared engine.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"
if [ -f .craftguild/gpg-failure ]; then
    echo 'OPENPGP_FAILURE: pause the goal before further work' >&2
    exit 71
fi
export RUSTUP_AUTO_INSTALL=0
export CARGO_NET_OFFLINE=true
case "${1:-}" in
    check|test)
        : "${PROFILE:=debug}"
        export PROFILE
        ;;
    assess)
        sh scripts/python.sh scripts/standards-ownership.py
        shift
        set -- assess --root "$root" "$@"
        ;;
esac
exec sh scripts/python.sh standards/launcher.py "$@"
