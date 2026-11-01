#!/usr/bin/env bats

# Proves scripts/pre-push loads the shared push-content gate (ai-config ADR-0091).
# No test reaches the network: a `gh` stub that exits 1 is first on PATH for every
# hook run, so the wiring's `gh auth token` fails before any curl is built.

readonly PCG_WIRING_SHA256="d556979bd64b72ca237e4bd3a8d4cf709e0db164ae155c18ab0eea86e39e221f"
readonly NULL_SHA="0000000000000000000000000000000000000000"

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/.." && pwd)"
  HOOK="${REPO_ROOT}/scripts/pre-push"
  unset GIT_DIR GIT_WORK_TREE GIT_COMMON_DIR GIT_INDEX_FILE
  export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
  export GIT_AUTHOR_NAME=Test GIT_AUTHOR_EMAIL=test@test.invalid
  export GIT_COMMITTER_NAME=Test GIT_COMMITTER_EMAIL=test@test.invalid

  FIXTURE="$(mktemp -d "${BATS_TEST_TMPDIR}/fixture.XXXXXX")" || return 1
  BIN="${FIXTURE}/bin"
  REPO="${FIXTURE}/repo"
  GH_CALLS="${FIXTURE}/gh.calls"
  MAKE_CALLS="${FIXTURE}/make.calls"
  mkdir -p "${BIN}" "${REPO}"
  export HOME="${BATS_TEST_TMPDIR}/home" XDG_CONFIG_HOME="${BATS_TEST_TMPDIR}/home/.config"
  mkdir -p "${HOME}" "${XDG_CONFIG_HOME}"

  # gh stub: records its arguments, always fails. make stub: records, succeeds.
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> "%s"\nexit 1\n' "${GH_CALLS}" > "${BIN}/gh"
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> "%s"\nexit 0\n' "${MAKE_CALLS}" > "${BIN}/make"
  chmod +x "${BIN}/gh" "${BIN}/make"

  git -C "${REPO}" init --quiet || return 1
  BASE_SHA="$(_commit_file README.md v1 'docs: v1')"
}

# _commit_file <path> <content> <message> -- commits one file, prints the new HEAD.
_commit_file() {
  mkdir -p "$(dirname "${REPO}/${1}")" || return 1
  printf '%s\n' "${2}" > "${REPO}/${1}" || return 1
  git -C "${REPO}" add -- "${1}" || return 1
  git -C "${REPO}" commit --quiet -m "${3}" || return 1
  git -C "${REPO}" rev-parse HEAD
}

# _run_hook <stdin text> [cwd] -- runs the hook with the stubs first on PATH, in
# the fixture repo unless a cwd is given.
_run_hook() {
  printf '%b' "${1}" | bash -c "
    export PATH='${BIN}:/usr/bin:/bin'
    cd '${2:-${REPO}}' || exit 2
    bash '${HOOK}' origin file:///nonexistent
  "
}

@test "a feature push with an .rs change runs make test and makes no gh call" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 0 ]
  [ ! -f "${GH_CALLS}" ]
  grep -qxF -- "-C ${REPO} test" "${MAKE_CALLS}"
}

# Table-driven: one feature push per trigger alternative. sub/Cargo.toml pins the
# ^ anchor on Cargo; dropping any single alternative turns this red.
@test "each trigger path runs make test and a nested Cargo.toml or a doc does not" {
  local f base head
  for f in x.rs Cargo.toml Cargo.lock tests/x.py scripts/x.py; do
    rm -f "${MAKE_CALLS}"
    base="$(git -C "${REPO}" rev-parse HEAD)"
    head="$(_commit_file "${f}" "v-${f}" "feat: ${f}")"
    run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${base}\n"
    [ "${status}" -eq 0 ]
    grep -qxF -- "-C ${REPO} test" "${MAKE_CALLS}"
  done
  rm -f "${MAKE_CALLS}"
  base="$(git -C "${REPO}" rev-parse HEAD)"
  head="$(_commit_file sub/Cargo.toml 'v' 'feat: nested')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${base}\n"
  [ "${status}" -eq 0 ]
  [ ! -f "${MAKE_CALLS}" ]
  base="${head}"
  head="$(_commit_file docs/x.md 'v' 'docs: x')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${base}\n"
  [ "${status}" -eq 0 ]
  [ ! -f "${MAKE_CALLS}" ]
}

@test "a new-branch push (all-zero remote sha) with an .rs change runs make test" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${NULL_SHA}\n"
  [ "${status}" -eq 0 ]
  grep -qxF -- "-C ${REPO} test" "${MAKE_CALLS}"
  # An unresolvable all-zero range would also run make (fail closed), so pin that
  # the all-zero branch itself was taken and no range failed to resolve.
  [[ "${output}" != *"cannot resolve"* ]]
}

@test "a deletion push (all-zero local sha) exits 0 without make or a git fatal" {
  run _run_hook "(delete) ${NULL_SHA} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 0 ]
  [ ! -f "${MAKE_CALLS}" ]
  [[ "${output}" != *fatal* ]]
}

# The hook pipes nothing: an unresolvable range (a remote sha this checkout lacks)
# must fail closed by running the suite, not read as "nothing changed".
@test "an unresolvable range fails closed by running make test" {
  local head
  head="$(_commit_file README.md v2 'docs: v2')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n"
  [ "${status}" -eq 0 ]
  [[ "${output}" == *"cannot resolve"* ]]
  grep -qxF -- "-C ${REPO} test" "${MAKE_CALLS}"
}

