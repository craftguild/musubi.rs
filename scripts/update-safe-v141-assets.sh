#!/usr/bin/env bash
# Regenerates reviewed Safe 1.4.1 registry and integration artifacts from pinned upstream inputs.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd -P)
. "$repo_root/scripts/sdk-cli.sh"
cd "$repo_root"
source "$repo_root/scripts/update-transaction.sh"
source_revision=bf943f80fec5ac647159d26161446ac5d716a294
source_safe_sha256=efd3301ed5c6bf522fb3a7a43be4a27341e65dff6d1fee04f8567f304a67e365
deployment_revision=5bb0ebd7150a777f39bec4733e4d799c4b637b49
package_url=https://registry.npmjs.org/@safe-global/safe-contracts/-/safe-contracts-1.4.1.tgz
package_sha512=7cfd637b0cb04b0b089e2334e0db2a3f255114a3cc02e8ab0b77ed03f4c0e17dd9739127c10a7f5022545363cbff7effcff8cca3c514689fa99c535698c53b75
temporary=$(mktemp -d "$repo_root/.safe-v141-update.XXXXXX")
trap 'rm -rf "$temporary"' EXIT
staged="$temporary/staged"
staged_assets="$staged/assets"
staged_fixtures="$staged/fixtures"
sdk_input="$staged/safe-v141-sdk-input"
staged_release_fixture="$staged/release-fixture"
mkdir -p "$staged_assets" "$staged_fixtures" "$sdk_input"
test -f "$repo_root/safe-v141-sdk.toml"
test -f "$repo_root/safe-v141-sdk-input/safe-v1.4.1.typed-data-schema.json"
cp "$repo_root/safe-v141-sdk-input/safe-v1.4.1.typed-data-schema.json" \
  "$sdk_input/safe-v1.4.1.typed-data-schema.json"
cp -R "$repo_root/release-fixture" "$staged_release_fixture"

curl --fail --location --silent --show-error "$package_url" >"$temporary/safe-contracts.tgz"
actual_package_sha512=$(shasum -a 512 "$temporary/safe-contracts.tgz" | awk '{print $1}')
test "$actual_package_sha512" = "$package_sha512"

asset_base="https://raw.githubusercontent.com/safe-global/safe-deployments/$deployment_revision/src/assets/v1.4.1"
for asset in safe safe_proxy_factory compatibility_fallback_handler; do
  curl --fail --location --silent --show-error "$asset_base/$asset.json" >"$temporary/$asset.json"
done

jq --sort-keys --indent 2 --null-input \
  --arg source_revision "$source_revision" \
  --arg source_safe_sha256 "$source_safe_sha256" \
  --arg deployment_revision "$deployment_revision" \
  --arg safe_sha "$(shasum -a 256 "$temporary/safe.json" | awk '{print $1}')" \
  --arg factory_sha "$(shasum -a 256 "$temporary/safe_proxy_factory.json" | awk '{print $1}')" \
  --arg handler_sha "$(shasum -a 256 "$temporary/compatibility_fallback_handler.json" | awk '{print $1}')" \
  --slurpfile safe "$temporary/safe.json" \
  --slurpfile factory "$temporary/safe_proxy_factory.json" \
  --slurpfile handler "$temporary/compatibility_fallback_handler.json" '
  def contract($input; $sha): {
    contract_name: $input.contractName,
    address: $input.deployments.canonical.address,
    runtime_code_hash: $input.deployments.canonical.codeHash,
    upstream_asset_sha256: $sha
  };
  {
    schema_version: 2,
    safe_version: "1.4.1",
    smart_account_revision: $source_revision,
    smart_account_safe_source_sha256: $source_safe_sha256,
    deployment_revision: $deployment_revision,
    contracts: {
      singleton: contract($safe[0]; $safe_sha),
      proxy_factory: contract($factory[0]; $factory_sha),
      compatibility_fallback_handler: contract($handler[0]; $handler_sha)
    }
  }' >"$staged_assets/safe-v1.4.1-registry.json"

tar -xzf "$temporary/safe-contracts.tgz" -C "$temporary" \
  package/build/artifacts/contracts/Safe.sol/Safe.json \
  package/build/artifacts/contracts/Safe.sol/Safe.dbg.json \
  package/build/artifacts/contracts/proxies/SafeProxy.sol/SafeProxy.json \
  package/build/artifacts/build-info/0128da95e283bfa75197e80e5a451911.json \
  package/contracts/Safe.sol
