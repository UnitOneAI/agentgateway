#!/bin/bash

###############################################################################
# Security Guard Real-Time Monitoring Script
###############################################################################
#
# This script provides real-time monitoring of security guard activity
# in the UnitOne AgentGateway deployment.
#
# Usage:
#   ./monitor_guards.sh [command]
#
# Commands:
#   live        - Live tail of guard activity (default)
#   recent      - Show recent guard decisions (last 100 lines)
#   blocks      - Show only blocked requests
#   allows      - Show allowed requests with guard checks
#   stats       - Show guard statistics
#   errors      - Show guard errors
#   performance - Show guard performance metrics
#   all         - Show all guard-related logs
#
###############################################################################

# Configuration
RESOURCE_GROUP="mcp-gateway-dev-rg"
APP_NAME="unitone-agentgateway"
LINES=100

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

###############################################################################
# Functions
###############################################################################

print_header() {
    echo -e "${BLUE}================================================================================${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${BLUE}================================================================================${NC}"
    echo ""
}

print_section() {
    echo ""
    echo -e "${YELLOW}→ $1${NC}"
    echo -e "${YELLOW}$(printf '─%.0s' {1..80})${NC}"
}

show_live_tail() {
    print_header "🛡️  SECURITY GUARD LIVE MONITORING"

    echo -e "${GREEN}Monitoring guard activity in real-time...${NC}"
    echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
    echo ""

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --follow \
        2>&1 | grep -iE "(guard|security|blocked|denied|allowed)" \
        | while read -r line; do
            if echo "$line" | grep -qi "blocked\|denied\|deny"; then
                echo -e "${RED}🛑 $line${NC}"
            elif echo "$line" | grep -qi "allowed\|allow\|passed"; then
                echo -e "${GREEN}✅ $line${NC}"
            elif echo "$line" | grep -qi "error\|failed"; then
                echo -e "${YELLOW}⚠️  $line${NC}"
            else
                echo -e "${CYAN}   $line${NC}"
            fi
        done
}

show_recent() {
    print_header "📋 RECENT GUARD ACTIVITY (Last $LINES lines)"

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail "$LINES" \
        2>&1 | grep -iE "(guard|security|blocked|denied|allowed)" \
        | while read -r line; do
            if echo "$line" | grep -qi "blocked\|denied\|deny"; then
                echo -e "${RED}🛑 $line${NC}"
            elif echo "$line" | grep -qi "allowed\|allow\|passed"; then
                echo -e "${GREEN}✅ $line${NC}"
            else
                echo -e "${CYAN}   $line${NC}"
            fi
        done
}

show_blocks() {
    print_header "🛑 BLOCKED REQUESTS"

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail "$LINES" \
        2>&1 | grep -iE "(blocked|denied|deny)" \
        | while read -r line; do
            echo -e "${RED}🛑 $line${NC}"
        done

    echo ""
    echo -e "${YELLOW}Block Statistics:${NC}"
    local count=$(az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail 1000 \
        2>&1 | grep -icE "(blocked|denied|deny)")
    echo -e "  Total blocks in last 1000 lines: ${RED}$count${NC}"
}

show_allows() {
    print_header "✅ ALLOWED REQUESTS"

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail "$LINES" \
        2>&1 | grep -iE "(allowed|allow|passed)" \
        | while read -r line; do
            echo -e "${GREEN}✅ $line${NC}"
        done
}

show_stats() {
    print_header "📊 GUARD STATISTICS"

    local logs=$(az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail 1000 \
        2>&1)

    local total_blocks=$(echo "$logs" | grep -icE "(blocked|denied|deny)" || echo "0")
    local total_allows=$(echo "$logs" | grep -icE "(allowed|allow|passed)" || echo "0")
    local total_errors=$(echo "$logs" | grep -icE "(error|failed)" || echo "0")
    local total_guard_logs=$(echo "$logs" | grep -icE "(guard)" || echo "0")

    echo -e "${CYAN}Guard Activity Summary (Last 1000 log lines):${NC}"
    echo ""
    echo -e "  🛑 Blocked Requests:   ${RED}$total_blocks${NC}"
    echo -e "  ✅ Allowed Requests:   ${GREEN}$total_allows${NC}"
    echo -e "  ⚠️  Guard Errors:       ${YELLOW}$total_errors${NC}"
    echo -e "  📝 Total Guard Logs:   ${CYAN}$total_guard_logs${NC}"
    echo ""

    if [ "$total_guard_logs" -gt 0 ]; then
        local block_rate=$(awk "BEGIN {printf \"%.2f\", ($total_blocks / $total_guard_logs) * 100}")
        echo -e "  📈 Block Rate:         ${RED}${block_rate}%${NC}"
    fi

    print_section "Guard Configuration"
    echo -e "  Guard ID:              ${CYAN}tool-poisoning-guard${NC}"
    echo -e "  Type:                  ${CYAN}Native Tool Poisoning Detector${NC}"
    echo -e "  Latency:               ${GREEN}< 1ms${NC}"
    echo -e "  Mode:                  ${CYAN}Strict (23+ built-in patterns)${NC}"
    echo -e "  Failure Mode:          ${YELLOW}fail_closed${NC}"
    echo -e "  Timeout:               ${CYAN}50ms${NC}"
}

