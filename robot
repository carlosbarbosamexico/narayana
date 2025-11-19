#!/bin/bash
# Unified Robot Demo Script
# Usage: ./robot [start|dev|stop|verify]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="${SCRIPT_DIR}/examples"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Configuration
HTTP_PORT=${NARAYANA_HTTP_PORT:-8080}
DATA_DIR=${NARAYANA_DATA_DIR:-./data}

# Functions
info() {
    echo -e "${BLUE}ℹ${NC}  $1"
}

success() {
    echo -e "${GREEN}✅${NC}  $1"
}

warning() {
    echo -e "${YELLOW}⚠${NC}  $1"
}

error() {
    echo -e "${RED}❌${NC}  $1"
    exit 1
}

# Check if server is running
is_server_running() {
    lsof -Pi :${HTTP_PORT} -sTCP:LISTEN -t >/dev/null 2>&1
}

# Start server in background
start_server() {
    if is_server_running; then
        warning "Server already running on port ${HTTP_PORT}"
        return 0
    fi

    info "Starting NarayanaDB server in background..."
    
    # Set environment variables
    export NARAYANA_ADMIN_USER=${NARAYANA_ADMIN_USER:-admin}
    export NARAYANA_ADMIN_PASSWORD=${NARAYANA_ADMIN_PASSWORD:-admin123}
    export NARAYANA_HTTP_PORT=${HTTP_PORT}
    export NARAYANA_DATA_DIR=${DATA_DIR}
    
    # Create data directory
    mkdir -p "${DATA_DIR}"
    
    # Start server
    cd "${SCRIPT_DIR}"
    nohup ./target/release/narayana-server > /tmp/narayana_server.log 2>&1 &
    SERVER_PID=$!
    echo $SERVER_PID > /tmp/narayana_server.pid
    
    # Wait for server to start
    info "Waiting for server to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:${HTTP_PORT}/health > /dev/null 2>&1; then
            success "Server started (PID: $SERVER_PID)"
            info "  HTTP API: http://localhost:${HTTP_PORT}"
            info "  Logs: tail -f /tmp/narayana_server.log"
            return 0
        fi
        sleep 1
    done
    
    error "Server failed to start. Check /tmp/narayana_server.log"
}

# Start server in foreground (dev mode)
dev_server() {
    if is_server_running; then
        warning "Server already running on port ${HTTP_PORT}"
        info "Stopping existing server..."
        stop_server
        sleep 2
    fi

    info "Starting NarayanaDB server in foreground (dev mode)..."
    
    # Set environment variables
    export NARAYANA_ADMIN_USER=${NARAYANA_ADMIN_USER:-admin}
    export NARAYANA_ADMIN_PASSWORD=${NARAYANA_ADMIN_PASSWORD:-admin123}
    export NARAYANA_HTTP_PORT=${HTTP_PORT}
    export NARAYANA_DATA_DIR=${DATA_DIR}
    
    # Create data directory
    mkdir -p "${DATA_DIR}"
    
    # Start server in foreground
    cd "${SCRIPT_DIR}"
    ./target/release/narayana-server
}

# Stop server
stop_server() {
    info "Stopping NarayanaDB server..."
    
    if [ -f /tmp/narayana_server.pid ]; then
        PID=$(cat /tmp/narayana_server.pid)
        if ps -p $PID > /dev/null 2>&1; then
            kill $PID 2>/dev/null || true
            sleep 2
            if ps -p $PID > /dev/null 2>&1; then
                warning "Force killing server..."
                kill -9 $PID 2>/dev/null || true
            fi
            success "Server stopped"
        else
            warning "Process $PID not found"
        fi
        rm -f /tmp/narayana_server.pid
    else
        # Try to find and kill by process name
        pkill -f narayana-server || warning "No running server found"
    fi
    
    # Clean up any remaining processes
    pkill -f "target/release/narayana-server" || true
    pkill -f "target/debug/narayana-server" || true
}

# Run robot demo
run_demo() {
    if ! is_server_running; then
        error "Server is not running. Start it with: ./robot start"
    fi
    
    info "Running robot learning demo..."
    "${EXAMPLES_DIR}/robot_demo.sh"
}

# Verify robot features
verify_features() {
    info "Verifying robot features..."
    "${EXAMPLES_DIR}/verify_robot_features.sh"
}

# Show usage
usage() {
    echo "Usage: ./robot [command]"
    echo ""
    echo "Commands:"
    echo "  start   - Start server in background"
    echo "  dev     - Start server in foreground (for development)"
    echo "  stop    - Stop the server"
    echo "  demo    - Run the robot learning demo (requires server running)"
    echo "  verify  - Verify all robot features are working"
    echo ""
    echo "Examples:"
    echo "  ./robot start    # Start server"
    echo "  ./robot demo     # Run demo"
    echo "  ./robot stop     # Stop server"
}

# Main command handler
case "${1:-}" in
    start)
        start_server
        ;;
    dev)
        dev_server
        ;;
    stop)
        stop_server
        ;;
    demo)
        run_demo
        ;;
    verify)
        verify_features
        ;;
    *)
        usage
        exit 1
        ;;
esac

