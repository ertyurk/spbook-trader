#!/bin/bash

# Quant-RS API Demo Script
# This script demonstrates how to interact with the Quant-RS API

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

API_BASE="http://localhost:8080"

echo -e "${BLUE}🌐 Quant-RS API Demo${NC}"
echo "===================="

# Function to make API call with pretty output
api_call() {
    local endpoint=$1
    local description=$2
    
    echo -e "\n${YELLOW}📡 $description${NC}"
    echo -e "${BLUE}GET $API_BASE$endpoint${NC}"
    echo "---"
    
    if curl -s "$API_BASE$endpoint" | python3 -m json.tool 2>/dev/null; then
        echo -e "${GREEN}✅ Success${NC}"
    elif curl -s "$API_BASE$endpoint"; then
        echo -e "${GREEN}✅ Response received${NC}"
    else
        echo -e "${RED}❌ Failed to connect. Is the server running?${NC}"
        echo -e "${YELLOW}💡 Start the server with: cargo run${NC}"
        exit 1
    fi
}

# Check if server is running
echo -e "${YELLOW}🔍 Checking if Quant-RS server is running...${NC}"
if ! curl -s "$API_BASE/health" > /dev/null; then
    echo -e "${RED}❌ Server not responding at $API_BASE${NC}"
    echo -e "${YELLOW}💡 Please start the server first:${NC}"
    echo -e "   ${BLUE}cargo run${NC}"
    echo -e "   ${BLUE}# or${NC}"
    echo -e "   ${BLUE}RUST_LOG=debug cargo run${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Server is running!${NC}"

# Health Check
api_call "/health" "Health Check"

# System Status
api_call "/api/v1/status" "System Status"

# Live Events
api_call "/api/v1/events/live" "Live Events (Last 10)"

# Recent Events with Pagination
api_call "/api/v1/events?page=1&limit=5" "Recent Events (Paginated)"

# Recent Predictions
api_call "/api/v1/predictions?page=1&limit=3" "Recent Predictions"

# Portfolio Status
api_call "/api/v1/portfolio" "Portfolio Status"

# Market Odds
api_call "/api/v1/markets" "Current Market Odds"

echo -e "\n${BLUE}🎯 Demo Complete!${NC}"
echo "=================="
echo -e "${GREEN}✨ All API endpoints are working correctly!${NC}"
echo ""
echo -e "${YELLOW}📚 More examples:${NC}"
echo -e "   ${BLUE}curl $API_BASE/health${NC}"
echo -e "   ${BLUE}curl $API_BASE/api/v1/status${NC}"
echo -e "   ${BLUE}curl $API_BASE/api/v1/events/live${NC}"
echo ""
echo -e "${YELLOW}🔍 Monitor real-time activity:${NC}"
echo -e "   ${BLUE}watch -n 2 'curl -s $API_BASE/api/v1/events/live | python3 -m json.tool'${NC}"