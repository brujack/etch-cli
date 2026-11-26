#!/usr/bin/env bats

# Proves scripts/pre-push loads the shared push-content gate (ai-config ADR-0091) and
# runs the shared push-time gates (ADR-0099). No test reaches the network: a `gh` stub
# that exits 1 is first on PATH for every hook run, so the wiring's `gh auth token`
# fails before any curl is built. End-to-end runs use a COPY of the hook whose
# `_pcg_fetch` is a fake: it serves the ai-config master commit JSON and a recorder in
# place of the gates driver, falls through to the real fetch only for the
# push-content gate library, and logs every path it is asked for.

readonly PCG_WIRING_SHA256="554c1385d10d77fac177df1a7aefdf6238e12ba009218689b7c5242fa24d4102"
readonly PG_WIRING_SHA256="e38c2b5fa6a5de641a421a1e1e71fe10061ab6d6ccf4c7aa4749de9156794689"
readonly NULL_SHA="0000000000000000000000000000000000000000"
readonly FAKE_MASTER_SHA="0123456789abcdef0123456789abcdef01234567"

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
  export GATES_LOG="${FIXTURE}/gates.log" ORDER_LOG="${FIXTURE}/order.log" FETCH_LOG="${FIXTURE}/fetch.log"
  export FAKE_MASTER_SHA FAKE_DRIVER="${FIXTURE}/fake_driver.sh" FAKE_FETCH="${FIXTURE}/fake_fetch.sh"
  unset FAKE_MASTER_FAIL GATES_RC
  mkdir -p "${BIN}" "${REPO}"
  export HOME="${BATS_TEST_TMPDIR}/home" XDG_CONFIG_HOME="${BATS_TEST_TMPDIR}/home/.config"
  mkdir -p "${HOME}" "${XDG_CONFIG_HOME}"

  # gh stub: records its arguments, always fails. make stub: records, succeeds.
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> "%s"\nexit 1\n' "${GH_CALLS}" > "${BIN}/gh"
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> "%s"\nprintf "make\\n" >> "%s"\nexit 0\n' "${MAKE_CALLS}" "${ORDER_LOG}" > "${BIN}/make"
  chmod +x "${BIN}/gh" "${BIN}/make"

  HOOK_COPY="$(_hook_copy)" || return 1

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

# _hook_copy -- writes an executable copy of the hook whose `_pcg_fetch` is a fake and
# prints its path. The original body is kept as `_pcg_fetch_real`, reachable only for the
# push-content gate library. Fails loudly when the markers are not where expected.
_hook_copy() {
  local start end copy="${FIXTURE}/pre-push.copy"
  start="$(command grep -n '^# >>> _pcg_fetch$' "${HOOK}" | cut -d: -f1)"
  end="$(command grep -n '^# <<< _pcg_fetch$' "${HOOK}" | cut -d: -f1)"
  if [[ ! "${start}" =~ ^[0-9]+$ || ! "${end}" =~ ^[0-9]+$ ]] || (( end <= start + 1 )) \
    || [[ "$(sed -n "$((start + 1))p" "${HOOK}")" != '_pcg_fetch() {'* ]]; then
    printf '_hook_copy: _pcg_fetch markers not found in %s\n' "${HOOK}" >&2
    return 1
  fi
  cat > "${FAKE_DRIVER}" <<'DRV' || return 1
push_gates_run() {
  printf 'url=%s\nsha=%s\nrefs=%s\n' "${1}" "${3}" "${2}" >> "${GATES_LOG}"
  printf 'gates\n' >> "${ORDER_LOG}"
  return "${GATES_RC:-0}"
}
DRV
  cat > "${FAKE_FETCH}" <<'FAKE' || return 1
_pcg_fetch() {
  printf '%s\n' "${1}" >> "${FETCH_LOG}"
  case "${1}" in
    repos/brujack/ai-config/commits/master)
      [[ -z "${FAKE_MASTER_FAIL:-}" ]] || return 1
      printf '{"sha":"%s"}\n' "${FAKE_MASTER_SHA}"
      ;;
    "repos/brujack/ai-config/contents/.claude/hooks/push-gates.sh?ref=${FAKE_MASTER_SHA}")
      printf '%s\n' "$(< "${FAKE_DRIVER}")"
      ;;
    repos/brujack/ai-config/contents/.claude/hooks/push-content-gate.sh?ref=master) _pcg_fetch_real "${1}" ;;
    *) return 1 ;;
  esac
}
FAKE
  awk -v s="${start}" -v e="${end}" -v fake="${FAKE_FETCH}" '
    NR > s && NR < e { sub(/^_pcg_fetch\(\)/, "_pcg_fetch_real()") }
    { print }
    NR == e { while ((getline l < fake) > 0) print l }
  ' "${HOOK}" > "${copy}" || return 1
  printf '%s' "${copy}"
}

