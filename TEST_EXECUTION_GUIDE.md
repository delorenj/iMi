# iMi Test Execution Guide

This guide provides practical instructions for running tests, setting up the testing environment, and interpreting results for the iMi Rust CLI application.

## Quick Start

```bash
# Clone the repository and enter the directory
cd iMi

# Install dependencies (after creating Cargo.toml from template)
cargo build

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_worktree_creation
```

## Test Categories and Commands

### 1. Unit Tests
```bash
# Run all unit tests
cargo test --lib

# Run unit tests for specific module
cargo test --lib git::operations

# Run unit tests with coverage
cargo tarpaulin --lib --out Html
```

### 2. Integration Tests
```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test suite
cargo test --test integration_tests
cargo test --test workflow_tests
cargo test --test database_tests

# Run integration tests with environment setup
RUST_LOG=debug cargo test --test integration_tests -- --nocapture
```

### 3. CLI Tests
```bash
# Run CLI-specific tests
cargo test --test cli_tests

# Run CLI tests with temporary directories
cargo test --test cli_tests -- --test-threads=1

# Test CLI help output
cargo test test_help_output -- --nocapture
```

### 4. Performance Tests
```bash
# Run performance benchmarks
cargo bench

# Run specific benchmark
cargo bench worktree_operations

# Generate HTML benchmark reports
cargo bench -- --output-format html

# Run performance regression tests
cargo test --test performance_tests --release
```

### 5. Property-Based Tests
```bash
# Run property-based tests
cargo test --features proptest-impl

# Run proptest with more cases
PROPTEST_CASES=10000 cargo test --features proptest-impl
```

## Environment Setup

### Prerequisites
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install required tools
cargo install cargo-tarpaulin  # For coverage
cargo install cargo-audit      # For security auditing
cargo install cargo-watch      # For continuous testing

# Install Git (required for testing)
# Ubuntu/Debian:
sudo apt-get install git

# macOS:
brew install git

# Windows:
# Download and install from https://git-scm.com/
```

### Test Database Setup
```bash
# Create test database directory
mkdir -p tests/fixtures/db

# Run database migrations for testing
sqlx database create --database-url sqlite://tests/fixtures/db/test.db
sqlx migrate run --database-url sqlite://tests/fixtures/db/test.db
```

### Test Repository Setup
```bash
# Create test fixture repositories
mkdir -p tests/fixtures/repositories
cd tests/fixtures/repositories

# Create a simple test repository
git init simple_repo
cd simple_repo
echo "# Test Repository" > README.md
git add README.md
git commit -m "Initial commit"
cd ..

# Create compressed fixture
tar -czf simple_repo.tar.gz simple_repo/
rm -rf simple_repo/
```

## Continuous Testing

### Watch Mode
```bash
# Run tests automatically on file changes
cargo watch -x test

# Run specific tests on changes
cargo watch -x "test --test integration_tests"

# Run with coverage on changes
cargo watch -x "tarpaulin --out Html"
```

### Pre-commit Testing
```bash
# Create a pre-commit hook script
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit tests..."

# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Unit tests
cargo test --lib

# Quick integration tests
cargo test --test cli_tests

echo "Pre-commit tests passed!"
EOF

chmod +x .git/hooks/pre-commit
```

## Test Data and Fixtures

### Managing Test Data
```bash
# Clean up test data
cargo test --test cleanup_tests

