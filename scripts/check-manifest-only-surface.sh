#!/bin/sh
# Static source inventory plus the actual public help surface for the clean cutover.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
. "$repo_root/scripts/sdk-cli.sh"
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM

# Only these small negative fixtures name removed schema identifiers or override flags.
if rg -n \
  'musubi/config-v[123]|musubi/release-v[123]|ManifestV[12]|ContractConfigV[12]|ReleaseContractV[12]|CONFIG_V[123]|legacy_v1|PackagedRuntime|RuntimeIdentity|validate_ssh_release_identity|MUSUBI_RELEASE_|repository_sdk|allow_unknown_length|optional_length_equal|--development-runtime-path|--bundled-runtime' \
  crates scripts external-acceptance release-fixture safe-v141-sdk.toml README.md .github \
  -g '!args_tests.rs' -g '!lifecycle.rs' -g '!manifest_tests.rs' \
  -g '!check_generated_tests.rs' -g '!runtime_source.rs' \
  -g '!snapshot_schema_tests.rs' -g '!generated_provenance_tests.rs' \
  -g '!check-manifest-only-surface.sh' -g '!**/out/**' -g '!**/target/**'; then
  echo "removed control-plane surface remains outside rejection fixtures" >&2
  exit 1
else
  test "$?" -eq 1
fi

for command in init generate update check doctor; do
  sdk_cli "$command" --help \
    >"$temporary/help"
  if rg -- '--(out|output|abi|bytecode|creation-bytecode|runtime-bytecode|ast|metadata|storage-layout|contract-id|force|override|runtime-git|development-runtime|bundled-runtime)' "$temporary/help"; then
    echo "public lifecycle help exposes a configuration override" >&2
    exit 1
  fi
done
sdk_cli --help >"$temporary/help"
if rg 'validate-release-identity' "$temporary/help"; then
  echo "removed publisher command appears in ordinary help" >&2
  exit 1
fi
