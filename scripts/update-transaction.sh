#!/usr/bin/env bash
# Transactional replacement helper for reviewed updater outputs.

update_transaction_script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

rename_no_replace() {
  python3 "$update_transaction_script_dir/rename-no-replace.py" "$1" "$2"
}

replace_paths_transactionally() {
  if test "$#" -lt 3 || test $((($# - 1) % 2)) -ne 0; then
    echo "transactional replacement requires a root and staged/output pairs" >&2
    return 2
  fi
  local transaction_root=$1
  shift
  if test "${transaction_root#/}" = "$transaction_root" || test -e "$transaction_root"; then
    echo "transaction root must be a new absolute path" >&2
    return 2
  fi
  mkdir -p "$transaction_root/backups"
  local transaction_device
  transaction_device=$(path_device "$transaction_root") || return 2

  local -a staged outputs had_previous installed
  local index=0
  while test "$#" -gt 0; do
    staged[index]=$1
    outputs[index]=$2
    had_previous[index]=0
    installed[index]=0
    if test "${staged[index]#/}" = "${staged[index]}" || \
      test "${outputs[index]#/}" = "${outputs[index]}" || \
      test "${outputs[index]}" = / || ! test -e "${staged[index]}" || \
      ! test -d "$(dirname "${outputs[index]}")"; then
      echo "transactional replacement path is invalid" >&2
      return 2
    fi
    if test "${MUSUBI_TEST_DEVICE_MISMATCH_INDEX-}" = "$((index + 1))" || \
      test "$(path_device "${staged[index]}")" != "$transaction_device" || \
      test "$(path_device "$(dirname "${outputs[index]}")")" != "$transaction_device"; then
      echo "transactional replacement requires one filesystem" >&2
      rm -rf "$transaction_root"
      return 2
    fi
    index=$((index + 1))
    shift 2
  done

  local count=$index
  for ((index = 0; index < count; index++)); do
    if test -e "${outputs[index]}"; then
      if ! rename_no_replace "${outputs[index]}" "$transaction_root/backups/$index"; then
        if rollback_replacements "$transaction_root" "$index"; then
          rm -rf "$transaction_root"
        fi
        return 1
      fi
      had_previous[index]=1
    fi
    if test "${MUSUBI_TEST_RACE_INSTALL_INDEX-}" = "$((index + 1))"; then
      mkdir "${outputs[index]}"
      printf 'racing markerless output\n' >"${outputs[index]}/intruder"
    fi
    if test "${MUSUBI_TEST_FAIL_INSTALL_INDEX-}" = "$((index + 1))" || \
      ! rename_no_replace "${staged[index]}" "${outputs[index]}"; then
      if rollback_replacements "$transaction_root" "$((index + 1))"; then
        rm -rf "$transaction_root"
      fi
      return 1
    fi
    installed[index]=1
  done
  rm -rf "$transaction_root"
}

path_device() {
  local path_device_value
  if path_device_value=$(stat -f '%d' "$1" 2>/dev/null); then
    printf '%s\n' "$path_device_value"
    return
  fi
  stat -c '%d' "$1" 2>/dev/null
}

rollback_replacements() {
  local transaction_root=$1
  local count=$2
  local index
  local failed=0
  for ((index = count - 1; index >= 0; index--)); do
    if test "${installed[index]:-0}" = 1; then
      if ! rename_no_replace "${outputs[index]}" "${staged[index]}"; then
        echo "rollback failed; replacement retained at ${outputs[index]}" >&2
        if test "${had_previous[index]:-0}" = 1; then
          echo "rollback failed; previous output retained at $transaction_root/backups/$index" >&2
        fi
        failed=1
        continue
      fi
    fi
    if test "${had_previous[index]:-0}" = 1 && \
      { test "${MUSUBI_TEST_FAIL_ROLLBACK_INDEX-}" = "$((index + 1))" || \
        ! rename_no_replace "$transaction_root/backups/$index" "${outputs[index]}"; }; then
      echo "rollback failed; previous output retained at $transaction_root/backups/$index" >&2
      failed=1
    fi
  done
  return "$failed"
}
