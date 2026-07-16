# Release SBOM Vulnerability Monitor Implementation Plan

> **Status: DONE** — merged via PR #121 (2026-07-16). Post-merge verification: manual workflow_dispatch on main succeeded, 0 issues opened. Repo Issues had to be enabled (previously disabled) for the notify path to work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Monthly scheduled workflow that re-scans the latest published release's SBOM
against current CVE feeds and opens a deduplicated GitHub issue for any Critical/High
finding, closing the gap between one-time SBOM generation/signing and drift over the
binary's shipped lifetime.

**Architecture:** Reusable workflow (`release-sbom-monitor.yml`, `workflow_call` +
`workflow_dispatch`) does the actual scan for one binary. A thin scheduling caller
(`release-sbom-monitor-schedule.yml`, `schedule` + `workflow_dispatch`) invokes it monthly
with `binary_name: etch`, `release_tag_pattern: "v"`. Uses `anchore/scan-action@v7` (`sbom:`
input, `output-format: json`, `fail-build: false`) to run grype against the downloaded SBOM
asset — same vendor family as the existing `anchore/sbom-action` used in `release-sign.yml`.

**Tech Stack:** GitHub Actions, `gh` CLI (release/issue operations), `jq` (JSON filtering),
`anchore/scan-action@v7` (grype-backed SBOM scan).

## Global Constraints

- Full design: `ai-config` `docs/superpowers/specs/2026-07-16-release-sbom-vuln-monitoring-design.md`
- Cross-cutting decision record: `dotfiles` ADR-0015
- Only the **latest** release matching `release_tag_pattern` is scanned — not history.
- Severity gate for issue creation: **Critical + High only**.
- Missing SBOM asset on the latest release → skip gracefully (`exit 0`), not a failure —
  covers releases that predate the SBOM+cosign rollout.
- Dedup: before creating an issue, search open issues labeled `sbom-monitor` with the CVE
  ID and binary name in the title; skip if one already exists.
- This is a non-blocking, informational workflow — no PR gate, no `fail-build`.

---

## Verification Planning

Per `behavior.md`, session-level verification (above the per-task acceptance gates below):

- **Command:** after this branch is pushed and the PR is open, manually trigger the
  scheduled workflow against the branch:
    ```bash
    gh workflow run release-sbom-monitor-schedule.yml --ref <branch-name>
    sleep 5
    RUN_ID=$(gh run list --workflow=release-sbom-monitor-schedule.yml --branch <branch-name> \
      --limit 1 --json databaseId --jq '.[0].databaseId')
    gh run watch "${RUN_ID}" --exit-status
    ```
- **Expected output:** run concludes `success`. Since etch-cli's current dependency set has
  no known Critical/High CVEs (per existing `cargo-audit-scheduled.yml`), expect **zero**
  issues created — confirms the pipeline runs end-to-end (tag resolution, SBOM download,
  scan, dedup query) without error.
- **Edge case to exercise:** the "no SBOM asset" skip path can't be exercised against a real
  release (etch-cli's only release already has an SBOM asset per the existing
  `release-sign.yml`). This path is covered by code inspection at review time, not a live
  run — the `if gh release download ... 2>/dev/null; then ... else ...` branch is
  straightforward enough that a runtime test isn't essential (matches spec's stated
  testing approach: no permanent test suite for this GH Actions logic, one-time manual
  verification at implementation).
- This verification is NOT one of the per-task acceptance gates below — it requires the
  branch to exist on `origin` (workflow_dispatch --ref needs the workflow file on that ref
  remotely), so it happens after push, as part of the normal PR/CI-monitoring flow, not
  mid-implementation.

---

## Task 1: `sbom-monitor` label + reusable `release-sbom-monitor.yml` workflow

```yaml-task
id: 1
description: Create sbom-monitor GitHub label and reusable release-sbom-monitor.yml workflow (resolve latest release, download SBOM, scan via anchore/scan-action, dedup + file issue on Critical/High)
role: executor
model: sonnet
tdd: not-applicable
acceptance:
  - cmd: 'python3 -c "import yaml, sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/release-sbom-monitor.yml'
    exit_code: 0
  - cmd: 'yamllint -d relaxed .github/workflows/release-sbom-monitor.yml'
    exit_code: 0
  - cmd: 'gh label list --json name --jq ".[].name" | grep -qx sbom-monitor'
    exit_code: 0
max_retries: 3
files_touched:
  - .github/workflows/release-sbom-monitor.yml
depends_on: []
```

