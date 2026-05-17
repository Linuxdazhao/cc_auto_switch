# Cross-Platform Support Design Spec

**Date:** 2026-05-17
**Topic:** Add full Windows and Linux support to cc-switch
**Status:** Approved

## Overview

This spec outlines the implementation plan for adding comprehensive cross-platform support to cc-switch. The goal is to make cc-switch a first-class citizen on Windows and Linux, with CI testing, binary releases, package manager distribution, and documentation.

**Scope covers both managed CLIs:**
- **Claude CLI** (launched from `src/utils.rs` and `src/interactive/interactive.rs`)
- **Codex CLI** (launched from `src/codex/commands.rs` and `src/interactive/codex_interactive.rs`, surfaced via the `cx` / `csd` subcommands)

Both are typically installed via `npm` on Windows, which means both ship as `.cmd` shims and share the same binary-resolution problem on that platform. Any fix for one must also be applied to the other.

## Goals

1. **CI/CD Pipeline** - Ensure code compiles and tests pass on Windows and Linux
2. **Binary Distribution** - Provide pre-built binaries for all platforms via GitHub Releases
3. **Package Manager Support** - Enable installation via Scoop (Windows) and Linuxbrew (Linux)
4. **Shell Integration** - Support PowerShell, CMD, and Git Bash on Windows
5. **Documentation** - Clear installation instructions for all platforms

## Non-Goals

- WinGet/MSI packaging (deferred, more complex)
- Chocolatey package (Scoop is sufficient for now)
- Linux .deb/.rpm packages (Homebrew + cargo install covers most users)

## Architecture

### Component Changes

| Component | Current State | Target State |
|-----------|---------------|--------------|
| **CI Test Matrix** | Ubuntu + macOS | Ubuntu + macOS + Windows |
| **Release Artifacts** | Linux + macOS (4 binaries) | Linux + macOS + Windows (6 binaries) |
| **Package Managers** | Homebrew (macOS) | Homebrew + Scoop + Linuxbrew |
| **Shell Support** | fish, zsh, bash, powershell | Same (verify PowerShell works) |
| **Documentation** | macOS/Linux focused | All platforms |

## Implementation Details

### 1. CI Pipeline Changes

#### 1.1 Test Matrix

Add `windows-latest` to the existing test job. `cargo test` is shell-independent (it runs the same compiled test binary regardless of which shell invoked cargo), so we do **not** add a separate per-shell matrix — that would burn CI minutes without testing anything new. Cross-shell concerns (completion script loading, interactive UI rendering) are covered by a dedicated smoke job in §1.4.

```yaml
jobs:
  test:
    name: Test on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Cache cargo + target
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test

      - name: Build in release mode
        run: cargo build --release
```

