#!/usr/bin/env bash
# Validates that docs/src/ doesn't reference platforms or providers that are
# no longer supported. Run in CI and locally via: bash scripts/check-docs.sh
#
# Add new terms to the denylist when platform/provider support is dropped.
# Remove terms when support is re-added.

set -e

DOCS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/docs/src"
FAILED=0

deny() {
    local term="$1"
    local reason="$2"
    if grep -rq "${term}" "${DOCS_DIR}"; then
        printf "FAIL: docs/src/ mentions '%s' (%s)\n" "${term}" "${reason}"
        grep -rn "${term}" "${DOCS_DIR}"
        FAILED=1
    fi
}

# Unsupported platforms
deny "Windows"    "not a supported platform"
deny "FreeBSD"    "not a supported platform"
deny "NetBSD"     "not a supported platform"
deny "OpenSUSE"   "not a supported platform"
deny "Void Linux" "not a supported platform"
deny "Fedora"     "not a supported platform"

# Unimplemented package providers
deny "winget"     "not an implemented package provider"
deny "xbps"       "not an implemented package provider"
deny "pkgin"      "not an implemented package provider"
deny "zypper"     "not an implemented package provider"
deny "macports"   "not an implemented package provider"

if [[ "${FAILED}" -eq 0 ]]; then
    printf "docs-lint OK\n"
fi

exit "${FAILED}"