**Files:**

1. Create the label (one-time repo setup, run directly — not part of the YAML file):

    ```bash
    gh label create sbom-monitor --color B60205 \
      --description "Opened by the monthly release SBOM vulnerability monitor" \
      || gh label list --json name --jq '.[].name' | grep -qx sbom-monitor
    ```

    (The `||` fallback makes this idempotent — if the label already exists, `gh label create`
    exits non-zero and the fallback just confirms it's present rather than failing the task.)

2. Create `.github/workflows/release-sbom-monitor.yml`:

    ```yaml
    name: Release SBOM Monitor

    on:
      workflow_call:
        inputs:
          binary_name:
            required: true
            type: string
          release_tag_pattern:
            required: true
            type: string
      workflow_dispatch:
        inputs:
          binary_name:
            required: true
            type: string
            description: "Binary name (e.g. etch)"
          release_tag_pattern:
            required: true
            type: string
            description: "Tag prefix to match latest release (e.g. v)"

    env:
      FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"

    jobs:
      monitor:
        name: SBOM Vulnerability Scan
        runs-on: ubuntu-latest
        permissions:
          contents: read
          issues: write
        steps:
          - uses: actions/checkout@v6

          - name: Resolve latest matching release tag
            id: latest
            env:
              GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
              TAG_PATTERN: ${{ inputs.release_tag_pattern }}
            run: |
              LATEST_TAG=$(gh release list --limit 50 --json tagName \
                --jq ".[] | select(.tagName | startswith(\"${TAG_PATTERN}\")) | .tagName" \
                | head -1)
              if [[ -z "${LATEST_TAG}" ]]; then
                echo "No matching release found for pattern ${TAG_PATTERN} — skipping"
                echo "found=false" >> "$GITHUB_OUTPUT"
                exit 0
              fi
              echo "found=true" >> "$GITHUB_OUTPUT"
              echo "tag=${LATEST_TAG}" >> "$GITHUB_OUTPUT"

          - name: Download SBOM asset
            if: steps.latest.outputs.found == 'true'
            id: sbom
            env:
              GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
              LATEST_TAG: ${{ steps.latest.outputs.tag }}
              BINARY_NAME: ${{ inputs.binary_name }}
            run: |
              if gh release download "${LATEST_TAG}" \
                  --pattern "${BINARY_NAME}.sbom.spdx.json" 2>/dev/null; then
                echo "present=true" >> "$GITHUB_OUTPUT"
              else
                echo "No SBOM asset on ${LATEST_TAG} — pre-dates SBOM rollout, skipping"
                echo "present=false" >> "$GITHUB_OUTPUT"
              fi

          - name: Scan SBOM
            if: steps.latest.outputs.found == 'true' && steps.sbom.outputs.present == 'true'
            id: scan
            uses: anchore/scan-action@v7
            with:
              sbom: "${{ inputs.binary_name }}.sbom.spdx.json"
              output-format: json
              fail-build: false

          - name: Filter Critical/High and file issues
            if: steps.latest.outputs.found == 'true' && steps.sbom.outputs.present == 'true'
            env:
              GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
              BINARY_NAME: ${{ inputs.binary_name }}
              LATEST_TAG: ${{ steps.latest.outputs.tag }}
              SCAN_JSON: ${{ steps.scan.outputs.json }}
            run: |
              jq -c '.matches[] | select(.vulnerability.severity == "Critical" or .vulnerability.severity == "High")' \
                "${SCAN_JSON}" | while read -r finding; do
                CVE_ID=$(echo "${finding}" | jq -r '.vulnerability.id')
                SEVERITY=$(echo "${finding}" | jq -r '.vulnerability.severity')
                PACKAGE_NAME=$(echo "${finding}" | jq -r '.artifact.name')
                PACKAGE_VERSION=$(echo "${finding}" | jq -r '.artifact.version')
                FIX_VERSION=$(echo "${finding}" | jq -r '.vulnerability.fix.versions[0] // "none"')

                EXISTING=$(gh issue list --label sbom-monitor --state open \
                  --search "in:title \"${CVE_ID}\" \"${BINARY_NAME}\"" \
                  --json number --jq '.[0].number // empty')

                if [[ -n "${EXISTING}" ]]; then
                  echo "Issue already open for ${CVE_ID} in ${BINARY_NAME}: #${EXISTING}"
                  continue
                fi

                gh issue create \
                  --label sbom-monitor \
                  --title "[SBOM Monitor] ${CVE_ID} in ${BINARY_NAME} ${LATEST_TAG}" \
                  --body "Severity: ${SEVERITY}
    Package: ${PACKAGE_NAME} ${PACKAGE_VERSION}
    Fix available: ${FIX_VERSION}
    Release: ${LATEST_TAG}
    Source: monthly release-sbom-monitor.yml scan

    Non-blocking. Human judgment call on whether this warrants a patch release
    (reachability, exploitability, whether ${LATEST_TAG} is still actively downloaded)."
              done
    ```

**Interfaces:**

- Consumes: nothing from earlier tasks (first task in this plan).
- Produces: reusable workflow `release-sbom-monitor.yml` callable via
  `uses: ./.github/workflows/release-sbom-monitor.yml` with inputs `binary_name` (string)
  and `release_tag_pattern` (string). Task 2 calls this.

---

## Task 2: Scheduled caller `release-sbom-monitor-schedule.yml`

```yaml-task
id: 2
description: Create monthly scheduled workflow that calls release-sbom-monitor.yml with binary_name=etch, release_tag_pattern=v
role: executor
model: sonnet
tdd: not-applicable
acceptance:
  - cmd: 'python3 -c "import yaml, sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/release-sbom-monitor-schedule.yml'
    exit_code: 0
  - cmd: 'yamllint -d relaxed .github/workflows/release-sbom-monitor-schedule.yml'
    exit_code: 0
  - cmd: 'grep -q "uses: ./.github/workflows/release-sbom-monitor.yml" .github/workflows/release-sbom-monitor-schedule.yml'
    exit_code: 0
max_retries: 3
files_touched:
  - .github/workflows/release-sbom-monitor-schedule.yml
depends_on: [1]
```

**Files:**

Create `.github/workflows/release-sbom-monitor-schedule.yml`:

```yaml
name: Release SBOM Monitor (scheduled)

on:
    schedule:
        - cron: "0 13 3 * *"
    workflow_dispatch:

jobs:
    monitor:
        uses: ./.github/workflows/release-sbom-monitor.yml
        with:
            binary_name: etch
            release_tag_pattern: "v"
        permissions:
            contents: read
            issues: write
```

**Interfaces:**

- Consumes: Task 1's `release-sbom-monitor.yml` reusable workflow (`binary_name`,
  `release_tag_pattern` inputs).
- Produces: nothing consumed by further tasks — this is the last task in this plan.

---

## Self-Review

1. **Spec coverage:** design spec's etch-cli scope (single binary, reusable+caller split,
   grype scan, dedup, Critical/High gate, graceful SBOM-missing skip) — all covered across
   Task 1 (scan logic) and Task 2 (schedule). ✓
2. **Placeholder scan:** none — both workflow files are complete, no TBD.
3. **Type consistency:** `binary_name`/`release_tag_pattern` input names match between
   Task 1's `workflow_call` inputs and Task 2's `with:` block. ✓
4. **YAML block:** both tasks have `yaml-task` fences; run `make validate-plan` (ai-config)
   before dispatch.
5. **TDD `files_touched`:** N/A — both tasks are `tdd: not-applicable` (CI YAML, no test
   harness for GH Actions logic in this repo).
6. **Token-budget check:** both task blocks are workflow-YAML-heavy by necessity (the YAML
   _is_ the deliverable) — no redundant BDD boilerplate added on top.
7. **ADR-significance check:** yes — this is a new recurring security-guardrail pattern.
   Covered by `dotfiles` ADR-0015 (already written, cross-cutting since it applies to
   etch-cli + math + future repos) — not duplicated here as a repo-specific ADR.
