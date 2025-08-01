#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🦀 Quant-RS Testing Suite${NC}"
echo "================================"

# Function to run tests with timing
run_test_suite() {
    local test_name=$1
    local test_command=$2
    
    echo -e "\n${YELLOW}📋 Running $test_name...${NC}"
    start_time=$(date +%s)
    
    if eval $test_command; then
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo -e "${GREEN}✅ $test_name completed successfully in ${duration}s${NC}"
        return 0
    else
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo -e "${RED}❌ $test_name failed after ${duration}s${NC}"
        return 1
    fi
}

# Initialize test results
total_tests=0
passed_tests=0

# Unit Tests
echo -e "\n${BLUE}🧪 Unit Tests${NC}"
echo "---------------"

if run_test_suite "ML Models Unit Tests" "cargo test ml_models_tests --lib"; then
    ((passed_tests++))
fi
((total_tests++))

if run_test_suite "Trading Engine Unit Tests" "cargo test trading_engine_tests --lib"; then
    ((passed_tests++))
fi
((total_tests++))

if run_test_suite "Models Crate Tests" "cargo test -p quant-models"; then
    ((passed_tests++))
fi
((total_tests++))

if run_test_suite "Services Crate Tests" "cargo test -p quant-services"; then
    ((passed_tests++))
fi
((total_tests++))

# Integration Tests
echo -e "\n${BLUE}🔗 Integration Tests${NC}"
echo "--------------------"

if run_test_suite "API Integration Tests" "cargo test integration_tests --test integration_tests"; then
    ((passed_tests++))
fi
((total_tests++))

# Performance Tests
echo -e "\n${BLUE}⚡ Performance Tests${NC}"
echo "--------------------"

if run_test_suite "Performance & Load Tests" "cargo test performance_tests --test performance_tests --release"; then
    ((passed_tests++))
fi
((total_tests++))

# Property-based Tests
echo -e "\n${BLUE}🎲 Property-based Tests${NC}"
echo "-----------------------"

if run_test_suite "Property Tests" "cargo test proptest"; then
    ((passed_tests++))
fi
((total_tests++))

# All Tests (comprehensive run)
echo -e "\n${BLUE}🌐 Comprehensive Test Run${NC}"
echo "-------------------------"

if run_test_suite "All Tests" "cargo test --all"; then
    ((passed_tests++))
fi
((total_tests++))

# Test Summary
echo -e "\n${BLUE}📊 Test Results Summary${NC}"
echo "======================="
echo -e "Total test suites: $total_tests"
echo -e "Passed: ${GREEN}$passed_tests${NC}"
echo -e "Failed: ${RED}$((total_tests - passed_tests))${NC}"

if [ $passed_tests -eq $total_tests ]; then
    echo -e "\n${GREEN}🎉 All test suites passed!${NC}"
    echo -e "${GREEN}✨ Your Quant-RS system is ready for production${NC}"
    exit 0
else
    echo -e "\n${RED}⚠️  Some test suites failed${NC}"
    echo -e "${YELLOW}Please review the failed tests above${NC}"
    exit 1
fi