# _order -- the recorded call order, comma-joined (empty when nothing was logged).
_order() {
  [ -f "${ORDER_LOG}" ] || return 0
  paste -sd, "${ORDER_LOG}"
}

# _run_hook <stdin text> [cwd] -- runs the hook with the stubs first on PATH, in
# the fixture repo unless a cwd is given.
_run_hook() {
  printf '%b' "${1}" | bash -c "
    export PATH='${BIN}:/usr/bin:/bin'
    cd '${2:-${REPO}}' || exit 2
    bash -p '${HOOK_COPY}' origin file:///nonexistent
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
  [ "$(_order)" = "gates" ]
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
  [ "$(_order)" = "gates" ]
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
  [ "$(_order)" = "gates" ]
}

@test "a main push whose gate cannot load is refused before make" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/main ${head} refs/heads/main ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"cannot load the push-content gate"* ]]
  grep -q 'auth token' "${GH_CALLS}"
  [ ! -f "${MAKE_CALLS}" ]
  # Positive check first: the fake fetch was on the path and saw the gate library.
  if ! command grep -qF 'push-content-gate.sh?ref=master' "${FETCH_LOG}"; then
    printf 'fake _pcg_fetch was not on the path (FETCH_LOG lacks the gate library)\n' >&2
    return 1
  fi
  if command grep -qF 'push-gates.sh' "${FETCH_LOG}"; then
    printf 'a refused main push must never fetch the gates driver\n' >&2
    return 1
  fi
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

@test "the push-gates block matches the sha256 pinned to ai-config docs/templates/pre-push-gates" {
  local got
  got="$(sed -n '/^# >>> push-gates wiring$/,/^# <<< push-gates wiring$/p' "${HOOK}" | sha256sum | cut -d' ' -f1)"
  if [ "${got}" != "${PG_WIRING_SHA256}" ]; then
    printf 'push-gates block missing or drifted (got %s); re-sync it byte for byte from ai-config docs/templates/pre-push-gates\n' "${got}" >&2
    return 1
  fi
}

@test "gates: a feature push reaches the recorder with url, refs and master sha before make test" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 0 ]
  [ "$(_order)" = "gates,make" ]
  grep -qxF -- "url=file:///nonexistent" "${GATES_LOG}"
  grep -qxF -- "sha=${FAKE_MASTER_SHA}" "${GATES_LOG}"
  grep -qxF -- "refs=refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}" "${GATES_LOG}"
}

@test "gates: a recorder returning 1 blocks the push and make is not run" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  GATES_RC=1 run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [ "$(_order)" = "gates" ]
  [ ! -f "${MAKE_CALLS}" ]
}

@test "gates: a master-JSON fetch failure blocks with the cannot-load line and make is not run" {
  local head
  head="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"pre-push: gates: cannot load push gates from GitHub; push blocked"* ]]
  [ ! -f "${MAKE_CALLS}" ]
  [ ! -f "${GATES_LOG}" ]
}

@test "gates: a copy of the hook refuses to build when the _pcg_fetch markers are missing" {
  local saved="${HOOK}"
  printf '#!/usr/bin/env bash\n:\n' > "${FIXTURE}/nomarkers"
  HOOK="${FIXTURE}/nomarkers"
  run _hook_copy
  HOOK="${saved}"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"markers not found"* ]]
}

