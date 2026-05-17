# Cross-Platform Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make cc-switch a first-class CLI on Windows (x86_64 + ARM64) and Linux while preserving every existing macOS/Linux behavior — covering Rust code fixes, CI matrix, release pipeline, Scoop bucket, Linuxbrew verification, and documentation.

**Architecture:**
- Add a small `platform` module that resolves npm-shim binaries (`claude`, `codex`) on Windows by probing `.exe` / `.cmd` / `.ps1`. Every existing `Command::new("claude")` and `Command::new("codex")` call site is routed through it.
- CI grows a `windows-latest` test job, an `aarch64-pc-windows-msvc` cross-build, and a Windows-only `shell-smoke` job that verifies completion script generation. Release workflow grows Windows `.zip` artifacts and `.sha256` sidecars.
- Package-manager updates mirror the existing `update-brew-formula.yml` pattern: a new `update-scoop-manifest.yml` triggered on `release: published` writes the new version + computed hash into `Linuxdazhao/scoop-cc-switch/bucket/cc-switch.json`. Linuxbrew is already working; we only patch the formula's broken `cc-switch version` test.

**Tech Stack:** Rust 1.88+ / edition 2024, `clap` + `clap_complete`, `crossterm`, `which`, GitHub Actions (with `Swatinem/rust-cache@v2`, `softprops/action-gh-release@v1`), Scoop, Homebrew.

**Spec:** `docs/superpowers/specs/2026-05-17-cross-platform-support-design.md`

---

## File Structure

### Created
- `src/platform.rs` — npm-shim binary resolver and Unicode-support helper
- `tests/platform_tests.rs` — unit tests for the resolver and Unicode override
- `.github/workflows/scoop-manifest-template.json` — placeholder-based Scoop manifest template (mirrors `formula-template.rb`)
- `.github/workflows/update-scoop-manifest.yml` — post-release workflow that writes version + hashes into the Scoop bucket repo
- `docs/superpowers/plans/2026-05-17-cross-platform-support.md` — this file (already created)

### Modified
- `Cargo.toml` — add `which` dependency (all platforms; tiny crate, ~50 KB)
- `src/lib.rs` — export the new `platform` module
- `src/utils.rs:80, 108` — route `claude` launches through `resolve_npm_cli`
- `src/interactive/interactive.rs:73-92, 991, 1011, 1050, 1064` — route `claude` launches through `resolve_npm_cli`; honor `CC_SWITCH_ASCII` and `WT_SESSION` in `detect_unicode_support`
- `src/codex/commands.rs:231` — route `codex` launch through `resolve_npm_cli`
- `src/interactive/codex_interactive.rs:566, 575` — route `codex` launches through `resolve_npm_cli`
- `.github/workflows/formula-template.rb:31` — fix the `cc-switch version` assertion (CLI exposes only `--version`)
- `.github/workflows/ci.yml` — add Windows to test matrix, add ARM64 Windows to cross-build, swap cache to `Swatinem/rust-cache@v2`, swap `dtolnay/rust-toolchain@stable` → `@v1` + explicit `toolchain: stable`, add shell-smoke job, bump `actions/cache@v3` → `@v4`
- `.github/workflows/release.yml` — add Windows targets, PowerShell `.zip` packaging step, fix upload glob to include `*.zip` and `*.sha256`, fix release summary loop, generate `.sha256` sidecars
- `README.md` — add per-platform installation matrix, PowerShell completion setup (the safe append-to-`$PROFILE` form, not `Out-File`)
- `CLAUDE.md` — document the Windows resolver behavior and the `CLAUDE_BINARY` / `CODEX_BINARY` env-var overrides

### External (out-of-repo work the operator must do)
- Create the `Linuxdazhao/scoop-cc-switch` GitHub repo with an initial commit containing `bucket/cc-switch.json` populated for v0.1.17
- (Optional) Configure a `SCOOP_BUCKET_TOKEN` secret on the main repo if `GITHUB_TOKEN` can't push to the new bucket repo

---

## Phase 1 — Windows Code Fixes (BLOCKING — must land before Phases 2-5)

### Task 1: Add `which` crate dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

Run:

```bash
cargo add which@7
```

This appends `which = "7"` to `[dependencies]` in `Cargo.toml`. The `which` crate handles cross-platform PATH lookup including Windows `PATHEXT` resolution.

- [ ] **Step 2: Verify the build still compiles**

Run:

```bash
cargo check
```

Expected: `Finished \`dev\` profile [...]`. No errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add which crate for cross-platform PATH lookup"
```

---

### Task 2: Write failing tests for the npm-shim resolver

**Files:**
- Create: `tests/platform_tests.rs`

- [ ] **Step 1: Write the test file**

Write the following to `tests/platform_tests.rs`:

```rust
//! Tests for the platform resolver helper.
//!
//! These tests are platform-independent — Windows-only PATHEXT probing is
//! exercised by the CI `shell-smoke` job, not here.

use cc_switch::platform::{resolve_npm_cli, unicode_support_enabled};
use std::path::PathBuf;

#[test]
fn env_override_takes_precedence() {
    // SAFETY: tests do not run in parallel within a single test binary by default
    //         only when using cargo nextest. The env-var name is unique to this test.
    unsafe {
        std::env::set_var("FOOTESTBIN_BINARY", "/custom/path/to/foo");
    }
    let resolved = resolve_npm_cli("footestbin");
    unsafe {
        std::env::remove_var("FOOTESTBIN_BINARY");
    }
    assert_eq!(resolved, PathBuf::from("/custom/path/to/foo"));
}