jq --sort-keys --indent 2 --null-input \
  --arg package_url "$package_url" \
  --arg package_sha512 "$package_sha512" \
  --slurpfile safe "$temporary/package/build/artifacts/contracts/Safe.sol/Safe.json" \
  --slurpfile proxy "$temporary/package/build/artifacts/contracts/proxies/SafeProxy.sol/SafeProxy.json" '
  {
    npm_package_url: $package_url,
    npm_package_sha512: $package_sha512,
    safe_creation_bytecode: $safe[0].bytecode,
    safe_runtime_bytecode: $safe[0].deployedBytecode,
    proxy_creation_bytecode: $proxy[0].bytecode,
    proxy_runtime_bytecode: $proxy[0].deployedBytecode
  }' >"$staged_fixtures/safe-v1.4.1-bytecode.json"

safe_artifact="$temporary/package/build/artifacts/contracts/Safe.sol/Safe.json"
proxy_artifact="$temporary/package/build/artifacts/contracts/proxies/SafeProxy.sol/SafeProxy.json"
safe_build_info="$temporary/package/build/artifacts/build-info/0128da95e283bfa75197e80e5a451911.json"
safe_source="$temporary/package/contracts/Safe.sol"
source_url="https://raw.githubusercontent.com/safe-global/safe-smart-account/$source_revision/contracts/Safe.sol"
pinned_safe_source="$temporary/pinned-Safe.sol"
curl --fail --location --silent --show-error "$source_url" >"$pinned_safe_source"
"$repo_root/scripts/verify-safe-source-provenance.sh" \
  "$source_revision" "$source_safe_sha256" "$safe_source" "$pinned_safe_source" "$source_url"
jq --sort-keys --indent 2 '.abi' "$safe_artifact" \
  >"$sdk_input/safe-v1.4.1.abi.json"
jq --sort-keys --indent 2 '.bytecode' "$safe_artifact" \
  >"$sdk_input/safe-v1.4.1.creation.json"
jq --sort-keys --indent 2 '.deployedBytecode' "$safe_artifact" \
  >"$sdk_input/safe-v1.4.1.runtime.json"
jq --sort-keys --indent 2 \
  '.output.contracts["contracts/Safe.sol"].Safe.metadata | fromjson' "$safe_build_info" \
  >"$sdk_input/safe-v1.4.1.metadata.json"
jq --sort-keys --indent 2 '.abi' "$proxy_artifact" \
  >"$sdk_input/safe-proxy-v1.4.1.abi.json"
jq --sort-keys --indent 2 '.bytecode' "$proxy_artifact" \
  >"$sdk_input/safe-proxy-v1.4.1.creation.json"
jq --sort-keys --indent 2 '.deployedBytecode' "$proxy_artifact" \
  >"$sdk_input/safe-proxy-v1.4.1.runtime.json"
jq --sort-keys --indent 2 \
  '.output.contracts["contracts/proxies/SafeProxy.sol"].SafeProxy.metadata | fromjson' \
  "$safe_build_info" \
  >"$sdk_input/safe-proxy-v1.4.1.metadata.json"
safe_source_sha256=$(shasum -a 256 "$safe_source" | awk '{print $1}')
safe_artifact_sha256=$(shasum -a 256 "$safe_artifact" | awk '{print $1}')
compiler_version=$(jq -r '.solcLongVersion' "$safe_build_info")
compiler_settings_sha256=$(jq --sort-keys --compact-output '.input.settings' \
  "$safe_build_info" | shasum -a 256 | awk '{print $1}')
