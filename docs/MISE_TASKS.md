# iMi Mise Tasks Reference

This document provides a comprehensive reference for all mise tasks available in the iMi project. These tasks automate common development, build, packaging, and deployment workflows.

## Prerequisites

- [mise](https://mise.jdx.dev/) must be installed
- Rust toolchain (cargo, rustc)
- Optional: jq, cargo-deb, cargo-audit, musl toolchain

## Quick Start

```bash
# List all available tasks
mise tasks

# Run a specific task
mise run <task-name>

# Or use the shorthand
mise <task-name>

# Check task dependencies
mise task deps <task-name>
```

## Task Categories

### 🧪 Testing Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `test` | Run all tests | None |
| `test-ci` | Run all tests in CI (with output) | None |
| `test-unit` | Run unit tests only | None |
| `test-integration` | Run integration tests only | None |
| `test-cli` | Run CLI tests only | None |
| `test-watch` | Run tests in watch mode | None |

**Examples:**
```bash
# Run all tests
mise run test

# Run only unit tests
mise run test-unit

# Watch mode for TDD
mise run test-watch
```

### 🏗️ Build Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `build` | Debug build | None |
| `build-release` | Optimized release build (LTO, stripped) | None |
| `build-dev` | Development build with all features | None |
| `build-musl` | Static binary with musl (Linux x86_64) | None |
| `check` | Quick compile check without building | None |
| `clean` | Clean build artifacts | None |

**Examples:**
```bash
# Quick compile check (fast)
mise run check

# Build for release
mise run build-release

# Create static binary for deployment
mise run build-musl
```

**Cargo.toml Release Profile:**
```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
panic = "abort"      # Smaller binary
strip = true         # Strip symbols
```

### 📦 Package Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `package` | Package release binary with docs | `build-release` |
| `package-musl` | Package static musl binary | `build-musl` |
| `package-deb` | Create Debian package (.deb) | `build-release` |

**Package Contents:**
- Binary in `bin/`
- Documentation (README, LICENSE, INSTALL)
- Shell completions (bash, zsh, fish)
- Compressed tarball

**Examples:**
```bash
# Create standard release package
mise run package

# Create portable Linux package
mise run package-musl

# Create Debian package (installs cargo-deb if needed)
mise run package-deb
```

**Output Location:** `target/dist/`

### 💿 Install Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `install` | Install to system (/usr/local/bin) | `build-release` |
| `install-local` | Install to user directory (~/.local/bin) | `build-release` |
| `install-completions` | Install shell completions | `build-release` |
| `uninstall` | Remove iMi from system | None |
| `verify-install` | Verify installation works | None |

**Examples:**
```bash
# Install for current user (recommended)
mise run install-local

# Install system-wide (requires sudo)
mise run install

# Install shell completions
mise run install-completions

# Verify everything works
mise run verify-install

# Uninstall
mise run uninstall
```

**Install Locations:**
- System: `/usr/local/bin/iMi`
- User: `~/.local/bin/iMi`
- Completions:
  - Bash: `~/.local/share/bash-completion/completions/iMi`
  - Zsh: `~/.zsh/completions/_iMi`
  - Fish: `~/.config/fish/completions/iMi.fish`

### 🚀 CI/CD Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `ci` | Run all CI checks | `lint`, `test-ci` |
| `ci-prepare` | Prepare CI environment | None |
| `ci-build` | Full CI build with checks | `check`, `build-release`, `test-ci` |
| `ci-package-all` | Create all release packages | None |
| `release-dry-run` | Simulate full release process | `clean`, `ci-build`, `ci-package-all` |

**Examples:**
```bash
# Run full CI checks locally
mise run ci

# Prepare environment (install tools, update Rust)
mise run ci-prepare

# Complete release simulation
mise run release-dry-run
```

**CI Workflow:**
1. `ci-prepare` - Set up environment
2. `ci-build` - Build and test
3. `ci-package-all` - Create packages
4. `release-dry-run` - Full release simulation

### 🔧 Development Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `lint` | Run clippy linter | None |
| `format` | Format code with rustfmt | None |

**Examples:**
```bash
# Lint code
mise run lint

# Format code
mise run format

# Run both before committing
mise run format && mise run lint
```

### 🔍 Utility Tasks

| Task | Description | Dependencies |
|------|-------------|--------------|
| `size-check` | Analyze binary size | `build-release` |
| `deps-tree` | Display dependency tree | None |
| `deps-audit` | Security vulnerability audit | None |

**Examples:**
```bash
# Check binary size
mise run size-check

# View dependency tree
mise run deps-tree

# Security audit (installs cargo-audit if needed)
mise run deps-audit
```

## Common Workflows

### Development Workflow

```bash
# Start development
mise run check           # Quick validation
mise run test-watch      # Watch tests

# Before committing
mise run format
mise run lint
mise run test
```

### Release Workflow

```bash
# Full release build and validation
mise run release-dry-run

# Or step by step:
mise run clean
mise run ci-build
mise run ci-package-all
```

### Local Testing

```bash
# Build and install locally
mise run build-release
mise run install-local
mise run verify-install

# Test in terminal
iMi --version
iMi --help
```

### CI/CD Pipeline

```yaml
# Example GitHub Actions workflow
- name: Prepare
  run: mise run ci-prepare

- name: Build and Test
  run: mise run ci-build

- name: Package
  run: mise run ci-package-all

- name: Upload artifacts
  uses: actions/upload-artifact@v3
  with:
    path: target/dist/*.tar.gz
```

## Task Dependencies

Some tasks automatically run prerequisite tasks:

```bash
# This runs: clean → check → build-release → test-ci → ci-package-all
mise run release-dry-run

# This runs: build-release → package
mise run package

# This runs: lint → test-ci
mise run ci
```

View dependencies:
```bash
mise task deps release-dry-run
```

## Environment Variables

Tasks respect these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PREFIX` | Install prefix for system install | `/usr/local` |
| `MISE_JOBS` | Parallel jobs for tasks | `4` |
| `MISE_TASK_OUTPUT` | Output mode (interleave/line) | `line` |

**Examples:**
```bash
# Install to custom location
PREFIX=/opt mise run install

# Run with more parallelism
MISE_JOBS=8 mise run ci-build
```

## Troubleshooting

### Task fails with "Binary not found"
```bash
# Some tasks require building first
mise run build-release
mise run install-local
```

### Permission denied during install
```bash
# Use local install instead of system install
mise run install-local

# Or provide PREFIX
PREFIX="${HOME}/.local" mise run install
```

### Package task fails with "jq not found"
```bash
# Install jq
sudo apt-get install jq  # Debian/Ubuntu
brew install jq          # macOS
```

### Musl build fails
```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# May also need musl-tools
sudo apt-get install musl-tools
```

## Advanced Usage

### Running Multiple Tasks

```bash
# Run tasks in sequence
mise run lint && mise run test && mise run build-release

# Run tasks in parallel (if independent)
mise run check lint format
```

### Task Aliases

Add to your shell config:
```bash
alias mr='mise run'
alias mb='mise run build-release'
alias mt='mise run test'
alias mp='mise run package'
```

### Custom Task Arguments

Some tasks accept additional arguments:
```bash
# Pass args to cargo test
mise run test -- --nocapture --test-threads=1

# Build with features
mise run build-release --all-features
```

## Contributing

When adding new tasks:

1. Add to `.mise.toml` in the appropriate section
2. Include a clear description
3. Specify dependencies if needed
4. Update this documentation
5. Test the task works as expected

**Example:**
```toml
[tasks.my-task]
description = "Clear description of what this does"
depends = ["build-release"]  # Optional
run = '''
#!/usr/bin/env bash
set -euo pipefail
# Task implementation
'''
```

## Resources

- [mise Documentation](https://mise.jdx.dev/)
- [mise Task Reference](https://mise.jdx.dev/tasks/)
- [iMi Project README](../README.md)
- [Installation Guide](../INSTALL.md)