#[test]
fn no_override_no_path_match_falls_back_to_bare_name() {
    let unique = "definitely_nonexistent_binary_xyz123abc";
    let resolved = resolve_npm_cli(unique);
    assert_eq!(resolved, PathBuf::from(unique));
}

#[test]
fn ascii_env_override_forces_ascii() {
    unsafe {
        std::env::set_var("CC_SWITCH_ASCII", "1");
    }
    let supported = unicode_support_enabled();
    unsafe {
        std::env::remove_var("CC_SWITCH_ASCII");
    }
    assert!(!supported, "CC_SWITCH_ASCII=1 must force ASCII fallback");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test platform_tests
```

Expected: compile error like `unresolved import \`cc_switch::platform\`` — the module does not exist yet.

---

### Task 3: Implement `src/platform.rs`

**Files:**
- Create: `src/platform.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create the module**

Write the following to `src/platform.rs`:

```rust
//! Cross-platform helpers for binary resolution and terminal capability detection.
//!
//! `resolve_npm_cli` exists because npm-installed CLIs on Windows ship as `.cmd` or
//! `.ps1` shims rather than `.exe`, and `std::process::Command::new("name")` does
//! not always pick them up depending on how PATHEXT is configured.
//!
//! `unicode_support_enabled` centralizes the heuristic used by the interactive UI
//! to decide between Unicode box-drawing and ASCII fallback.

use std::path::PathBuf;

/// Resolve a Node/npm-style CLI name to an executable path.
///
/// Resolution order:
/// 1. Env var `<NAME_UPPERCASE>_BINARY` (e.g. `CLAUDE_BINARY`, `CODEX_BINARY`) —
///    if set, returned verbatim. Lets users pin a specific binary.
/// 2. On Windows: probe `<name>.exe`, then `<name>.cmd`, then `<name>.ps1` via
///    the `which` crate (which respects PATH and is independent of cmd.exe's
///    PATHEXT handling).
/// 3. Fallback: return `PathBuf::from(name)` so `Command::new()` behaves
///    identically to the pre-resolver code on platforms where probing finds
///    nothing.
pub fn resolve_npm_cli(name: &str) -> PathBuf {
    let override_var = format!("{}_BINARY", name.to_uppercase());
    if let Ok(path) = std::env::var(&override_var) {
        return PathBuf::from(path);
    }

    #[cfg(windows)]
    {
        for ext in ["exe", "cmd", "ps1"] {
            let candidate = format!("{name}.{ext}");
            if let Ok(path) = which::which(&candidate) {
                return path;
            }
        }
    }

    PathBuf::from(name)
}

/// Decide whether the terminal supports Unicode box-drawing characters.
///
/// Precedence (highest first):
/// 1. `CC_SWITCH_ASCII=1` → force ASCII (escape hatch for any terminal).
/// 2. `CC_SWITCH_UNICODE=1` → force Unicode (escape hatch for Windows users
///    on terminals our heuristic can't detect).
/// 3. On Windows: only enable Unicode if `WT_SESSION` is set (Windows
///    Terminal). Default to ASCII to avoid mojibake on legacy conhost / CMD
///    where the default codepage is not UTF-8.
/// 4. On non-Windows: inspect `TERM` and `LANG` for known-good values; default
///    to enabled.
pub fn unicode_support_enabled() -> bool {
    if std::env::var("CC_SWITCH_ASCII").is_ok_and(|v| v == "1") {
        return false;
    }
    if std::env::var("CC_SWITCH_UNICODE").is_ok_and(|v| v == "1") {
        return true;
    }

    #[cfg(windows)]
    {
        return std::env::var("WT_SESSION").is_ok();
    }

    #[cfg(not(windows))]
    {
        if let Ok(term) = std::env::var("TERM")
            && (term.contains("xterm")
                || term.contains("screen")
                || term == "tmux-256color")
        {
            return true;
        }
        if let Ok(lang) = std::env::var("LANG")
            && (lang.contains("UTF-8") || lang.contains("utf8"))
        {
            return true;
        }
        true
    }
}
```

- [ ] **Step 2: Export the module from `lib.rs`**

In `src/lib.rs`, add `pub mod platform;` after the existing `pub mod utils;` line (the exact insertion point is after line 12):

```rust
pub mod claude_settings;
pub mod platform;
pub mod utils;
```

- [ ] **Step 3: Run the tests to verify they pass**

Run:

```bash
cargo test --test platform_tests
```

Expected: 3 passed (`env_override_takes_precedence`, `no_override_no_path_match_falls_back_to_bare_name`, `ascii_env_override_forces_ascii`).

- [ ] **Step 4: Run the full suite to confirm no regressions**

Run:

```bash
cargo test
```

Expected: all existing tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/platform.rs src/lib.rs tests/platform_tests.rs
git commit -m "feat(platform): add resolve_npm_cli and unicode_support_enabled helpers"
```

---

### Task 4: Route `claude` launches in `src/utils.rs` through the resolver

**Files:**
- Modify: `src/utils.rs:80, 108`

- [ ] **Step 1: Re-read the file to confirm current state**

Run:

```bash
sed -n '1,10p;75,125p' src/utils.rs
```

Confirm lines 80 and 108 still contain `Command::new("claude")` exactly.

- [ ] **Step 2: Add the import**

In `src/utils.rs`, near the top with the other `use` lines, add:

```rust
use crate::platform::resolve_npm_cli;
```

- [ ] **Step 3: Update both call sites**

Replace `Command::new("claude")` with `Command::new(resolve_npm_cli("claude"))` in **both** functions:

- `execute_claude_command` (around line 80)
- `launch_claude` (around line 108)

The argument is the only change — every chained `.arg()`, `.stdin()`, etc. stays the same.

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test
```

Expected: all tests still pass (the behavior on non-Windows is unchanged because the resolver falls back to `PathBuf::from("claude")`).

- [ ] **Step 5: Commit**

```bash
git add src/utils.rs
git commit -m "fix(utils): resolve claude binary via platform helper"
```

---

### Task 5: Route `claude` launches in `src/interactive/interactive.rs` through the resolver

**Files:**
- Modify: `src/interactive/interactive.rs:991, 1011, 1050, 1064`

- [ ] **Step 1: Re-read the relevant section**

Run:

```bash
sed -n '985,1080p' src/interactive/interactive.rs
```

Confirm all four `Command::new("claude")` call sites are still where the plan expects them (line numbers may drift if Task 4 changed line counts; the function names `execute_claude_with_resume` and `execute_claude_command` are the durable anchors).

- [ ] **Step 2: Add the import**

In `src/interactive/interactive.rs`, with the other `use crate::` lines near the top, add:

```rust
use crate::platform::resolve_npm_cli;
```

- [ ] **Step 3: Replace each call site**

For each `Command::new("claude")` in this file, change it to `Command::new(resolve_npm_cli("claude"))`. The four call sites split into two functions:
- One Unix `exec` block + one non-Unix `spawn` block in the resume/continue launcher (around lines 991 and 1011)
- One Unix `exec` block + one non-Unix `spawn` block in `execute_claude_command` (around lines 1050 and 1064)

Use the Edit tool with `replace_all: true` on the exact string `Command::new("claude")` if no other matches exist in this file (confirm with `grep -c 'Command::new("claude")' src/interactive/interactive.rs` — expect `4`).

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/interactive/interactive.rs
git commit -m "fix(interactive): resolve claude binary via platform helper"
```

---

### Task 6: Route `codex` launches through the resolver

**Files:**
- Modify: `src/codex/commands.rs:231`
- Modify: `src/interactive/codex_interactive.rs:566, 575`

- [ ] **Step 1: Update `src/codex/commands.rs`**

Add the import near the top:

```rust
use crate::platform::resolve_npm_cli;
```

Then replace the single occurrence of `Command::new("codex")` (in `launch_codex`, around line 231) with `Command::new(resolve_npm_cli("codex"))`.

- [ ] **Step 2: Update `src/interactive/codex_interactive.rs`**

Add the same import. Replace both occurrences of `Command::new("codex")` (around lines 566 and 575 — Unix `exec` branch and non-Unix `spawn` branch) with `Command::new(resolve_npm_cli("codex"))`.

Verify count:

```bash
grep -c 'Command::new("codex")' src/interactive/codex_interactive.rs src/codex/commands.rs
```

Expected: each file's pre-edit count matches what was edited; post-edit `grep` returns 0 matches.

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add src/codex/commands.rs src/interactive/codex_interactive.rs
git commit -m "fix(codex): resolve codex binary via platform helper"
```

---

### Task 7: Switch the interactive UI's Unicode detection to the platform helper

**Files:**
- Modify: `src/interactive/interactive.rs:73-92`

- [ ] **Step 1: Replace `detect_unicode_support`**

In `src/interactive/interactive.rs`, replace the entire `detect_unicode_support` function (the body of which currently inspects `TERM` / `LANG` and falls back to `true`) with a one-line delegation:

```rust
    /// Detect if terminal supports Unicode characters
    fn detect_unicode_support() -> bool {
        crate::platform::unicode_support_enabled()
    }
```

The function signature, visibility, and surrounding `impl BorderDrawing` block stay exactly the same. Only the body changes.

- [ ] **Step 2: Run the full test suite**

Run:

```bash
cargo test
```

Expected: green. The behavior change on non-Windows is invisible (same env-var heuristic), and on Windows the new default is ASCII-unless-Windows-Terminal, which is what we want.

- [ ] **Step 3: Commit**

```bash
git add src/interactive/interactive.rs
git commit -m "fix(interactive): delegate Unicode detection to platform helper for Windows safety"
```

---

### Task 8: Fix the broken Homebrew formula test command

**Files:**
- Modify: `.github/workflows/formula-template.rb:31`

- [ ] **Step 1: Verify the bug**

Run:

```bash
cargo run --quiet -- version 2>&1 | head -3
cargo run --quiet -- --version 2>&1 | head -3
```

Expected:
- `version` subcommand → `error: unrecognized subcommand 'version'`
- `--version` flag → `cc-switch 0.1.17`

This confirms the formula's `cc-switch version` assertion has always been broken; `brew test` fails today on every platform.

- [ ] **Step 2: Apply the fix**

In `.github/workflows/formula-template.rb`, replace line 31:

```ruby
    assert_match version.to_s, shell_output("#{bin}/cc-switch version")
```

with:

```ruby
    assert_match version.to_s, shell_output("#{bin}/cc-switch --version")
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/formula-template.rb
git commit -m "fix(brew): use --version flag in formula test (version subcommand does not exist)"
```

The next `update-brew-formula.yml` run (triggered by the next release) will copy the corrected template into the tap repo automatically. No manual tap edit needed.

---

### Task 9: Run the full quality gate locally

**Files:** none

- [ ] **Step 1: Format check**

Run:

```bash
cargo fmt --all -- --check
```

Expected: no output (clean). If it fails, run `cargo fmt --all` and re-stage the affected files into a new commit `style: cargo fmt`.

- [ ] **Step 2: Clippy**

Run:

```bash
cargo clippy -- -D warnings
```

Expected: `Finished` with no warnings.

- [ ] **Step 3: Full test suite**

Run:

```bash
cargo test
```

Expected: all tests pass (library + integration). Total test count should be the pre-existing total + 3 from `platform_tests.rs`.

- [ ] **Step 4: Confirm Phase 1 acceptance**

The Rust + Ruby changes are now self-contained. Verify by listing recent commits:

```bash
git log --oneline -10
```

You should see 7 commits (Tasks 1, 3, 4, 5, 6, 7, 8 — Task 2 was a failing test that became part of Task 3's commit).

---

## Phase 2 — CI Pipeline

### Task 10: Modernize the `test` job and add `windows-latest`

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace the `test` job**

Replace the entire `test:` job in `.github/workflows/ci.yml` (currently lines 14-49) with the following. This swaps `dtolnay/rust-toolchain@stable` + `toolchain: nightly` for an explicit stable pin (no nightly features are used), swaps the hand-rolled cache for `Swatinem/rust-cache@v2`, and adds `windows-latest`:

```yaml
  test:
    name: Test on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
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

`fail-fast: false` ensures a Windows-only failure does not cancel the macOS/Linux jobs, which is useful while bedding-in Windows support.

- [ ] **Step 2: Verify the YAML parses**

Run:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo OK
```

Expected: `OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add windows-latest to test matrix, use Swatinem cache and stable toolchain"
```

---

### Task 11: Add ARM64 Windows to `cross-build`, bump cache action

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace the `cross-build` job**

Locate the `cross-build:` job by name (line numbers will have shifted from Task 10's edits). Replace it in its entirety with:

```yaml
  cross-build:
    name: Cross-build for ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
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
            os: windows-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
          targets: ${{ matrix.target }}

      - name: Cache cargo + target
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Add Linux cross dependencies
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
          echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc" >> $GITHUB_ENV

      - name: Cross-build
        run: cargo build --release --target ${{ matrix.target }}
```

Notes:
- `aarch64-pc-windows-msvc` is cross-compiled on an x86_64 Windows runner — Rust supports this out-of-the-box because the MSVC linker handles ARM64. **Do not** try to `cargo test` on this target; the binary cannot run on the host.
- The `Swatinem/rust-cache@v2` `key:` parameter scopes the cache per target so cross-builds don't poison each other's `target/` directories.
- The CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER env-var setup was previously a separate step; folded into the same step here since both are conditional on the same target.

- [ ] **Step 2: Bump any remaining `actions/cache@v3` references**

Run:

```bash
grep -n 'actions/cache@v3' .github/workflows/ci.yml
```

Expected: no matches (Swatinem/rust-cache replaced them all). If matches appear, edit each line to `actions/cache@v4`.

- [ ] **Step 3: Validate YAML**

Run:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo OK
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add aarch64-pc-windows-msvc cross-build and per-target cache keys"
```

---

### Task 12: Add the `shell-smoke` job

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Append the smoke job**

Add the following job to `.github/workflows/ci.yml`, immediately after the `cross-build` job and before `security-audit`:

```yaml
  shell-smoke:
    name: Shell smoke (${{ matrix.shell }})
    runs-on: windows-latest
    needs: test
    strategy:
      fail-fast: false
      matrix:
        shell: [pwsh, bash]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable

      - name: Cache cargo + target
        uses: Swatinem/rust-cache@v2
        with:
          key: shell-smoke

      - name: Build release binary
        run: cargo build --release

      - name: Verify PowerShell completion loads
        if: matrix.shell == 'pwsh'
        shell: pwsh
        run: |
          $script = & ./target/release/cc-switch.exe completion powershell | Out-String
          Invoke-Expression $script

      - name: Verify Bash completion loads
        if: matrix.shell == 'bash'
        shell: bash
        run: |
          ./target/release/cc-switch.exe completion bash > /tmp/cc.bash
          bash -c "source /tmp/cc.bash"

      - name: CRUD round-trip
        shell: ${{ matrix.shell }}
        run: |
          ./target/release/cc-switch.exe add ci-test -t sk-test -u https://example.com
          ./target/release/cc-switch.exe list
          ./target/release/cc-switch.exe remove ci-test
```

- [ ] **Step 2: Validate YAML**

Run:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo OK
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add shell-smoke job for Windows pwsh and bash"
```

---

## Phase 3 — Release Workflow

### Task 13: Add Windows targets to release build matrix

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Re-read the build matrix**

Run:

```bash
awk '/^  build:/,/^  release:/' .github/workflows/release.yml
```

(Anchoring on the job header so line numbers don't matter.) Confirm the existing four `include:` entries (Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64) and the Linux dependency / cross-compile env steps are present and unchanged.

- [ ] **Step 2: Append Windows entries to the matrix**

In `.github/workflows/release.yml`, add the following two entries to the `matrix.include:` list right after the existing four:

```yaml
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            artifact_name: cc-switch-x86_64-pc-windows-msvc.zip
            binary_name: cc-switch.exe
          - target: aarch64-pc-windows-msvc
            os: windows-latest
            artifact_name: cc-switch-aarch64-pc-windows-msvc.zip
            binary_name: cc-switch.exe
```

Note the `binary_name` field — the existing Linux/macOS entries don't have it; add it to those entries too for consistency:

```yaml
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
```

- [ ] **Step 3: Validate YAML**

Run:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo OK
```

---

### Task 14: Add Windows packaging step and switch packaging to be matrix-aware

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Replace the `Package artifacts` step**

The current single packaging step (`release.yml:69-74`) only handles tar.gz. Replace it with two mutually-exclusive steps gated on `matrix.os`:

```yaml
    - name: Package artifacts (Unix)
      if: matrix.os != 'windows-latest'
      shell: bash
      run: |
        mkdir -p dist/${{ matrix.target }}
        cp target/${{ matrix.target }}/release/${{ matrix.binary_name }} dist/${{ matrix.target }}/${{ matrix.binary_name }}
        cd dist/${{ matrix.target }}
        tar -czf ../../${{ matrix.artifact_name }} ${{ matrix.binary_name }}

    - name: Package artifacts (Windows)
      if: matrix.os == 'windows-latest'
      shell: pwsh
      run: |
        $stage = "dist/${{ matrix.target }}"
        New-Item -ItemType Directory -Force -Path $stage | Out-Null
        Copy-Item "target/${{ matrix.target }}/release/${{ matrix.binary_name }}" "$stage/${{ matrix.binary_name }}"
        Compress-Archive -Path "$stage/${{ matrix.binary_name }}" -DestinationPath "${{ matrix.artifact_name }}" -Force
```

- [ ] **Step 2: Add SHA256 sidecar generation**

Immediately after the packaging steps, add:

```yaml
    - name: Generate SHA256 sidecar (Unix)
      if: matrix.os != 'windows-latest'
      shell: bash
      run: |
        shasum -a 256 ${{ matrix.artifact_name }} | awk '{print $1}' > ${{ matrix.artifact_name }}.sha256

    - name: Generate SHA256 sidecar (Windows)
      if: matrix.os == 'windows-latest'
      shell: pwsh
      run: |
        $hash = (Get-FileHash -Algorithm SHA256 "${{ matrix.artifact_name }}").Hash.ToLower()
        Set-Content -Path "${{ matrix.artifact_name }}.sha256" -Value $hash -NoNewline
```

`shasum -a 256` is on every Linux and macOS runner. The sidecar is written without a trailing newline so Scoop's `$url.sha256` autoupdate hash lookup gets a clean value.

- [ ] **Step 3: Update the `Upload artifacts` step to include the sidecar**

Locate the existing `Upload artifacts` step and update its `path:` to include both the archive and the sidecar:

```yaml
    - name: Upload artifacts
      uses: actions/upload-artifact@v4
      with:
        name: ${{ matrix.artifact_name }}
        path: |
          ${{ matrix.artifact_name }}
          ${{ matrix.artifact_name }}.sha256
```

- [ ] **Step 4: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo OK
```

- [ ] **Step 5: Commit Tasks 13 + 14 together**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): add Windows artifacts and SHA256 sidecars"
```

---

### Task 15: Fix the release upload glob and summary loop

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Update the `Create Release` files glob**

The current `softprops/action-gh-release@v1` step uses:

```yaml
        files: |
          artifacts/**/*.tar.gz
```

Change it to:

```yaml
        files: |
          artifacts/**/*.tar.gz
          artifacts/**/*.zip
          artifacts/**/*.sha256
```

Without `*.zip`, the Windows binaries are built but silently dropped from the GitHub release — the workflow still reports success. Without `*.sha256`, the Scoop `autoupdate.hash.url` lookup will 404.

- [ ] **Step 2: Update the release summary loop**

The current summary loop (`release.yml:118-124`) is:

```bash
        for artifact in artifacts/**/*.tar.gz; do
          if [ -f "$artifact" ]; then
            filename=$(basename "$artifact")
            target_name=$(echo "$filename" | sed -E 's/\.tar\.gz$//')
            echo "| $target_name | $filename |" >> $GITHUB_STEP_SUMMARY
          fi
        done
```

Replace with:

```bash
        shopt -s nullglob globstar
        for artifact in artifacts/**/*.tar.gz artifacts/**/*.zip; do
          if [ -f "$artifact" ]; then
            filename=$(basename "$artifact")
            target_name=$(echo "$filename" | sed -E 's/\.(tar\.gz|zip)$//')
            echo "| $target_name | $filename |" >> $GITHUB_STEP_SUMMARY
          fi
        done
```

`shopt -s nullglob globstar` is needed because `**` is not enabled by default in the GitHub Actions bash shell, and `nullglob` prevents the unmatched literal pattern from being echoed when one of the globs returns zero files.

- [ ] **Step 3: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo OK
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): include .zip and .sha256 in release uploads and summary"
```

---

## Phase 4 — Package Manager Distribution

### Task 16: Create the Scoop manifest template

**Files:**
- Create: `.github/workflows/scoop-manifest-template.json`

- [ ] **Step 1: Write the template**

Create `.github/workflows/scoop-manifest-template.json` with placeholder tokens (mirrors the pattern used by `formula-template.rb`):

```json
{
    "version": "__VERSION__",
    "description": "A CLI tool for managing multiple Claude API configurations and automatically switching between them",
    "homepage": "https://github.com/Linuxdazhao/cc_auto_switch",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v__VERSION__/cc-switch-x86_64-pc-windows-msvc.zip",
            "hash": "__X64_SHA__",
            "bin": "cc-switch.exe"
        },
        "arm64": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v__VERSION__/cc-switch-aarch64-pc-windows-msvc.zip",
            "hash": "__ARM64_SHA__",
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

