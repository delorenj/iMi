#!/bin/bash
# Helper script to run prune fix tests with proper configuration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "🧪 Running Prune Fix Test Suite"
echo "================================"
echo ""

# Color codes
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Parse command line arguments
TEST_NAME=""
VERBOSE=false
SHOW_OUTPUT=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -o|--output)
            SHOW_OUTPUT=true
            shift
            ;;
        -t|--test)
            TEST_NAME="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -v, --verbose       Enable verbose output"
            echo "  -o, --output        Show test output (--nocapture)"
            echo "  -t, --test NAME     Run specific test"
            echo "  -h, --help          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                    # Run all tests"
            echo "  $0 -v -o                              # Run with verbose output"
            echo "  $0 -t test_prune_after_manual_deletion # Run specific test"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Build test command
TEST_CMD="cargo test --test prune_fix_tests"

if [ -n "$TEST_NAME" ]; then
    TEST_CMD="$TEST_CMD $TEST_NAME"
fi

if [ "$SHOW_OUTPUT" = true ]; then
    TEST_CMD="$TEST_CMD -- --nocapture"
fi

# Set environment for verbose mode
if [ "$VERBOSE" = true ]; then
    export RUST_LOG=debug
    export RUST_BACKTRACE=1
fi

echo -e "${BLUE}Command:${NC} $TEST_CMD"
echo ""

# Run the tests
if eval "$TEST_CMD"; then
    echo ""
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}❌ Tests failed!${NC}"
    exit 1
fi