Notes on the change from the existing workflow:
- `dtolnay/rust-toolchain@stable` + `toolchain: nightly` (current `ci.yml:25-28`) is confusing: `@stable` is the action version, `toolchain:` is what gets installed. Since `CLAUDE.md` declares Rust 1.88+ and nothing in the code uses nightly features, switch to `@v1` + `toolchain: stable`.
- `Swatinem/rust-cache@v2` replaces the hand-rolled `actions/cache@v3` block. It handles `CARGO_HOME` resolution cross-platform (including Windows' `C:\Users\runneradmin\.cargo`), avoids `target/` corruption between targets, and is the de-facto Rust-on-CI cache.

#### 1.2 Cache

Handled by `Swatinem/rust-cache@v2` in §1.1 — no manual path juggling needed.

#### 1.3 Cross-Build Matrix

Add Windows targets to the cross-build job. **Important:** `aarch64-pc-windows-msvc` on `windows-latest` (an x86_64 host) is a cross-compile — the resulting binary cannot be executed by the runner, so it must live in `cross-build` / release matrices only, never in the `test` matrix.

```yaml
cross-build:
  name: Cross-build for ${{ matrix.target }}
  runs-on: ${{ matrix.os }}
  strategy:
    matrix:
      include:
        - target: x86_64-unknown-linux-gnu
          os: ubuntu-latest
        - target: aarch64-unknown-linux-gnu
          os: ubuntu-latest
        - target: x86_64-apple-darwin
          os: macos-latest
        - target: aarch64-apple-darwin
          os: macos-latest
        - target: x86_64-pc-windows-msvc
          os: windows-latest
        - target: aarch64-pc-windows-msvc
          os: windows-latest  # cross-compile only; no test execution
```

#### 1.4 Shell Integration Smoke Test

The genuine cross-shell concerns are completion script generation and interactive UI behavior, not `cargo test`. Add a small Windows-only smoke job:

```yaml
shell-smoke:
  name: Shell smoke (${{ matrix.shell }})
  runs-on: windows-latest
  needs: test
  strategy:
    matrix:
      shell: [pwsh, bash]  # cmd has no completion; covered by manual checklist
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@v1
      with:
        toolchain: stable
    - uses: Swatinem/rust-cache@v2
    - name: Build release binary
      run: cargo build --release

    - name: Verify completion script loads (PowerShell)
      if: matrix.shell == 'pwsh'
      shell: pwsh
      run: |
        $script = & ./target/release/cc-switch.exe completion powershell | Out-String
        Invoke-Expression $script  # fails if the generated script has syntax errors

    - name: Verify completion script loads (Bash)
      if: matrix.shell == 'bash'
      shell: bash
      run: |
        ./target/release/cc-switch.exe completion bash > /tmp/cc.bash
        bash -c "source /tmp/cc.bash"

    - name: Verify list/add/remove round-trip
      shell: ${{ matrix.shell }}
      run: |
        ./target/release/cc-switch.exe add ci-test -t sk-test -u https://example.com
        ./target/release/cc-switch.exe list
        ./target/release/cc-switch.exe remove ci-test
```

Interactive UI rendering (crossterm under conhost vs. Windows Terminal) cannot be automated meaningfully in CI — it stays on the manual testing checklist (§5.3).

### 2. Release Workflow Changes

#### 2.1 Add Windows Builds

Extend the release matrix to include Windows binaries. Artifact names follow the **existing convention** (no version in filename — version lives in the GitHub release tag path):

```yaml
jobs:
  build:
    name: Build for ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact_name: cc-switch-x86_64-unknown-linux-gnu.tar.gz
            binary_name: cc-switch
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            artifact_name: cc-switch-aarch64-unknown-linux-gnu.tar.gz
            binary_name: cc-switch
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact_name: cc-switch-x86_64-apple-darwin.tar.gz
            binary_name: cc-switch
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact_name: cc-switch-aarch64-apple-darwin.tar.gz
            binary_name: cc-switch
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            artifact_name: cc-switch-x86_64-pc-windows-msvc.zip
            binary_name: cc-switch.exe
          - target: aarch64-pc-windows-msvc
            os: windows-latest  # cross-compile, no smoke test on this artifact
            artifact_name: cc-switch-aarch64-pc-windows-msvc.zip
            binary_name: cc-switch.exe
```

Download URL on release becomes:
`https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v<VERSION>/cc-switch-<TARGET>.zip`

Anything that references these URLs (Scoop manifest, Homebrew formula, README) **must** use this exact filename pattern.

#### 2.3 Release Job Must Upload `.zip` Artifacts

The current `release.yml:102` only globs tar.gz:

```yaml
files: |
  artifacts/**/*.tar.gz
```

This silently drops the Windows `.zip` artifacts built by the matrix. **Fix to:**

```yaml
files: |
  artifacts/**/*.tar.gz
  artifacts/**/*.zip
```

Also update the "Release Summary" loop (`release.yml:118-124`) — it currently only iterates `artifacts/**/*.tar.gz` for the markdown summary table. Extend to handle both extensions, e.g.:

```bash
for artifact in artifacts/**/*.tar.gz artifacts/**/*.zip; do
  if [ -f "$artifact" ]; then
    filename=$(basename "$artifact")
    target_name=$(echo "$filename" | sed -E 's/\.(tar\.gz|zip)$//')
    echo "| $target_name | $filename |" >> $GITHUB_STEP_SUMMARY
  fi
done
```

Without this fix, the Windows binaries are built but never published, and `scoop install` will 404 on the download URL — easy to miss because the release job still "succeeds."

#### 2.2 Packaging Steps

**Windows (PowerShell):**
```powershell
# Package as .zip
New-Item -ItemType Directory -Force -Path "dist/${{ matrix.target }}"
Copy-Item "target/${{ matrix.target }}/release/cc-switch.exe" "dist/${{ matrix.target }}/cc-switch.exe"
Compress-Archive -Path "dist/${{ matrix.target }}/cc-switch.exe" -DestinationPath "${{ matrix.artifact_name }}"
```

**Linux/macOS (Bash):**
```bash
# Package as .tar.gz
mkdir -p dist/${{ matrix.target }}
cp target/${{ matrix.target }}/release/cc-switch dist/${{ matrix.target }}/cc-switch
cd dist/${{ matrix.target }}
tar -czf ../../${{ matrix.artifact_name }} cc-switch
```

### 3. Package Manager Distribution

#### 3.1 Scoop Bucket (Windows)

Create a Scoop bucket repository at `Linuxdazhao/scoop-cc-switch`:

**Repository structure:**
```
scoop-cc-switch/
├── bucket/
│   └── cc-switch.json
└── README.md
```

**Scoop manifest (`bucket/cc-switch.json`):** URLs follow the §2.1 naming (no version in filename). The `autoupdate.hash` block tells Scoop how to find the SHA256 — without it, hashes still need a manual update on every release.

```json
{
    "version": "0.1.17",
    "description": "A CLI tool for managing multiple Claude API configurations and automatically switching between them",
    "homepage": "https://github.com/Linuxdazhao/cc_auto_switch",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v0.1.17/cc-switch-x86_64-pc-windows-msvc.zip",
            "hash": "SHA256_HASH_HERE",
            "bin": "cc-switch.exe"
        },
        "arm64": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v0.1.17/cc-switch-aarch64-pc-windows-msvc.zip",
            "hash": "SHA256_HASH_HERE",
            "bin": "cc-switch.exe"
        }
    },
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v$version/cc-switch-x86_64-pc-windows-msvc.zip",
                "hash": {
                    "url": "$url.sha256"
                }
            },
            "arm64": {
                "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v$version/cc-switch-aarch64-pc-windows-msvc.zip",
                "hash": {
                    "url": "$url.sha256"
                }
            }
        }
    }
}
```

**Initial version** (`0.1.17`) matches `Cargo.toml`; on every release, `scripts/release.sh` (or the GitHub Actions release workflow) must update the manifest. See §3.4 for the automation.

**Installation:**
```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

#### 3.2 Linuxbrew (Linux)

**Status: already working in production.** `brew install Linuxdazhao/cc-switch/cc-switch` has been verified on Linux — the existing `Linuxdazhao/homebrew-cc-switch` formula already has the `on_linux` blocks pointing at the `cc-switch-*-unknown-linux-gnu.tar.gz` artifacts. No formula changes are required for the initial cross-platform rollout.

What still needs to happen:
1. Keep the formula's Linux URLs in sync with the §2.1 artifact naming when versions bump (handled by the release automation in §3.4).
2. When publishing the README installation matrix (§4.1), point Linux users at `brew` as the primary install option without caveats.

Note: "bottles" in Homebrew terminology are formula-specific pre-built archives hosted on a bottle endpoint; the generic tar.gz binaries we publish are **not** bottles. We're shipping `url + sha256` source-style archives that Homebrew installs without compilation, which is sufficient for this project — no bottle infrastructure required.

#### 3.3 Cargo Install (All Platforms)

Already works on all platforms:
```bash
cargo install cc-switch
```

No changes needed, but verify in §5.3 that the published crate compiles cleanly on Windows (the test matrix only proves *this branch* compiles, not the latest crates.io version).

#### 3.4 Release Automation for Package Manifests

**Use GitHub Actions, not a local script.** The actual cross-platform binaries are built inside Actions runners, so only the post-release workflow has reliable access to the final published artifact hashes. A local `scripts/release.sh` would need to either re-download the assets (racy — they may not be live yet) or recompute hashes from the local build (which never includes the Windows artifacts cross-compiled on the runner).

The Homebrew formula is already updated this way by `.github/workflows/update-brew-formula.yml` — Scoop should mirror that pattern exactly.

**Add `.github/workflows/update-scoop-manifest.yml`** with the same shape:

1. Trigger: `on: release: { types: [published] }` + `workflow_dispatch` for backfill.
2. Wait for the Windows `.zip` assets to appear on the release (reuse the `curl --head` polling loop from `update-brew-formula.yml:44-87`, with the FILES array set to `cc-switch-x86_64-pc-windows-msvc.zip` and `cc-switch-aarch64-pc-windows-msvc.zip`).
3. Download each `.zip` and compute SHA256 (reuse the `calculate_sha` helper from `update-brew-formula.yml:89-162`).
4. Checkout `Linuxdazhao/scoop-cc-switch` with a token (`SCOOP_BUCKET_TOKEN` or `GITHUB_TOKEN` if same-owner permissions are enough — same convention as `HOMEBREW_TAP_TOKEN`).
5. Render `bucket/cc-switch.json` from a template (`.github/workflows/scoop-manifest-template.json`) with `sed`-replaced `__VERSION__`, `__X64_SHA__`, `__ARM64_SHA__` placeholders — same pattern as `formula-template.rb`.
6. Commit + push to the bucket repo's `main` branch.

**Why mirror the existing pattern rather than invent a new one:**
- Already-debugged retry/timeout logic for "assets not yet live"
- Already-debugged token handling
- One mental model for both package manager updates — anyone who can debug the brew workflow can debug the scoop one

**`update-brew-formula.yml` itself does not need changes**: it only consumes `*.tar.gz` files (lines 51-56), and no Linux/macOS tarball names are changing.

### 4. Documentation Updates

#### 4.1 README Installation Section

Add comprehensive installation instructions for all platforms:

```markdown
## Installation

### Windows

**Option 1: Scoop (Recommended)**
```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

**Option 2: Download Binary**
Download the latest `.zip` from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) and add to PATH.

**Option 3: Cargo**
```powershell
cargo install cc-switch
```

### Linux

**Option 1: Homebrew (Recommended)**
```bash
# Short form works because the tap repo is named `homebrew-cc-switch`
brew install Linuxdazhao/cc-switch/cc-switch
# Equivalent explicit form:
# brew tap Linuxdazhao/cc-switch && brew install cc-switch
```

**Option 2: Download Binary**
Download the latest `.tar.gz` from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) and add to PATH.

**Option 3: Cargo**
```bash
cargo install cc-switch
```

### macOS

**Option 1: Homebrew (Recommended)**
```bash
brew install Linuxdazhao/cc-switch/cc-switch
```

**Option 2: Download Binary**
Download the latest `.tar.gz` from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) and add to PATH.

**Option 3: Cargo**
```bash
cargo install cc-switch
```

### Shell Setup

**PowerShell (Windows)**

Do **not** use `cc-switch completion powershell | Out-File $PROFILE` — `Out-File` overwrites the file, destroying any other content in the user's profile (aliases, theme setup, module imports, etc.). Write the completion to a dedicated file and dot-source it from `$PROFILE` instead:

```powershell
# One-time setup
$completionDir = Split-Path -Parent $PROFILE
if (-not (Test-Path $completionDir)) { New-Item -ItemType Directory -Path $completionDir | Out-Null }
$completionPath = Join-Path $completionDir 'cc-switch.completion.ps1'
cc-switch completion powershell | Out-File -Encoding utf8 $completionPath

# Append a single source line to the profile if it isn't already there
$line = ". '$completionPath'"
if (-not ((Test-Path $PROFILE) -and (Select-String -Path $PROFILE -Pattern ([regex]::Escape($line)) -Quiet))) {
    Add-Content -Path $PROFILE -Value $line
}
```

This is idempotent (safe to re-run) and never overwrites existing profile content.

**Git Bash (Windows)**
```bash
cc-switch completion bash >> ~/.bashrc
source ~/.bashrc
```

**CMD (Windows)**
CMD doesn't support advanced completion, but basic usage works:
```cmd
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
cc-switch list
```
```

### 5. Code Audit for Cross-Platform Issues

#### 5.1 Already Cross-Platform (No Changes Needed)

Based on code review, the codebase already uses cross-platform crates:

- **Path handling**: `dirs` crate handles home directory paths on all platforms
- **Terminal UI**: `crossterm` handles terminal manipulation cross-platform
- **JSON serialization**: `serde_json` is platform-agnostic
- **Error handling**: `anyhow` works on all platforms
- **No Windows-specific conditional dependencies needed** in `Cargo.toml` based on current code; revisit only if §5.2 fixes pull in a Windows-only crate (e.g., `windows-sys`).

#### 5.2 Issues That Require Code Changes

| Component | Concern | Required fix |
|-----------|---------|--------------|
| **Npm-shim binary resolution (`claude` and `codex`)** | Both Claude CLI and Codex CLI on Windows ship as `.cmd` / `.ps1` shims from npm, with `.exe` as a less common alternative. `std::process::Command::new("claude")` (or `"codex"`) on Windows relies on `PATHEXT` resolution and may fail for `.cmd` shims depending on how PATH is configured. | Add a single shared `#[cfg(windows)]` helper, e.g. `resolve_npm_cli(name: &str) -> PathBuf`, that probes `<name>.exe`, `<name>.cmd`, `<name>.ps1` in order (using the `which` crate or manual PATH walk), respecting an env-var override (`CLAUDE_BINARY`, `CODEX_BINARY`). Apply it at every `Command::new("claude")` and `Command::new("codex")` call site (listed in the next two rows). |
| **`claude` launch call sites** | All three call sites currently use bare `Command::new("claude")`. `src/utils.rs:79-103` has no cfg gating at all. | Wire the resolver into: `src/utils.rs:80` (`execute_claude_command`), `src/utils.rs:108` (`launch_claude`), `src/interactive/interactive.rs:991, 1011, 1050, 1064`. Gate any Unix-only logic (`os::unix::process::CommandExt::exec`) with `#[cfg(unix)]`. |
| **`codex` launch call sites** | `src/codex/commands.rs:231` (`launch_codex` — bare, no cfg gating) and `src/interactive/codex_interactive.rs:566, 575` (already has `#[cfg(unix)]` / `#[cfg(not(unix))]` split, but the `not(unix)` branch still uses bare `"codex"`). | Wire the same resolver into all three sites. The interactive file's cfg split is already correct — just swap the bare strings for the resolver call. |
| **Interactive UI** | Crossterm on Windows Terminal vs. legacy conhost vs. CMD. | Manual verification (§5.3). Crossterm enables VT processing on Windows 10+ automatically. |
| **Border drawing** | Unicode support detection in `interactive.rs:73-92` returns `true` by default; Windows conhost without UTF-8 codepage may render mojibake. | Add a `#[cfg(windows)]` check for `GetConsoleOutputCP() == CP_UTF8`, or expose a `--ascii` flag as a manual override. |
| **Shell completion** | PowerShell script correctness — `clap_complete` generates it but it should be loadable. | Covered by the §1.4 smoke job. |
| **Path separators** | Windows uses `\`, Unix uses `/`. | `std::path::PathBuf` handles this automatically; no fix needed. |
| **Line endings in generated completion scripts** | *No code change needed.* `clap_complete` writes to stdout; the shell that does `cc-switch completion bash > file` chooses the line endings. The README documents using Git Bash (LF) for bash redirects and dot-sourcing for PowerShell (CRLF-tolerant), which removes the practical risk. Add a `--output` flag with binary-mode writes only if a real user report appears. | Documentation only (§4.1). |
| **Homebrew formula `test do` block is broken** | `.github/workflows/formula-template.rb:31` asserts the output of `cc-switch version`, but the CLI exposes the version only via `--version` (verified: `cc-switch version` errors with `unrecognized subcommand 'version'`). `brew test` currently fails on any platform — including the existing macOS/Linux ones. This is a pre-existing bug, but it will surface immediately when we start running `brew test` as part of Phase 4 validation. | Change line 31 to `assert_match version.to_s, shell_output("#{bin}/cc-switch --version")`. Patch the template in this repo *and* re-render the published formula via the next `update-brew-formula.yml` run. |

#### 5.3 Testing Checklist

CI automation (§1.1, §1.4) covers:
- [x] `cargo test` on Windows x86_64
- [x] PowerShell completion script loads
- [x] Bash completion script loads on Git Bash
- [x] `add` / `list` / `remove` round-trip on Windows

Remaining manual verification:
- [ ] Test interactive UI on Windows Terminal (Unicode borders render)
- [ ] Test interactive UI on legacy conhost / CMD (ASCII fallback if needed)
- [ ] Verify `~/.claude/cc_auto_switch_setting.json` resolves correctly on Windows (`C:\Users\<user>\.claude\cc_auto_switch_setting.json`) — this is the current canonical path; the legacy `~/.cc-switch/configurations.json` is only read for auto-migration (see `config_storage.rs:10-71`)
- [ ] Verify `~/.codex/auth.json` writes correctly on Windows (`C:\Users\<user>\.codex\auth.json`)
- [ ] Verify auto-migration from the legacy `~/.cc-switch/configurations.json` path works on Windows (drop a file at that path before first run)
- [ ] Verify `claude` launch works when Claude CLI is installed via `npm i -g @anthropic-ai/claude-code` on Windows (tests the §5.2 resolver fix)
- [ ] Verify `codex` launch works when Codex CLI is installed via npm on Windows (same resolver, separate binary)
- [ ] Verify `cx` and `csd` subcommands work end-to-end on Windows (add → list → use → remove)
- [ ] `cargo install cc-switch` from crates.io compiles & runs on a clean Windows VM (validates the published crate, not just this branch)
- [ ] `scoop install cc-switch` after publishing manifest
- [x] `brew install Linuxdazhao/cc-switch/cc-switch` on Linux — **already confirmed working**; re-verify after release automation goes live
- [ ] `brew test cc-switch` passes on both macOS and Linux **after the §5.2 `--version` fix lands** (it currently fails on every platform due to the broken `cc-switch version` assertion)

## Testing Strategy

### CI Testing

1. **Unit tests**: Run on all three platforms (Windows, Linux, macOS)
2. **Integration tests**: Run on all three platforms
3. **Shell-specific tests**: Test on PowerShell, CMD, and Git Bash on Windows

### Manual Testing

Before release, manually test on:
- Windows 10/11 (x86_64)
- Windows on ARM (if hardware available)
- Ubuntu 22.04 (x86_64)
- Ubuntu 22.04 (aarch64, via Docker or VM)

### Test Scenarios

1. **Fresh install**: Install via package manager, verify binary runs
2. **Add configuration**: Run `cc-switch add my-config -t xxx -u yyy`
3. **List configurations**: Run `cc-switch list`
4. **Switch configuration**: Run `cc-switch` (interactive mode)
5. **Shell completion**: Verify completion works in each shell
6. **Remove configuration**: Run `cc-switch remove my-config`

## Rollout Plan

Phases are ordered by dependency, not calendar weeks — a determined contributor can compress Phase 1–3 into a few days; Phase 4 and 5 require external repos and real Windows/Linux hosts for verification.

### Phase 1: Code Fixes for Windows (blocking)

1. Implement shared `resolve_npm_cli(name)` helper (`#[cfg(windows)]` probe for `.exe` / `.cmd` / `.ps1` + env-var override).
2. Apply the resolver at every `Command::new("claude")` site: `src/utils.rs:80, 108`, `src/interactive/interactive.rs:991, 1011, 1050, 1064`.
3. Apply the resolver at every `Command::new("codex")` site: `src/codex/commands.rs:231`, `src/interactive/codex_interactive.rs:566, 575`.
4. Add `#[cfg(windows)]` UTF-8 codepage check in `interactive.rs::detect_unicode_support` (or fall back to ASCII when in doubt).
5. Normalize line endings when writing completion scripts.

### Phase 2: CI Pipeline

1. Swap `dtolnay/rust-toolchain@stable` → `@v1` with explicit `toolchain: stable`.
2. Swap hand-rolled cache for `Swatinem/rust-cache@v2`, bump any remaining `actions/cache@v3` → `@v4`.
3. Add `windows-latest` to the `test` matrix (x86_64 only).
4. Add `aarch64-pc-windows-msvc` to `cross-build` matrix (build only, no test).
5. Add the §1.4 `shell-smoke` job.

### Phase 3: Release Workflow

1. Add Windows targets to release matrix (x86_64 + aarch64 MSVC).
2. Add PowerShell packaging step for `.zip` artifacts.
3. Tag a pre-release (e.g., `v0.1.18-rc1`) to validate end-to-end.
4. Verify the produced URLs match the §2.1 pattern exactly.

### Phase 4: Package Manager Distribution

1. Create `Linuxdazhao/scoop-cc-switch` repo with the §3.1 manifest.
2. Implement release automation (§3.4 option 2): extend `scripts/release.sh` to compute SHA256s and push manifest/formula updates to both `scoop-cc-switch` and `homebrew-cc-switch`.
3. Test `scoop install cc-switch` on a Windows host.
4. Re-verify `brew install Linuxdazhao/cc-switch/cc-switch` on Linux after the first automated release to confirm the automation didn't break the existing formula.

### Phase 5: Documentation

1. Update README with the §4.1 installation matrix.
2. Add shell setup instructions for PowerShell, CMD, Git Bash.
3. Update `CLAUDE.md` with the Windows binary-resolver behavior so future contributors don't undo it.
4. Update CLI help text if any commands gain platform-specific flags (e.g., `--ascii` override).

## Success Criteria

- [ ] CI passes on Windows, Linux, and macOS (x86_64 runs full `cargo test`; aarch64-pc-windows-msvc cross-builds only)
- [ ] `shell-smoke` job passes (PowerShell + Git Bash completion scripts load; CRUD round-trip succeeds)
- [ ] Windows binaries (x86_64 + ARM64) available in GitHub Releases under the §2.1 naming
- [ ] Linux binaries (x86_64 + aarch64) available in GitHub Releases
- [ ] `claude` CLI launches successfully from `cc-switch` on Windows when installed via npm
- [ ] `codex` CLI launches successfully from `cc-switch` (`cx` / `csd` subcommands) on Windows when installed via npm
- [ ] Scoop installation works: `scoop install cc-switch`, including auto-update on the next release
- [x] Linuxbrew installation works: `brew install Linuxdazhao/cc-switch/cc-switch` on Linux (**already verified in production** — guard against regression in Phase 4)
- [ ] Interactive UI renders correctly on Windows Terminal (Unicode borders) and degrades to ASCII on legacy conhost
- [ ] README has clear installation instructions for all platforms
- [ ] All existing macOS functionality preserved (no regressions in `cargo test` on macos-latest)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Windows ARM64 cross-compilation fails | Cannot support Windows on ARM | Build only (no test execution required); fallback is to drop ARM64 from the release matrix in v1 and add later when a GitHub-hosted Windows ARM64 runner is generally available |
| `claude` or `codex` binary not found on Windows (npm shim is `.cmd`) | `cc-switch` / `cx` / `csd` "switches" config but fails to launch the target CLI — silent broken UX | Shared Phase 1 `resolve_npm_cli` helper probing PATHEXT variants; covered by §5.3 manual checklist items for both binaries |
| Crossterm renders mojibake on legacy conhost | Interactive UI unreadable | UTF-8 codepage probe + ASCII fallback (§5.2); document Windows Terminal as recommended host |
| Scoop manifest hash mismatch after release | `scoop install` fails until hash is fixed manually | `autoupdate.hash.url` block in manifest + release automation in §3.4 |
| Release automation regresses the existing Linux `brew install` flow | Linux users (currently working) suddenly can't install | The current formula's `on_linux` block is already verified working — Phase 4 step 4 re-tests `brew install` on Linux after the first automated release so any regression is caught before users notice |
| Release automation pushes broken manifest before binaries finish uploading | Users hit a window where the manifest references a 404 URL | Run manifest-update job with `needs: [build, release]` so it only fires after release artifacts are live |
| `npm` Claude/Codex CLI installs at different paths across Windows user profiles | Resolver helper picks the wrong binary | Document `CLAUDE_BINARY` / `CODEX_BINARY` env-var overrides; resolver checks them first |

## Future Enhancements (Out of Scope)

- WinGet/MSI packaging for Windows Store
- Chocolatey package
- Linux .deb/.rpm packages for apt/dnf
- Auto-update mechanism
- GUI installer for Windows

## References

- [Cargo cross-compilation guide](https://doc.rust-lang.org/cargo/reference/config.html#target)
- [GitHub Actions Windows runners](https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-runners-and-hardware-resources)
- [Scoop manifest reference](https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests)
- [Linuxbrew documentation](https://docs.brew.sh/Homebrew-on-Linux)