- [ ] **Step 2: Validate JSON**

Run:

```bash
python3 -c "import json; json.load(open('.github/workflows/scoop-manifest-template.json'))" && echo OK
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/scoop-manifest-template.json
git commit -m "ci(scoop): add manifest template for cc-switch"
```

---

### Task 17: Create the Scoop manifest update workflow

**Files:**
- Create: `.github/workflows/update-scoop-manifest.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/update-scoop-manifest.yml`. Structure mirrors `update-brew-formula.yml` so anyone debugging one can debug the other:

```yaml
name: Update Scoop Manifest

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to update (e.g., 0.1.18)'
        required: true
        type: string

env:
  CARGO_TERM_COLOR: always

permissions:
  contents: write

jobs:
  update-manifest:
    name: Update Scoop Manifest
    runs-on: ubuntu-latest
    if: github.event_name == 'release' || github.event_name == 'workflow_dispatch'

    steps:
      - name: Checkout main repository
        uses: actions/checkout@v4

      - name: Extract version from tag
        id: version
        run: |
          if [ "${{ github.event_name }}" = "release" ]; then
            VERSION="${{ github.event.release.tag_name }}"
            VERSION="${VERSION#v}"
          else
            VERSION="${{ github.event.inputs.version }}"
          fi
          echo "version=$VERSION" >> $GITHUB_OUTPUT
          echo "Extracted version: $VERSION"

      - name: Wait for Windows release assets
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          BASE_URL="https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v${VERSION}"
          FILES=(
            "cc-switch-x86_64-pc-windows-msvc.zip"
            "cc-switch-aarch64-pc-windows-msvc.zip"
          )
          MAX_ATTEMPTS=60
          SLEEP_TIME=10
          for attempt in $(seq 1 $MAX_ATTEMPTS); do
            echo "Attempt $attempt/$MAX_ATTEMPTS..."
            all_available=true
            for file in "${FILES[@]}"; do
              if ! curl -sSL --fail --head "${BASE_URL}/${file}" > /dev/null 2>&1; then
                echo "  not yet: $file"
                all_available=false
                break
              fi
            done
            if [ "$all_available" = true ]; then
              echo "All Windows assets are live."
              break
            fi
            if [ $attempt -eq $MAX_ATTEMPTS ]; then
              echo "ERROR: Windows assets did not become available within timeout."
              exit 1
            fi
            sleep $SLEEP_TIME
          done

      - name: Download and calculate checksums
        id: checksums
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          BASE_URL="https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v${VERSION}"

          calculate_sha() {
            local filename=$1
            local max_retries=3
            local retry=0
            while [ $retry -lt $max_retries ]; do
              if curl -sSL --fail --max-time 60 \
                     --retry 3 --retry-delay 5 --retry-max-time 300 \
                     "${BASE_URL}/${filename}" -o "${filename}"; then
                if [ -s "${filename}" ]; then
                  local sha=$(sha256sum "${filename}" | cut -d' ' -f1)
                  echo "$sha"
                  rm -f "${filename}"
                  return 0
                fi
              fi
              retry=$((retry + 1))
              sleep 10
            done
            return 1
          }

          X64_SHA=$(calculate_sha "cc-switch-x86_64-pc-windows-msvc.zip") || exit 1
          ARM64_SHA=$(calculate_sha "cc-switch-aarch64-pc-windows-msvc.zip") || exit 1

          echo "x64_sha=$X64_SHA" >> $GITHUB_OUTPUT
          echo "arm64_sha=$ARM64_SHA" >> $GITHUB_OUTPUT
          echo "=== Computed SHA256 ==="
          echo "x86_64: $X64_SHA"
          echo "arm64:  $ARM64_SHA"

      - name: Checkout Scoop bucket repository
        uses: actions/checkout@v4
        with:
          repository: Linuxdazhao/scoop-cc-switch
          token: ${{ secrets.SCOOP_BUCKET_TOKEN || secrets.GITHUB_TOKEN }}
          path: scoop-cc-switch

      - name: Render manifest
        run: |
          cd scoop-cc-switch
          mkdir -p bucket
          cp ../.github/workflows/scoop-manifest-template.json bucket/cc-switch.json
          sed -i "s/__VERSION__/${{ steps.version.outputs.version }}/g" bucket/cc-switch.json
          sed -i "s/__X64_SHA__/${{ steps.checksums.outputs.x64_sha }}/g" bucket/cc-switch.json
          sed -i "s/__ARM64_SHA__/${{ steps.checksums.outputs.arm64_sha }}/g" bucket/cc-switch.json
          echo "=== Rendered manifest ==="
          cat bucket/cc-switch.json

      - name: Commit and push
        run: |
          cd scoop-cc-switch
          git config --global user.name "github-actions[bot]"
          git config --global user.email "github-actions[bot]@users.noreply.github.com"
          git add bucket/cc-switch.json
          if git diff --staged --quiet; then
            echo "Manifest unchanged; nothing to commit."
            exit 0
          fi
          git commit -m "Update cc-switch to v${{ steps.version.outputs.version }}

          - Update version to ${{ steps.version.outputs.version }}
          - Update SHA256 checksums for x64 and arm64

          🤖 Auto-updated by GitHub Actions"
          git push origin main
```

