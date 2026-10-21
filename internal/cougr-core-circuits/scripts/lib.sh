#!/usr/bin/env bash
# Shared PATH for Sunvest Circom scripts (Bun-managed node_modules).

Sunvest_circuits_root() {
  local lib_dir
  lib_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  cd "$lib_dir/.." && pwd
}

Sunvest_circuits_export_path() {
  local root
  root="$(Sunvest_circuits_root)"
  local repo_root
  repo_root="$(cd "$root/../.." && pwd)"
  export PATH="$repo_root/target/circom_bin/bin:$root/node_modules/.bin:$PATH"
}