# R17 exemption (ai-config ADR-0099 2A). These tests build the one fixture the exemption
# accepts: an `origin` remote whose single URL equals the hook's URL argument, an
# origin/HEAD, and tracking refs equal to the remote sha being replaced. The master-JSON
# fetch is made to fail, so any run that reaches GitHub blocks with the cannot-load line.
_with_origin() {
  git -C "${REPO}" remote add origin file:///nonexistent || return 1
  git -C "${REPO}" update-ref refs/remotes/origin/main "${BASE_SHA}" || return 1
  git -C "${REPO}" symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main || return 1
  git -C "${REPO}" update-ref refs/remotes/origin/feat/x "${BASE_SHA}" || return 1
  git -C "${REPO}" update-ref refs/remotes/origin/feat/y "${BASE_SHA}" || return 1
}

@test "exempt: a docs-only update of an existing branch passes without any GitHub fetch" {
  local head
  _with_origin
  head="$(_commit_file docs/guide.md v2 'docs: guide')"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 0 ]
  [[ "${output}" == *"pre-push: gates: every pushed ref is a deletion or a docs-only branch; gates not run"* ]]
  [ ! -f "${FETCH_LOG}" ]
  [ ! -f "${GATES_LOG}" ]
  [ ! -f "${MAKE_CALLS}" ]
}

@test "exempt: a deletion-only push passes offline" {
  _with_origin
  FAKE_MASTER_FAIL=1 run _run_hook "(delete) ${NULL_SHA} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 0 ]
  [[ "${output}" == *"gates not run"* ]]
  [ ! -f "${FETCH_LOG}" ]
  [ ! -f "${MAKE_CALLS}" ]
}

@test "exempt: a push mixing a docs-only ref with a code ref loads the gates and blocks offline" {
  local docs code
  _with_origin
  docs="$(_commit_file README.md v2 'docs: v2')"
  code="$(_commit_file x.rs 'fn main() {}' 'feat: code')"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${docs} refs/heads/feat/x ${BASE_SHA}\nrefs/heads/feat/y ${code} refs/heads/feat/y ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"pre-push: gates: cannot load push gates from GitHub; push blocked"* ]]
  [[ "${output}" != *"gates not run"* ]]
  command grep -qxF 'repos/brujack/ai-config/commits/master' "${FETCH_LOG}"
}

@test "exempt: a push mixing a docs-only ref with a new branch loads the gates and blocks offline" {
  local docs
  _with_origin
  docs="$(_commit_file README.md v2 'docs: v2')"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${docs} refs/heads/feat/x ${BASE_SHA}\nrefs/heads/feat/new ${docs} refs/heads/feat/new ${NULL_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"cannot load push gates from GitHub; push blocked"* ]]
  [[ "${output}" != *"gates not run"* ]]
}

@test "exempt: a .sh file under docs/ is not docs-only and blocks offline" {
  local head
  _with_origin
  head="$(_commit_file docs/run.sh 'echo hi' 'docs: script')"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" == *"cannot load push gates from GitHub; push blocked"* ]]
  [[ "${output}" != *"gates not run"* ]]
}

@test "exempt: a .md file that starts with a shebang is not docs-only and blocks offline" {
  local head
  _with_origin
  head="$(_commit_file docs/sneaky.md '#!/bin/sh' 'docs: sneaky')"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" != *"gates not run"* ]]
}

@test "exempt: a docs-only push whose tracking ref differs from the remote sha is not exempt" {
  local head
  _with_origin
  head="$(_commit_file README.md v2 'docs: v2')"
  git -C "${REPO}" update-ref refs/remotes/origin/feat/x "${head}"
  FAKE_MASTER_FAIL=1 run _run_hook "refs/heads/feat/x ${head} refs/heads/feat/x ${BASE_SHA}\n"
  [ "${status}" -eq 1 ]
  [[ "${output}" != *"gates not run"* ]]
}