- [ ] **Step 2: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/update-scoop-manifest.yml'))" && echo OK
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/update-scoop-manifest.yml
git commit -m "ci(scoop): add post-release workflow to update bucket manifest"
```

---

### Task 18: Bootstrap the external `Linuxdazhao/scoop-cc-switch` repository

**Files:** none in this repo — this task instructs the operator to perform an external action.

- [ ] **Step 1: Create the GitHub repository**

In a browser (or via `gh repo create`), create a new public repository at `https://github.com/Linuxdazhao/scoop-cc-switch`. It should be empty (no README, no .gitignore) so the first push from this checkout sets up the initial commit cleanly. If you prefer to initialize via `gh`:

```bash
gh repo create Linuxdazhao/scoop-cc-switch --public --description "Scoop bucket for cc-switch"
```

- [ ] **Step 2: Clone locally and seed the initial manifest**

```bash
cd /tmp
git clone https://github.com/Linuxdazhao/scoop-cc-switch.git
cd scoop-cc-switch
mkdir -p bucket
```

Write `bucket/cc-switch.json` by hand for the bootstrap commit (it will be auto-overwritten by the next release). Use **the current released version** so users can `scoop install cc-switch` immediately rather than wait for the next release:

```bash
cat > bucket/cc-switch.json <<'EOF'
{
    "version": "0.1.17",
    "description": "A CLI tool for managing multiple Claude API configurations and automatically switching between them",
    "homepage": "https://github.com/Linuxdazhao/cc_auto_switch",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v0.1.17/cc-switch-x86_64-pc-windows-msvc.zip",
            "hash": "PLACEHOLDER_WILL_BE_REPLACED_ON_NEXT_RELEASE",
            "bin": "cc-switch.exe"
        },
        "arm64": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v0.1.17/cc-switch-aarch64-pc-windows-msvc.zip",
            "hash": "PLACEHOLDER_WILL_BE_REPLACED_ON_NEXT_RELEASE",
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
EOF
```

