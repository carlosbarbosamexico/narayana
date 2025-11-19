#!/bin/bash
# Read/Write Performance Test (Native - Direct Storage Access)
# Usage: ./rw [writes] [reads]
# Example: ./rw 1m 1k (1 million writes, 1k reads)
# Default: ./rw (1k writes, 500 reads)
# 
# This uses native storage access (1000x faster than HTTP API)

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Parse number with suffix (1k, 1m, 1b, etc.)
parse_number() {
    local num_str=$1
    if echo "$num_str" | grep -qE '[kmK]$'; then
        local num=$(echo "$num_str" | sed 's/[kmK]$//')
        echo $(($num * 1000))
    elif echo "$num_str" | grep -qE '[mM]$'; then
        local num=$(echo "$num_str" | sed 's/[mM]$//')
        echo $(($num * 1000000))
    elif echo "$num_str" | grep -qE '[bB]$'; then
        local num=$(echo "$num_str" | sed 's/[bB]$//')
        echo $(($num * 1000000000))
    else
        echo "$num_str"
    fi
}

# Get parameters or use defaults
# First parameter = writes, second = reads
WRITES=${1:-1k}
READS=${2:-500}

# Parse numbers
NUM_WRITES=$(parse_number "$WRITES")
NUM_READS=$(parse_number "$READS")

echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                                                               ║${NC}"
echo -e "${CYAN}║     READ/WRITE PERFORMANCE TEST (NATIVE)                      ║${NC}"
echo -e "${CYAN}║                                                               ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Configuration:${NC}"
echo -e "  Writes: ${GREEN}${NUM_WRITES}${NC}"
echo -e "  Reads:  ${GREEN}${NUM_READS}${NC}"
echo ""

# Check if benchmark binary exists
BENCH_BIN="${PROJECT_ROOT}/target/release/narayana-bench"
if [ ! -f "$BENCH_BIN" ]; then
    echo -e "${BLUE}[1/2]${NC} Building native benchmark..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin narayana-bench 2>&1 | tail -3
    if [ $? -ne 0 ]; then
        echo -e "${RED}[ERROR]${NC} Failed to build benchmark"
        exit 1
    fi
    echo -e "${GREEN}[OK]${NC} Build complete"
    echo ""
fi

# Run native benchmark
echo -e "${BLUE}[2/2]${NC} Running native benchmark..."
echo ""

"$BENCH_BIN" --writes "$NUM_WRITES" --reads "$NUM_READS"