show_errors() {
    print_header "⚠️  GUARD ERRORS"

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail "$LINES" \
        2>&1 | grep -iE "(guard.*error|guard.*failed|guard.*timeout)" \
        | while read -r line; do
            echo -e "${YELLOW}⚠️  $line${NC}"
        done

    echo ""
    local error_count=$(az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail 1000 \
        2>&1 | grep -icE "(guard.*error|guard.*failed|guard.*timeout)")
    echo -e "${YELLOW}Total guard errors in last 1000 lines: $error_count${NC}"
}

show_performance() {
    print_header "⚡ GUARD PERFORMANCE METRICS"

    print_section "Latest Guard Execution Times"

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail 200 \
        2>&1 | grep -iE "(guard.*latency|guard.*duration|guard.*ms)" \
        | tail -20 \
        | while read -r line; do
            echo -e "${CYAN}   $line${NC}"
        done

    print_section "Guard Timeout Events"

    local timeout_count=$(az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail 1000 \
        2>&1 | grep -icE "(guard.*timeout)")

    if [ "$timeout_count" -gt 0 ]; then
        echo -e "${RED}⚠️  Guard timeouts detected: $timeout_count${NC}"
        az containerapp logs show \
            --name "$APP_NAME" \
            --resource-group "$RESOURCE_GROUP" \
            --tail 1000 \
            2>&1 | grep -iE "(guard.*timeout)" | tail -10
    else
        echo -e "${GREEN}✅ No guard timeouts detected${NC}"
    fi
}

show_all() {
    print_header "📝 ALL GUARD LOGS"

    az containerapp logs show \
        --name "$APP_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --tail "$LINES" \
        2>&1 | grep -iE "(guard)" \
        | while read -r line; do
            if echo "$line" | grep -qi "blocked\|denied\|deny"; then
                echo -e "${RED}🛑 $line${NC}"
            elif echo "$line" | grep -qi "allowed\|allow\|passed"; then
                echo -e "${GREEN}✅ $line${NC}"
            elif echo "$line" | grep -qi "error\|failed"; then
                echo -e "${YELLOW}⚠️  $line${NC}"
            else
                echo -e "${CYAN}   $line${NC}"
            fi
        done
}

show_help() {
    print_header "📖 SECURITY GUARD MONITORING HELP"

    echo "Usage: ./monitor_guards.sh [command]"
    echo ""
    echo "Commands:"
    echo -e "  ${GREEN}live${NC}        - Live tail of guard activity (default)"
    echo -e "  ${GREEN}recent${NC}      - Show recent guard decisions (last 100 lines)"
    echo -e "  ${GREEN}blocks${NC}      - Show only blocked requests"
    echo -e "  ${GREEN}allows${NC}      - Show allowed requests with guard checks"
    echo -e "  ${GREEN}stats${NC}       - Show guard statistics"
    echo -e "  ${GREEN}errors${NC}      - Show guard errors"
    echo -e "  ${GREEN}performance${NC} - Show guard performance metrics"
    echo -e "  ${GREEN}all${NC}         - Show all guard-related logs"
    echo -e "  ${GREEN}help${NC}        - Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./monitor_guards.sh              # Live monitoring (default)"
    echo "  ./monitor_guards.sh recent       # Recent activity"
    echo "  ./monitor_guards.sh blocks       # Show blocked requests"
    echo "  ./monitor_guards.sh stats        # Show statistics"
    echo ""
    echo "Guard Configuration:"
    echo "  Resource Group: $RESOURCE_GROUP"
    echo "  Container App:  $APP_NAME"
    echo ""
}

###############################################################################
# Main
###############################################################################

# Check if Azure CLI is installed
if ! command -v az &> /dev/null; then
    echo -e "${RED}Error: Azure CLI (az) is not installed${NC}"
    echo "Install it from: https://docs.microsoft.com/cli/azure/install-azure-cli"
    exit 1
fi

# Parse command
COMMAND="${1:-live}"

case "$COMMAND" in
    live)
        show_live_tail
        ;;
    recent)
        show_recent
        ;;
    blocks)
        show_blocks
        ;;
    allows)
        show_allows
        ;;
    stats)
        show_stats
        ;;
    errors)
        show_errors
        ;;
    performance)
        show_performance
        ;;
    all)
        show_all
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo -e "${RED}Unknown command: $COMMAND${NC}"
        echo ""
        show_help
        exit 1
        ;;
esac
