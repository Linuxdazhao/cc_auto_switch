# Cross-Platform Support Design Spec

**Date:** 2026-05-17
**Topic:** Add full Windows and Linux support to cc-switch
**Status:** Approved

## Overview

This spec outlines the implementation plan for adding comprehensive cross-platform support to cc-switch. The goal is to make cc-switch a first-class citizen on Windows and Linux, with CI testing, binary releases, package manager distribution, and documentation.

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

Add `windows-latest` to the test job:

```yaml
jobs:
  test:
    name: Test on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        shell: [bash, pwsh, cmd]  # Test all three shells on Windows
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: nightly
          components: rustfmt, clippy

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests (Bash)
        if: matrix.shell == 'bash'
        shell: bash
        run: cargo test

      - name: Run tests (PowerShell)
        if: matrix.shell == 'pwsh'
        shell: pwsh
        run: cargo test

      - name: Run tests (CMD)
        if: matrix.shell == 'cmd'
        shell: cmd
        run: cargo test
```

#### 1.2 Cache Path Fix

The current cache paths use `~/.cargo/registry` which won't work on Windows. Use `actions/cache` with proper per-OS paths or use `$CARGO_HOME`.

#### 1.3 Cross-Build Matrix

Add Windows targets to the cross-build job:

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
          os: windows-latest
```

### 2. Release Workflow Changes

#### 2.1 Add Windows Builds

Extend the release matrix to include Windows binaries:

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
            os: windows-latest
            artifact_name: cc-switch-aarch64-pc-windows-msvc.zip
            binary_name: cc-switch.exe
```

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

**Scoop manifest (`bucket/cc-switch.json`):**
```json
{
    "version": "0.1.18",
    "description": "A CLI tool for managing multiple Claude API configurations and automatically switching between them",
    "homepage": "https://github.com/Linuxdazhao/cc_auto_switch",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v0.1.18/cc-switch-v0.1.18-x86_64-pc-windows-msvc.zip",
            "hash": "SHA256_HASH_HERE",
            "bin": "cc-switch.exe"
        },
        "arm64": {
            "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v0.1.18/cc-switch-v0.1.18-aarch64-pc-windows-msvc.zip",
            "hash": "SHA256_HASH_HERE",
            "bin": "cc-switch.exe"
        }
    },
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v$version/cc-switch-v$version-x86_64-pc-windows-msvc.zip"
            },
            "arm64": {
                "url": "https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v$version/cc-switch-v$version-aarch64-pc-windows-msvc.zip"
            }
        }
    }
}
```

**Installation:**
```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

#### 3.2 Linuxbrew (Linux)

The existing Homebrew formula at `Linuxdazhao/homebrew-cc-switch` should already work with Linuxbrew. Homebrew auto-detects the platform and downloads the appropriate binary.

**Verify Linux support:**
- Check that the formula uses `on_linux` blocks if needed
- Test installation on Linux: `brew install Linuxdazhao/cc-switch/cc-switch`
- Update formula to include Linux bottles in release workflow

#### 3.3 Cargo Install (All Platforms)

Already works on all platforms:
```bash
cargo install cc-switch
```

No changes needed.

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
brew install Linuxdazhao/cc-switch/cc-switch
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
```powershell
# Add to $PROFILE for permanent setup
cc-switch completion powershell | Out-File $PROFILE
```

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
- **Process execution**: `std::process::Command` works on all platforms
- **JSON serialization**: `serde_json` is platform-agnostic
- **Error handling**: `anyhow` works on all platforms

#### 5.2 Potential Issues to Verify

| Component | Concern | Mitigation |
|-----------|---------|------------|
| **Interactive UI** | Crossterm on Windows Terminal/CMD | Already tested by crossterm, but verify manually |
| **Border drawing** | Unicode support detection | ASCII fallback already exists in `interactive.rs:73-92` |
| **Shell completion** | PowerShell script correctness | Already implemented, verify it works |
| **Path separators** | Windows uses `\`, Unix uses `/` | `std::path::PathBuf` handles this automatically |

#### 5.3 Testing Checklist

- [ ] Run `cargo test` on Windows PowerShell
- [ ] Run `cargo test` on Windows CMD
- [ ] Run `cargo test` on Git Bash (Windows)
- [ ] Test interactive UI on Windows Terminal
- [ ] Test interactive UI on CMD
- [ ] Verify configuration file paths work on Windows
- [ ] Verify shell completion works in PowerShell
- [ ] Test `cc-switch add`, `list`, `use`, `remove` on Windows

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

### Phase 1: CI Pipeline (Week 1)

1. Add Windows to test matrix
2. Fix cache paths for Windows
3. Add shell-specific test steps
4. Verify all tests pass on Windows

### Phase 2: Release Workflow (Week 2)

1. Add Windows targets to release matrix
2. Update packaging scripts for Windows `.zip`
3. Test release workflow on a pre-release tag
4. Verify binaries are correctly packaged

### Phase 3: Scoop Bucket (Week 3)

1. Create `Linuxdazhao/scoop-cc-switch` repository
2. Write Scoop manifest
3. Test installation via Scoop
4. Update README with Scoop instructions

### Phase 4: Documentation (Week 3)

1. Update README with Windows/Linux installation instructions
2. Add shell setup instructions for PowerShell/CMD/Git Bash
3. Update CLAUDE.md with cross-platform notes
4. Update help text in CLI if needed

### Phase 5: Linuxbrew Verification (Week 4)

1. Test existing Homebrew formula on Linux
2. Update formula if needed for Linux support
3. Add Linux bottles to release workflow
4. Verify `brew install` works on Linux

## Success Criteria

- [ ] CI passes on Windows, Linux, and macOS
- [ ] Windows binaries (x86_64 + ARM64) available in GitHub Releases
- [ ] Linux binaries (x86_64 + aarch64) available in GitHub Releases
- [ ] Scoop installation works: `scoop install cc-switch`
- [ ] Linuxbrew installation works: `brew install Linuxdazhao/cc-switch/cc-switch`
- [ ] Interactive UI works on Windows Terminal, CMD, and PowerShell
- [ ] Shell completion works in PowerShell, Git Bash, and CMD
- [ ] README has clear installation instructions for all platforms
- [ ] All existing macOS functionality preserved

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Windows ARM64 cross-compilation fails | Cannot support Windows on ARM | Use GitHub Actions Windows ARM64 runner or skip ARM64 initially |
| Crossterm doesn't work on CMD | Interactive UI broken on CMD | Add ASCII fallback, document Git Bash as recommended |
| Scoop manifest hash mismatch after release | Installation fails | Automate hash update in release workflow |
| Linuxbrew formula doesn't work on Linux | Linux users can't install via Homebrew | Test formula on Linux before release, fallback to cargo install |

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