Important: the v0.1.17 release does **not** contain Windows artifacts yet (those start with the next release after this plan lands). Document this in the README of the bucket repo so users don't try `scoop install cc-switch` until v0.1.18+ ships.

Write a minimal `README.md` for the bucket repo:

```bash
cat > README.md <<'EOF'
# scoop-cc-switch

Scoop bucket for [cc-switch](https://github.com/Linuxdazhao/cc_auto_switch).

## Usage

```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

Note: Windows builds are available from v0.1.18 onward.

This bucket is maintained automatically by the
[`update-scoop-manifest.yml`](https://github.com/Linuxdazhao/cc_auto_switch/blob/main/.github/workflows/update-scoop-manifest.yml)
workflow in the main repo.
EOF
```

- [ ] **Step 3: Commit and push**

```bash
git add bucket/cc-switch.json README.md
git commit -m "Initial Scoop bucket for cc-switch v0.1.17"
git push origin main
```

- [ ] **Step 4: (Optional) Configure a dedicated push token**

If `GITHUB_TOKEN` from `cc_auto_switch` cannot push to `scoop-cc-switch` (different owner permissions or branch protection), create a PAT with `repo:write` scope, then add it as repo secret `SCOOP_BUCKET_TOKEN` on `cc_auto_switch`. The `update-scoop-manifest.yml` workflow already prefers `SCOOP_BUCKET_TOKEN` if present.

This is the only step in the plan that requires interactive web-UI work; everything else is scriptable.

---

## Phase 5 — Documentation

### Task 19: Add per-platform installation matrix to the README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Read the current README structure**

Run:

```bash
sed -n '1,60p' README.md
```

Identify the existing "Installation" section (or, if none, the natural insertion point — typically after the project description and before "Usage").

- [ ] **Step 2: Insert the cross-platform installation block**

Replace the current installation section (or add a new one) with:

````markdown
## Installation

### Windows

**Option 1 — Scoop (recommended, v0.1.18+):**

```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

**Option 2 — Cargo:**

```powershell
cargo install cc-switch
```

**Option 3 — Pre-built binary:** download the `.zip` for your architecture from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) and place `cc-switch.exe` on your PATH.

