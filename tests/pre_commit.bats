#!/usr/bin/env bats
#
# Tests for scripts/pre-commit (audit 2026-11-16 F9): make lint gates the commit
# before any secret scan runs, the ggshield binary is resolved through an override,
# PATH, then fixed fallback paths, an unresolvable binary is a loud skip rather than
# a silent one, and ggshield's exit codes are read as distinct outcomes: 0 clean,
# 3 could not authenticate (warn), every other code blocks.

setup() {
  # A bats fixture must never inherit the outer repo's git environment: git
  # exports GIT_DIR (and GIT_INDEX_FILE) into a pre-push hook and `make test`
  # inherits them.
  unset GIT_DIR GIT_WORK_TREE GIT_COMMON_DIR GIT_INDEX_FILE
  export REPO_ROOT
  REPO_ROOT="$(cd "$(dirname "${BATS_TEST_DIRNAME}")" && pwd)"
  export SCRIPT="${REPO_ROOT}/scripts/pre-commit"

  # HOME is redirected so nothing the hook does reaches the real environment.
  # Every test that lets the hook resolve ggshield points it at a stub, so the
  # real ggshield installed on this machine is never invoked.
  export HOME="${BATS_TEST_TMPDIR}"
}

# A fixture needs only a Makefile with a controllable `lint` target: the hook
# calls bare `make lint`, relying on cwd being the worktree root, as git does.
write_fixture_makefile() {
  local dir="$1" lint_exit="$2" call_log="$3"

  cat > "${dir}/Makefile" <<MAKEFILE_EOF
.PHONY: lint
lint:
	@printf 'lint\n' >> '${call_log}'
	@exit ${lint_exit}
MAKEFILE_EOF
}

# A ggshield stub that records its call and arguments, then exits with the
# given code.
write_stub_ggshield() {
  local path="$1" call_log="$2" exit_code="$3"

  cat > "${path}" <<STUB_EOF
#!/usr/bin/env bash
printf 'scan\n' >> '${call_log}'
printf '%s\n' "\$*" >> '${call_log}.args'
exit ${exit_code}
STUB_EOF
  chmod +x "${path}"
}

# Tests that express "not on PATH" with PATH=/usr/bin:/bin are only sound when
# no ggshield lives there; otherwise they would reach the real binary.
require_no_system_ggshield() {
  if [[ -x /usr/bin/ggshield || -x /bin/ggshield ]]; then
    skip "a system ggshield in /usr/bin or /bin would be reached by the PATH lookup"
  fi
}

new_fixture() {
  local name="$1" dir
  dir="${BATS_TEST_TMPDIR}/${name}"
  mkdir -p "${dir}"
  printf '%s\n' "${dir}"
}

# Runs the hook in a fixture with the override pointing at a stub that exits
# with the given code. Sets `fixture` and `call_log` for the caller.
run_with_stub_exit() {
  local name="$1" rc="$2" stub
  fixture="$(new_fixture "${name}")"
  call_log="${fixture}/call.log"
  : > "${call_log}"
  write_fixture_makefile "${fixture}" 0 "${call_log}"
  stub="${fixture}/stub-ggshield"
  write_stub_ggshield "${stub}" "${call_log}" "${rc}"
  run env GGSHIELD_BIN="${stub}" bash -c "cd '${fixture}' && bash '${SCRIPT}'"
}

# --- make lint gates the scan ---

@test "make lint failing blocks the commit and never invokes ggshield" {
  local fixture call_log stub
  fixture="$(new_fixture "lint-fail")"
  call_log="${fixture}/call.log"
  : > "${call_log}"
  write_fixture_makefile "${fixture}" 1 "${call_log}"
  stub="${fixture}/stub-ggshield"
  write_stub_ggshield "${stub}" "${call_log}" 0

  run env GGSHIELD_BIN="${stub}" bash -c "cd '${fixture}' && bash '${SCRIPT}'"
  [ "$status" -ne 0 ]
  [ "$(cat "${call_log}")" = "lint" ]
}

@test "lint runs before the secret scan" {
  local fixture call_log
  run_with_stub_exit "ordering" 0
  [ "$status" -eq 0 ]
  [ "$(cat "${call_log}")" = "$(printf 'lint\nscan')" ]
}

# --- resolution: override, PATH, fallback, none ---

@test "a valid override is the binary that runs, with --fail-on-server-error pinned" {
  # A machine-level `fail_on_server_error: false` turns exit 4 into a silent 0;
  # the flag on the command line overrides it.
  local fixture call_log
  run_with_stub_exit "override" 0
  [ "$status" -eq 0 ]
  [ "$(cat "${call_log}.args")" = "secret scan pre-commit --fail-on-server-error" ]
}

@test "an override that is not executable blocks the commit" {
  local fixture bad
  fixture="$(new_fixture "bad-override")"
  write_fixture_makefile "${fixture}" 0 "${fixture}/call.log"
  bad="${fixture}/not-executable"
  printf 'not a script\n' > "${bad}"

  run env GGSHIELD_BIN="${bad}" bash -c "cd '${fixture}' && bash '${SCRIPT}'"
  [ "$status" -ne 0 ]
  [[ "$output" == *"not executable"* ]]
}