jq --sort-keys --indent 2 --null-input \
  --arg source_sha "0x$safe_source_sha256" \
  --arg artifact_sha "0x$safe_artifact_sha256" \
  --arg compiler_version "$compiler_version" \
  --arg settings_sha "0x$compiler_settings_sha256" '
  {
    schema_version: "musubi/typed-data-vectors-v1",
    evidence: {
      solidity_source_path: "safe-contracts-1.4.1/contracts/Safe.sol",
      solidity_source_sha256: $source_sha,
      artifact_target: "contracts/Safe.sol:Safe",
      artifact_sha256: $artifact_sha,
      compiler_version: $compiler_version,
      compiler_settings_sha256: $settings_sha,
      vector_command: "cargo test -p musubi-safe --test safe_v141_anvil -- --ignored"
    },
    vectors: [{
      root: "SafeTx",
      domain_type: "EIP712Domain",
      value: {
        to: "0x70997970c51812dc3a010c7d01b50e0d17dc79c8",
        value: "0",
        data: "0x",
        operation: "0",
        safeTxGas: "0",
        baseGas: "0",
        gasPrice: "0",
        gasToken: "0x0000000000000000000000000000000000000000",
        refundReceiver: "0x0000000000000000000000000000000000000000",
        nonce: "0"
      },
      domain: {
        chainId: "31337",
        verifyingContract: "0x948b3c65b89df0b4894abe91e6d02fe579834f8f"
      },
      type_string: "SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)",
      type_hash: "0xbb8310d486368db6bd6f849402fdd73ad53d316b5a4b2644ad6efe0f941286d8",
      struct_hash: "0x29ac27abd58c548be83dccf4f9946756500b2872f560eeda27f78558bc93a820",
      domain_type_string: "EIP712Domain(uint256 chainId,address verifyingContract)",
      domain_type_hash: "0x47e79534a245952e8b16893a336b85a3d9ea9fa8c573f3d803afb92a79469218",
      domain_separator: "0x6dd4323dbab478cc965d23390bbe1a8ef88b28876856da3b547567bcd0ef5a4b",
      digest: "0x54abba6b7ac0068752dd2da32357cf2505e548b1599508e55193f6d49101e723"
    }]
  }' >"$sdk_input/safe-v1.4.1.typed-data-vectors.json"

# The release smoke fixture publishes only the reviewed Safe evidence above. Keeping
# these copies mechanical prevents an unrelated hand-authored vector from becoming
# release provenance.
cp "$sdk_input/safe-v1.4.1.typed-data-schema.json" \
  "$staged_release_fixture/typed-data-schema.json"
cp "$sdk_input/safe-v1.4.1.typed-data-vectors.json" \
  "$staged_release_fixture/typed-data-vectors.json"

validation="$temporary/validation"
mkdir "$validation"
cp "$repo_root/Cargo.toml" "$repo_root/Cargo.lock" "$validation/"
cp -R "$repo_root/crates" "$validation/crates"
rm -rf "$validation/crates/musubi-safe-v141-generated"
cp -R "$sdk_input" "$validation/safe-v141-sdk-input"
cp "$repo_root/safe-v141-sdk.toml" "$validation/safe-v141-sdk.toml"
cp "$staged_assets/safe-v1.4.1-registry.json" \
  "$validation/crates/musubi-safe/assets/safe-v1.4.1-registry.json"
cp "$staged_fixtures/safe-v1.4.1-bytecode.json" \
  "$validation/crates/musubi-safe/tests/fixtures/safe-v1.4.1-bytecode.json"

sdk_cli --manifest "$validation/safe-v141-sdk.toml" generate >/dev/null
sdk_cli --manifest "$validation/safe-v141-sdk.toml" check >/dev/null
(cd "$validation" && cargo test --locked -p musubi-safe --test safe_v141_anvil -- --ignored)

"$repo_root/scripts/python.sh" "$repo_root/scripts/check-owned-sdk.py" \
  "$validation/crates/musubi-safe-v141-generated"
if test -e "$repo_root/crates/musubi-safe-v141-generated"; then
  "$repo_root/scripts/python.sh" "$repo_root/scripts/check-owned-sdk.py" \
    "$repo_root/crates/musubi-safe-v141-generated"
fi
"$repo_root/scripts/python.sh" "$repo_root/scripts/refresh-artifact-group.py" \
  "$repo_root" safe-v141-sdk-input "$sdk_input" "$temporary/artifact-inventory.json"
install_root=$(mktemp -d "$repo_root/.safe-v141-install.XXXXXX")
rmdir "$install_root"
replace_paths_transactionally "$install_root" \
  "$temporary/artifact-inventory.json" "$repo_root/scripts/artifact-inventory.json" \
  "$staged_assets" "$repo_root/crates/musubi-safe/assets" \
  "$staged_fixtures" "$repo_root/crates/musubi-safe/tests/fixtures" \
  "$sdk_input" "$repo_root/safe-v141-sdk-input" \
  "$staged_release_fixture" "$repo_root/release-fixture" \
  "$validation/crates/musubi-safe-v141-generated" \
  "$repo_root/crates/musubi-safe-v141-generated"