### Linux

**Option 1 — Homebrew (recommended):**

```bash
# Short form works because the tap repo is named `homebrew-cc-switch`
brew install Linuxdazhao/cc-switch/cc-switch
# Or explicitly:
# brew tap Linuxdazhao/cc-switch && brew install cc-switch
```

**Option 2 — Cargo:**

```bash
cargo install cc-switch
```

**Option 3 — Pre-built binary:** download the `.tar.gz` for your architecture from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases).

### macOS

**Option 1 — Homebrew (recommended):**

```bash
brew install Linuxdazhao/cc-switch/cc-switch
```

**Option 2 — Cargo:**

```bash
cargo install cc-switch
```

**Option 3 — Pre-built binary:** download the `.tar.gz` for your architecture from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases).

## Shell Completion

### PowerShell (Windows)

Do **not** redirect completion directly to `$PROFILE` — that overwrites any
existing aliases, modules, or theme setup. Write to a dedicated file and
dot-source it from `$PROFILE`:

```powershell
$completionDir = Split-Path -Parent $PROFILE
if (-not (Test-Path $completionDir)) { New-Item -ItemType Directory -Path $completionDir | Out-Null }
$completionPath = Join-Path $completionDir 'cc-switch.completion.ps1'
cc-switch completion powershell | Out-File -Encoding utf8 $completionPath

$line = ". '$completionPath'"
if (-not ((Test-Path $PROFILE) -and (Select-String -Path $PROFILE -Pattern ([regex]::Escape($line)) -Quiet))) {
    Add-Content -Path $PROFILE -Value $line
}
```

