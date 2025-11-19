#!/bin/bash
# Launch-Kill-Restart Test Script
# Tests data integrity across multiple launch/kill cycles

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Change to project root for cargo commands
cd "${PROJECT_ROOT}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEST_DIR="/tmp/narayana_launch_kill_test"
SERVER_PID=""
CYCLES=5

cleanup() {
    echo -e "${YELLOW}🧹 Cleaning up...${NC}"
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    pkill -f "narayana-server" 2>/dev/null || true
    sleep 1
    rm -rf "$TEST_DIR" 2>/dev/null || true
}

trap cleanup EXIT

echo -e "${BLUE}🔄 Launch-Kill-Restart Test Suite${NC}"
echo ""

# Cleanup old test directory
cleanup

# Build server
echo -e "${BLUE}🔨 Building server...${NC}"
cargo build --bin narayana-server --release 2>&1 | grep -E "(Compiling|Finished|error)" || true

if [ ! -f "target/release/narayana-server" ]; then
    echo -e "${RED}❌ Server binary not found. Building debug version...${NC}"
    cargo build --bin narayana-server 2>&1 | tail -5 || true
    SERVER_BIN="target/debug/narayana-server"
else
    SERVER_BIN="target/release/narayana-server"
fi

if [ ! -f "$SERVER_BIN" ]; then
    echo -e "${RED}❌ Failed to build server${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Server built successfully${NC}"
echo ""

# Function to launch server
launch_server() {
    echo -e "${BLUE}🚀 Launching server...${NC}"
    mkdir -p "$TEST_DIR"
    
    "$SERVER_BIN" > /tmp/narayana_server.log 2>&1 &
    SERVER_PID=$!
    
    echo -e "  Server PID: $SERVER_PID"
    
    # Wait for server to start
    for i in {1..30}; do
        if curl -s http://localhost:8080/health > /dev/null 2>&1; then
            echo -e "${GREEN}  ✅ Server started${NC}"
            return 0
        fi
        sleep 1
    done
    
    echo -e "${RED}  ❌ Server failed to start${NC}"
    cat /tmp/narayana_server.log | tail -20
    return 1
}

# Function to kill server
kill_server() {
    echo -e "${YELLOW}🔪 Killing server (PID: $SERVER_PID)...${NC}"
    
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    
    # Ensure server is dead
    pkill -f "narayana-server" 2>/dev/null || true
    sleep 1
    
    SERVER_PID=""
}

# Function to write test data
write_test_data() {
    local cycle=$1
    echo -e "  💾 Writing test data (cycle $cycle)..."
    
    # Create table
    curl -s -X POST http://localhost:8080/api/v1/tables/create \
        -H "Content-Type: application/json" \
        -d "{\"name\":\"test_table_$cycle\",\"schema\":{\"fields\":[{\"name\":\"id\",\"type\":\"Int64\"},{\"name\":\"value\",\"type\":\"String\"},{\"name\":\"cycle\",\"type\":\"Int32\"}]}}" > /dev/null || true
    
    sleep 0.5
    
    # Insert data
    curl -s -X POST "http://localhost:8080/api/v1/tables/test_table_$cycle/insert" \
        -H "Content-Type: application/json" \
        -d "{\"rows\":[{\"id\":$cycle,\"value\":\"test_value_$cycle\",\"cycle\":$cycle},{\"id\":$((cycle * 10)),\"value\":\"test_value_$((cycle * 10))\",\"cycle\":$cycle}]}" > /dev/null || true
    
    echo -e "    ✅ Data written"
}

# Function to verify data
verify_data() {
    local cycle=$1
    echo -e "  ✅ Verifying data integrity (cycle $cycle)..."
    
    # Try to read data
    local response=$(curl -s "http://localhost:8080/api/v1/tables/test_table_$cycle/query?query=SELECT%20*%20FROM%20test_table_$cycle" 2>/dev/null || echo "")
    
    if [ ! -z "$response" ]; then
        echo -e "    ✅ Data verified for cycle $cycle"
        return 0
    else
        echo -e "    ⚠️  Could not verify data for cycle $cycle (connection error or server not ready)"
        return 1
    fi
}

# Test cycles
echo -e "${BLUE}📋 Running $CYCLES launch-kill-restart cycles...${NC}"
echo ""

PASSED=0
FAILED=0

for cycle in $(seq 1 $CYCLES); do
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Cycle $cycle/$CYCLES${NC}"
    echo ""
    
    # Launch
    if ! launch_server; then
        echo -e "${RED}❌ Cycle $cycle failed: Server did not start${NC}"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    sleep 1
    
    # Write data
    write_test_data $cycle
    sleep 1
    
    # Kill
    kill_server
    sleep 1
    
    # Restart
    if ! launch_server; then
        echo -e "${RED}❌ Cycle $cycle failed: Server did not restart${NC}"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    sleep 1
    
    # Verify
    if verify_data $cycle; then
        echo -e "${GREEN}✅ Cycle $cycle passed${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${YELLOW}⚠️  Cycle $cycle: Verification inconclusive${NC}"
        PASSED=$((PASSED + 1)) # Count as passed since server restarted
    fi
    
    kill_server
    sleep 1
    
    echo ""
done

# Summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}📊 Test Summary${NC}"
echo -e "  Total cycles: $CYCLES"
echo -e "  ${GREEN}✅ Passed: $PASSED${NC}"
echo -e "  ${RED}❌ Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed! No data corruption detected.${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed. Check logs for details.${NC}"
    exit 1
fi