# From inside .git there is no work tree, so `rev-parse --show-toplevel` fails and
# the hook must derive the root from the common git dir.
@test "run from inside .git the hook still finds the repo root" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n" "${REPO}/.git"
  [ "${status}" -eq 0 ]
  grep -qxF -- "-C ${REPO} test" "${MAKE_CALLS}"
}

# A new branch is judged against its merge-base with origin/main, not all history:
# origin/main already holds an .rs change, so a docs-only commit on top must skip.
@test "a docs-only new branch on top of origin/main skips make test" {
  local code head
  code="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  git -C "${REPO}" update-ref refs/remotes/origin/main "${code}"
  head="$(_commit_file README.md v2 'docs: v2')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${NULL_SHA}\n"
  [ "${status}" -eq 0 ]
  [ ! -f "${MAKE_CALLS}" ]
}

# PATH holds make, env and git only (symlinks), so neither shellcheck nor bats
# resolves and bats cannot recurse into itself.
_hooks_target_without_tools() { # <CI value, or empty to unset>
  local shim="${FIXTURE}/notools" tool root
  root="$(cd "${BATS_TEST_DIRNAME}/.." && pwd)" || return 1
  mkdir -p "${shim}" || return 1
  for tool in make env git; do
    ln -sf "$(command -v "${tool}")" "${shim}/${tool}" || return 1
  done
  if [ -n "${1}" ]; then
    CI="${1}" PATH="${shim}" "${shim}/make" -C "${root}" test-hooks 2>&1
  else
    env -u CI PATH="${shim}" "${shim}/make" -C "${root}" test-hooks 2>&1
  fi
}

@test "make test-hooks without shellcheck and bats fails in CI and names both" {
  run _hooks_target_without_tools 1
  [ "${status}" -ne 0 ]
  [[ "${output}" == *"shellcheck"* ]]
  [[ "${output}" == *"bats"* ]]
}

@test "make test-hooks without shellcheck and bats skips locally with exit 0" {
  run _hooks_target_without_tools ""
  [ "${status}" -eq 0 ]
  [[ "${output}" == *"skipping"* ]]
}

@test "a docs-only feature push skips make test" {
  local head
  head="$(_commit_file README.md v2 'docs: v2')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 0 ]
  [ ! -f "${MAKE_CALLS}" ]
}

@test "a main push whose gate cannot load is refused before make" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/main ${head} refs/heads/main ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"cannot load the push-content gate"* ]]
  grep -q 'auth token' "${GH_CALLS}"
  [ ! -f "${MAKE_CALLS}" ]
}

@test "main on a later ref line is still judged" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${NULL_SHA}\nrefs/heads/main ${head} refs/heads/main ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"cannot load the push-content gate"* ]]
  grep -q 'auth token' "${GH_CALLS}"
  [ ! -f "${MAKE_CALLS}" ]
}

# The wiring decides main/master in bash and reads stdin with a builtin, so
# neither a PATH awk that says "no match" nor a PATH cat that prints nothing can
# make a main push look like a feature push.
@test "a PATH awk or cat cannot hide a main push" {
  local head stub
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  for stub in 'awk:exit 1' 'cat:exit 0'; do
    printf '#!/usr/bin/env bash\n%s\n' "${stub#*:}" > "${BIN}/${stub%%:*}"
    chmod +x "${BIN}/${stub%%:*}"
    rm -f "${GH_CALLS}"
    run _run_hook "refs/heads/main ${head} refs/heads/main ${BASE_SHA}\n"
    rm -f "${BIN}/${stub%%:*}"
    [ "${status}" -eq 1 ]
    [[ "${output}" == *"cannot load the push-content gate"* ]]
    grep -q 'auth token' "${GH_CALLS}"
  done
  [ ! -f "${MAKE_CALLS}" ]
}

# git always hands pre-push a pipe, but with fd 0 closed the read fails and the
# hook must refuse rather than read "no refs".
@test "an unreadable stdin refuses" {
  # A `$(cat)` read blocks on a closed fd 0 instead of failing, so a regression
  # to it would hang the suite. The watchdog turns that into a failure after 30s.
  # `set -m` gives the hook its own process group so the kill also reaches the
  # blocked `cat`, which otherwise keeps bats' output pipe open.
  run bash -c "
    set -m
    export PATH='${BIN}:/usr/bin:/bin'
    cd '${REPO}' || exit 2
    bash -p '${HOOK}' <&- &
    hook=\$!
    ( sleep 30; kill -- -\"\${hook}\" 2>/dev/null ) &
    dog=\$!
    wait \"\${hook}\"; rc=\$?
    kill -- -\"\${dog}\" 2>/dev/null
    exit \"\${rc}\"
  "
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"cannot read the pushed refs"* ]]
  [ ! -f "${GH_CALLS}" ]
  [ ! -f "${MAKE_CALLS}" ]
}

@test "the hook starts with bash -p" {
  [ "$(head -n 1 "${HOOK}")" = '#!/usr/bin/env -S bash -p' ]
}

@test "the wiring block matches the pinned template hash" {
  local got
  got="$(sed -n '/^# >>> push-content-gate wiring$/,/^# <<< push-content-gate wiring$/p' "${HOOK}" | sha256sum | cut -d' ' -f1)"
  [ "${got}" = "${PCG_WIRING_SHA256}" ]
}