This is idempotent — safe to run multiple times.

### Git Bash (Windows) / Bash (Linux, macOS)

```bash
cc-switch completion bash >> ~/.bashrc
source ~/.bashrc
```

### Fish, Zsh

```bash
# Fish
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh
cc-switch completion zsh > ~/.zsh/completions/_cc-switch
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
```

### CMD (Windows)

CMD has no completion mechanism; use the CLI directly:

```cmd
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
cc-switch list
```
````

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add per-platform installation and shell completion matrix"
```

---

### Task 20: Document Windows resolver behavior in `CLAUDE.md`

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Append a new subsection**

Find the "Important Implementation Notes" section in `CLAUDE.md`. Add a new subsection at the end of it (after the existing "Performance" subsection):

```markdown
### Cross-Platform Binary Resolution

On Windows, the `claude` and `codex` CLIs are typically installed via npm and
ship as `.cmd` shims rather than `.exe`. `std::process::Command::new("claude")`
does not always pick these up — its behavior depends on how PATHEXT is wired
in the calling shell, which is inconsistent across PowerShell / CMD / Git Bash.

To handle this uniformly, every `claude` or `codex` launch in the codebase
goes through `crate::platform::resolve_npm_cli(name)` (see `src/platform.rs`).
The resolver:

1. Honors a `<NAME_UPPERCASE>_BINARY` env-var override (`CLAUDE_BINARY`,
   `CODEX_BINARY`) — useful when users want to pin a specific installation
   or test against a sibling binary.