# Reset test databases
rm -rf tests/fixtures/db/*.db
sqlx migrate run --database-url sqlite://tests/fixtures/db/test.db

# Regenerate test repositories
./scripts/setup_test_fixtures.sh
```

### Creating Test Fixtures
```bash
# Script: scripts/setup_test_fixtures.sh
#!/bin/bash
set -e

FIXTURES_DIR="tests/fixtures"
REPOS_DIR="$FIXTURES_DIR/repositories"

mkdir -p "$REPOS_DIR"
cd "$REPOS_DIR"

# Create various repository types for testing
create_repo() {
    local name=$1
    local type=$2
    
    echo "Creating $type repository: $name"
    
    git init "$name"
    cd "$name"
    
    case $type in
        "simple")
            echo "# $name" > README.md
            git add README.md
            git commit -m "Initial commit"
            ;;
        "with_branches")
            echo "# $name" > README.md
            git add README.md
            git commit -m "Initial commit"
            git checkout -b feature/test
            echo "Test feature" > feature.txt
            git add feature.txt
            git commit -m "Add test feature"
            git checkout main
            ;;
        "with_worktrees")
            echo "# $name" > README.md
            git add README.md
            git commit -m "Initial commit"
            git worktree add ../feat-test feature/test
            ;;
    esac
    
    cd ..
    tar -czf "$name.tar.gz" "$name/"
    rm -rf "$name/"
}

create_repo "simple_repo" "simple"
create_repo "complex_repo" "with_branches"
create_repo "worktree_repo" "with_worktrees"

echo "Test fixtures created successfully"
```

## Test Debugging

### Debug Specific Tests
```bash
# Run test with debug output
RUST_LOG=debug cargo test test_worktree_creation -- --nocapture

# Run test with backtraces
RUST_BACKTRACE=1 cargo test test_error_handling

# Run test with full backtraces
RUST_BACKTRACE=full cargo test test_complex_scenario
```

### Test Isolation
```bash
# Run tests sequentially (for debugging race conditions)
cargo test -- --test-threads=1

# Run specific test in isolation
cargo test test_concurrent_operations -- --exact --nocapture

# Run tests with specific filter
cargo test worktree -- --nocapture
```

### Memory and Performance Debugging
```bash
# Run tests with memory profiling (using valgrind on Linux)
valgrind --tool=memcheck cargo test

# Profile test performance
cargo test --release -- --nocapture

# Check for memory leaks in async tests
cargo test --features=testing-utils -- --test-threads=1
```

## Coverage Analysis

### Generate Coverage Reports
```bash
# HTML coverage report
cargo tarpaulin --out Html

# XML coverage report (for CI)
cargo tarpaulin --out Xml

# Multiple formats
cargo tarpaulin --out Html --out Xml --out Lcov

# Coverage with exclusions
cargo tarpaulin --out Html --exclude-files "tests/*" --exclude-files "benches/*"
```

### Coverage Thresholds
```bash
# Fail if coverage below threshold
cargo tarpaulin --fail-under 80

# Coverage for specific packages
cargo tarpaulin --packages iMi

# Branch coverage
cargo tarpaulin --branch
```

## Cross-Platform Testing

### Local Cross-Platform Testing
```bash
# Test on different targets (if available)
cargo test --target x86_64-unknown-linux-gnu
cargo test --target x86_64-pc-windows-msvc
cargo test --target x86_64-apple-darwin

# Test with different Rust versions
rustup toolchain install beta
cargo +beta test

rustup toolchain install nightly
cargo +nightly test
```

### Platform-Specific Tests
```bash
# Run only Linux tests
cargo test --test linux_tests

# Run only Windows tests
cargo test --test windows_tests

# Run only macOS tests
cargo test --test macos_tests
```

## CI/CD Integration

### GitHub Actions Commands
```bash
# Simulate CI environment locally
export CI=true
export GITHUB_ACTIONS=true

cargo test --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Docker Testing
```dockerfile
# Dockerfile.test
FROM rust:1.70

WORKDIR /app
COPY . .

RUN cargo test --release
```

```bash
# Build and run test container
docker build -f Dockerfile.test -t iMi-tests .
docker run --rm iMi-tests
```

## Performance Testing

### Benchmark Execution
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- worktree_creation

# Save benchmark results
cargo bench -- --save-baseline main

# Compare with baseline
cargo bench -- --baseline main

# Generate flame graphs (with flamegraph installed)
cargo flamegraph --bench worktree_operations
```

### Load Testing
```bash
# Run load tests
cargo test --test load_tests --release -- --ignored

# Stress testing with multiple processes
for i in {1..10}; do
    cargo test --test stress_tests --release &
done
wait
```

## Troubleshooting

### Common Issues

#### Test Failures Due to Git Configuration
```bash
# Set git config for tests
git config --global user.name "Test User"
git config --global user.email "test@example.com"
git config --global init.defaultBranch main
```

#### Permission Issues on Windows
```bash
# Run as administrator or adjust test for Windows
cargo test -- --skip test_symlink_creation
```

#### Database Lock Issues
```bash
# Run database tests sequentially
cargo test database -- --test-threads=1
```

#### Temporary Directory Cleanup
```bash
# Clean up test directories
find /tmp -name "iMi_test*" -type d -exec rm -rf {} + 2>/dev/null || true

# On Windows
for /d %i in (%TEMP%\iMi_test*) do rmdir /s /q "%i"
```

### Debug Environment
```bash
# Create debug environment
export RUST_LOG=iMi=debug,sqlx=debug
export RUST_BACKTRACE=1
export IMI_TEST_MODE=1

cargo test -- --nocapture
```

## Test Reporting

### Generate Test Reports
```bash
# Install cargo-nextest for better test output
cargo install cargo-nextest

# Run tests with better reporting
cargo nextest run

# Generate JUnit reports
cargo nextest run --profile ci
```

### Custom Test Output
```bash
# Install test result processors
cargo install cargo-test-results

# Generate detailed reports
cargo test-results --format json > test_results.json
```

## Maintenance

### Regular Test Maintenance
```bash
# Update test dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Security audit
cargo audit

# Check for unused dependencies
cargo machete
```

### Test Performance Monitoring
```bash
# Track test execution times
cargo test -- --report-time

# Monitor test flakiness
for i in {1..10}; do
    echo "Run $i:"
    cargo test test_concurrent_operations || echo "FAILED on run $i"
done
```

This guide provides a comprehensive reference for executing and managing tests throughout the development lifecycle of the iMi application.