> **Status: DONE**

# SBOM + Cosign Signing — etch-cli Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add keyless cosign signing and syft SBOM generation to etch-cli releases via a reusable `release-sign.yml` workflow. Each release gains three extra assets: `.sig`, `.pem`, `.sbom.spdx.json`.

**Architecture:** A reusable `release-sign.yml` workflow downloads the release binary from GitHub, generates an SPDX JSON SBOM with syft, signs the binary with cosign (keyless/Sigstore OIDC), and uploads all three attestation files back to the same release. The existing `release.yml` gains a `sign` job that calls this workflow. Verification command added to README.

**Tech Stack:** `sigstore/cosign-installer@v3`, `anchore/sbom-action/download-syft@v0`, `gh release download/upload`, `actions/checkout@v5`

**Prerequisite:** Plan 1 (release.yml) must be merged first.

---

## Files

- **Create:** `.github/workflows/release-sign.yml`
- **Modify:** `.github/workflows/release.yml` — add `sign` job
- **Modify:** `README.md` — add "Verifying releases" section
- **Modify:** `docs/superpowers/README.md` — **post-merge on main only**

---

## Task 1: Create release-sign.yml

**Files:**

- Create: `.github/workflows/release-sign.yml`

- [ ] **Step 1: Create `.github/workflows/release-sign.yml`**

```yaml
name: Release Sign

on:
    workflow_call:
        inputs:
            release_tag:
                required: true
                type: string
            binary_name:
                required: true
                type: string

jobs:
    sign:
        name: SBOM + Sign
        runs-on: ubuntu-latest
        permissions:
            id-token: write
            contents: write
        steps:
            - uses: actions/checkout@v5

            - name: Download binary from release
              env:
                  GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
              run: |
                  gh release download "${{ inputs.release_tag }}" \
                    --pattern "${{ inputs.binary_name }}"

            - name: Install syft
              uses: anchore/sbom-action/download-syft@v0

            - name: Generate SBOM
              run: |
                  syft "${{ inputs.binary_name }}" \
                    -o spdx-json \
                    --file "${{ inputs.binary_name }}.sbom.spdx.json"

            - name: Install cosign
              uses: sigstore/cosign-installer@v3

            - name: Sign binary (keyless)
              run: |
                  cosign sign-blob --yes "${{ inputs.binary_name }}" \
                    --output-signature "${{ inputs.binary_name }}.sig" \
                    --output-certificate "${{ inputs.binary_name }}.pem"

            - name: Upload signatures and SBOM to release
              env:
                  GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
              run: |
                  gh release upload "${{ inputs.release_tag }}" \
                    "${{ inputs.binary_name }}.sig" \
                    "${{ inputs.binary_name }}.pem" \
                    "${{ inputs.binary_name }}.sbom.spdx.json"
```

- [ ] **Step 2: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-sign.yml'))" && echo "valid"
```

Expected: `valid`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-sign.yml
git commit -m "ci: add reusable release-sign workflow (SBOM + cosign)

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Wire sign job into release.yml

**Files:**

- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Read current release.yml to find end of file**

The file ends after the `release` job. The `sign` job goes after it, at the same indentation level.

- [ ] **Step 2: Add sign job to release.yml**

Append after the `release` job (inside `jobs:`, same indentation as `release:`):

```yaml
sign:
    needs: [release]
    uses: ./.github/workflows/release-sign.yml
    with:
        release_tag: "v${{ inputs.version }}"
        binary_name: etch
    permissions:
        id-token: write
        contents: write
```

- [ ] **Step 3: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "valid"
```

Expected: `valid`

- [ ] **Step 4: Confirm both jobs present**

```bash
python3 -c "
import yaml
w = yaml.safe_load(open('.github/workflows/release.yml'))
print(list(w['jobs'].keys()))
"
```

Expected: `['release', 'sign']`

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: wire sign job into etch-cli release workflow

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Add verification docs to README

**Files:**

- Modify: `README.md`

- [ ] **Step 1: Find a good insertion point in README.md**

```bash
grep -n "## " README.md | head -15
```

Find a section near the end (e.g. after "## License" or "## Contributing") to insert the verification section.

- [ ] **Step 2: Add "Verifying releases" section**

Add the following section to `README.md` (after the last major section, before any footer):

````markdown
## Verifying releases

Release binaries are signed with [cosign](https://docs.sigstore.dev/cosign/overview/) using keyless Sigstore signing. Each release includes:

- `etch` — compiled binary
- `etch.sig` — detached signature
- `etch.pem` — signing certificate
- `etch.sbom.spdx.json` — SPDX bill of materials

To verify a release binary:

```bash
cosign verify-blob etch \
  --signature etch.sig \
  --certificate etch.pem \
  --certificate-identity \
    "https://github.com/brujack/etch-cli/.github/workflows/release-sign.yml@refs/tags/TAG" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"
```
````

Replace `TAG` with the release tag (e.g. `v1.2.0`).

````

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add release verification instructions

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
````

---

## Task 4: Post-merge docs update

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plan index**

In `docs/superpowers/README.md`, update the etch-cli sbom-cosign row: add plan link, set status to Done.

- [ ] **Step 2: Add Done banner**

Add `> **Status: DONE**` at the top of `docs/superpowers/plans/2026-05-20-sbom-cosign.md`.

- [ ] **Step 3: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-20-sbom-cosign.md
git commit -m "docs: mark etch-cli sbom-cosign plan done

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```