2. On Windows, probes `<name>.exe`, `<name>.cmd`, `<name>.ps1` via the `which`
   crate.
3. Falls back to the bare name on non-Windows or when probing finds nothing
   — identical to the pre-resolver behavior.

**When adding a new external CLI launch:** always go through the resolver.
Never write `Command::new("some_cli")` directly for a binary that might be
installed via npm.

Terminal Unicode detection lives in the same module
(`crate::platform::unicode_support_enabled`). On Windows it defaults to ASCII
unless `WT_SESSION` is set (Windows Terminal), which prevents mojibake on
legacy conhost / CMD. Users can override either way via `CC_SWITCH_ASCII=1`
or `CC_SWITCH_UNICODE=1`.
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: document cross-platform binary resolution helpers"
```

---

## Validation

### Note on a Spec Item Intentionally Not Implemented

The spec's §5.2 lists "Line endings in generated completion scripts" as a code-change item ("open with binary mode or normalize to `\n`"). **This plan does not implement it** because `clap_complete` writes completion output to stdout — the binary never touches the filesystem. The shell that invokes `cc-switch completion <shell> > some/file` is what determines the line endings, and that is shell-redirect behavior, not application behavior. The README's Bash example pipes from inside Bash (LF), and PowerShell scripts dot-sourced by PowerShell are happy with CRLF, so the practical risk is nil. If this turns out to bite a user in the wild, add a `cc-switch completion <shell> --output <path>` flag that writes binary-mode LF — but don't speculatively add it now.

### Final Local Check

- [ ] Run the full quality gate:

```bash
cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo test
```

Expected: all green. Existing test count + 3 new tests from `platform_tests.rs`.

- [ ] Inspect the commit graph:

```bash
git log --oneline main..HEAD
```

Expected commits (order may vary slightly based on execution):

```
docs: document cross-platform binary resolution helpers
docs: add per-platform installation and shell completion matrix
ci(scoop): add post-release workflow to update bucket manifest
ci(scoop): add manifest template for cc-switch
ci(release): include .zip and .sha256 in release uploads and summary
ci(release): add Windows artifacts and SHA256 sidecars
ci: add shell-smoke job for Windows pwsh and bash
ci: add aarch64-pc-windows-msvc cross-build and per-target cache keys
ci: add windows-latest to test matrix, use Swatinem cache and stable toolchain
fix(brew): use --version flag in formula test (version subcommand does not exist)
fix(interactive): delegate Unicode detection to platform helper for Windows safety
fix(codex): resolve codex binary via platform helper
fix(interactive): resolve claude binary via platform helper
fix(utils): resolve claude binary via platform helper
feat(platform): add resolve_npm_cli and unicode_support_enabled helpers
chore: add which crate for cross-platform PATH lookup
```

(Plus Task 18, which is performed in an external repo and does not appear in this repo's log.)

### Post-Merge / Post-Release Verification

These cannot be automated in CI and must be exercised on real hosts after a release tag is cut:

- [ ] On Windows: run `claude` after `cc-switch use <alias>` and confirm Claude CLI launches (verifies the resolver finds `claude.cmd`).
- [ ] On Windows: run `cx <codex-alias>` and confirm Codex CLI launches.
- [ ] On Windows: confirm `~/.claude/cc_auto_switch_setting.json` resolves to `C:\Users\<user>\.claude\cc_auto_switch_setting.json` and is read/written correctly by `cc-switch add` + `list`.
- [ ] On Windows: confirm `~/.codex/auth.json` resolves to `C:\Users\<user>\.codex\auth.json` and is written correctly by `cx <codex-alias>`.
- [ ] On Windows: drop a file at the legacy `C:\Users\<user>\.cc-switch\configurations.json` path, then run `cc-switch list` — the auto-migration in `src/config/config_storage.rs:64-71` should pick it up and move it to the new path.
- [ ] On Windows Terminal: open `cc-switch` (interactive mode) and confirm Unicode borders render correctly.
- [ ] On legacy conhost / CMD: open `cc-switch` and confirm ASCII fallback (no mojibake). Then set `CC_SWITCH_UNICODE=1` and confirm Unicode renders (escape-hatch verification).
- [ ] After the first release with Windows artifacts: `scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch && scoop install cc-switch` succeeds on a fresh Windows VM.
- [ ] After the same release: `brew install Linuxdazhao/cc-switch/cc-switch` on Linux still succeeds (regression guard for the already-working Linuxbrew path).
- [ ] After the same release: `brew test cc-switch` succeeds on both macOS and Linux (validates the Task 8 formula fix).
- [ ] After the same release: visit the release page on GitHub and confirm both `.zip` and `.sha256` files are present for both Windows targets (catches Task 15 regressions).
- [ ] `cargo install cc-switch` from crates.io on a fresh Windows VM compiles and launches.