@test "ggshield found on PATH is used when there is no override" {
  local fixture call_log bindir
  fixture="$(new_fixture "path-hit")"
  call_log="${fixture}/call.log"
  : > "${call_log}"
  write_fixture_makefile "${fixture}" 0 "${call_log}"
  bindir="${fixture}/bin"
  mkdir -p "${bindir}"
  write_stub_ggshield "${bindir}/ggshield" "${call_log}" 0

  # Empty fallbacks, so a scan can only have come from the PATH lookup.
  run env -u GGSHIELD_BIN PATH="${bindir}:/usr/bin:/bin" GGSHIELD_FALLBACK_PATHS= \
    bash -c "cd '${fixture}' && bash '${SCRIPT}'"
  [ "$status" -eq 0 ]
  [ "$(cat "${call_log}")" = "$(printf 'lint\nscan')" ]
}

@test "ggshield absent from PATH is found at a fallback path" {
  require_no_system_ggshield
  # This is the audit F9 case: the binary is installed, but outside the PATH a
  # non-interactive git sees. The old hook skipped the scan here, silently.
  local fixture call_log stub
  fixture="$(new_fixture "fallback-hit")"
  call_log="${fixture}/call.log"
  : > "${call_log}"
  write_fixture_makefile "${fixture}" 0 "${call_log}"
  stub="${fixture}/elsewhere/ggshield"
  mkdir -p "${fixture}/elsewhere"
  write_stub_ggshield "${stub}" "${call_log}" 0

  run env -u GGSHIELD_BIN PATH=/usr/bin:/bin \
    GGSHIELD_FALLBACK_PATHS="/nonexistent/ggshield ${stub}" \
    bash -c "cd '${fixture}' && bash '${SCRIPT}'"
  [ "$status" -eq 0 ]
  [ "$(cat "${call_log}")" = "$(printf 'lint\nscan')" ]
}

@test "ggshield absent from every resolution path warns loudly and does not block" {
  require_no_system_ggshield
  local fixture
  fixture="$(new_fixture "absent")"
  write_fixture_makefile "${fixture}" 0 "${fixture}/call.log"

  # PATH alone cannot express absence once the absolute fallback list exists:
  # its entries are real installed paths on the dev machines.
  run env -u GGSHIELD_BIN PATH=/usr/bin:/bin GGSHIELD_FALLBACK_PATHS=/nonexistent/ggshield \
    bash -c "cd '${fixture}' && bash '${SCRIPT}'"
  [ "$status" -eq 0 ]
  [[ "$output" == *"not found on PATH or in any known install location"* ]]
  [[ "$output" == *"SECRET SCAN DID NOT RUN"* ]]
}

@test "an explicitly empty fallback list means empty, not the default list" {
  require_no_system_ggshield
  # ${VAR:-default} would treat empty as unset, restore the built-in list and
  # invoke the real ggshield; a test expressing absence that way would then
  # pass for the wrong reason.
  local fixture
  fixture="$(new_fixture "empty-fallbacks")"
  write_fixture_makefile "${fixture}" 0 "${fixture}/call.log"

  run env -u GGSHIELD_BIN PATH=/usr/bin:/bin GGSHIELD_FALLBACK_PATHS= \
    bash -c "cd '${fixture}' && bash '${SCRIPT}'"
  [ "$status" -eq 0 ]
  [[ "$output" == *"not found on PATH or in any known install location"* ]]
  [[ "$output" != *"could not authenticate"* ]]
}

# --- ggshield exit codes (ggshield 1.56.1: 0 clean, 1 secret, 2 usage or 401,
# --- 3 could not authenticate, 4 server unavailable, 128 unexpected) ---

@test "a clean scan exits zero with no warning" {
  local fixture call_log
  run_with_stub_exit "clean" 0
  [ "$status" -eq 0 ]
  [[ "$output" != *"SECRET SCAN DID NOT RUN"* ]]
}

@test "a secret found (exit 1) blocks the commit" {
  local fixture call_log
  run_with_stub_exit "secret-found" 1
  [ "$status" -ne 0 ]
  # The block must come from the scan itself, not from an earlier failure.
  [ "$(cat "${call_log}")" = "$(printf 'lint\nscan')" ]
}

@test "could not authenticate (exit 3) warns but does not block" {
  local fixture call_log
  run_with_stub_exit "no-auth" 3
  [ "$status" -eq 0 ]
  [[ "$output" == *"could not authenticate"* ]]
  [[ "$output" == *"ggshield auth login"* ]]
  [[ "$output" == *"SECRET SCAN DID NOT RUN"* ]]
}

@test "server unavailable (exit 4) blocks the commit" {
  # A timed-out chunk of a multi-chunk scan can drop the findings of chunks
  # that finished, so a 4 can carry a secret.
  local fixture call_log
  run_with_stub_exit "unreachable" 4
  [ "$status" -ne 0 ]
  [[ "$output" == *"GitGuardian unreachable"* ]]
}

@test "a usage error (exit 2) blocks the commit" {
  local fixture call_log
  run_with_stub_exit "usage" 2
  [ "$status" -ne 0 ]
  [[ "$output" == *"(exit 2)"* ]]
  [[ "$output" == *"upgrade it"* ]]
}

@test "unknown exit codes (128 and 99) block the commit" {
  local fixture call_log rc
  for rc in 128 99; do
    run_with_stub_exit "unknown-${rc}" "${rc}"
    [ "$status" -ne 0 ]
    [[ "$output" == *"(exit ${rc})"* ]]
    [[ "$output" == *"unexpected exit code"* ]]
  done
}
