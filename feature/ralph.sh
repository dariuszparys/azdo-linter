#!/bin/bash
# Ralph Wiggum Loop Runner
# Generated for: pipeline-validator
#
# This script runs Claude Code in a loop, completing one user story per iteration
# until all stories pass or max iterations is reached.

set -e

# Configuration
PROMPT_FILE="./prompt.md"
PRD_FILE="./prd.json"
PROGRESS_FILE="./progress.txt"
LOG_FILE="./ralph.log"
MAX_ITERATIONS=${MAX_ITERATIONS:-30}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check required files exist
check_files() {
    local missing=0
    for file in "$PROMPT_FILE" "$PRD_FILE" "$PROGRESS_FILE"; do
        if [[ ! -f "$file" ]]; then
            echo -e "${RED}Error: Required file not found: $file${NC}"
            missing=1
        fi
    done
    if [[ $missing -eq 1 ]]; then
        exit 1
    fi
}

# Get current story number from progress.txt
get_current_story() {
    grep "^Current story:" "$PROGRESS_FILE" | sed 's/Current story: //'
}

# Get total story count from prd.json
get_total_stories() {
    jq '.userStories | length' "$PRD_FILE"
}

# Check if all stories are complete
all_stories_complete() {
    local incomplete
    incomplete=$(jq '[.userStories[] | select(.passes == false)] | length' "$PRD_FILE")
    [[ "$incomplete" -eq 0 ]]
}

# Run one iteration of Claude Code
run_iteration() {
    local iteration=$1
    local current_story
    current_story=$(get_current_story)
    local total_stories
    total_stories=$(get_total_stories)

    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Iteration $iteration of $MAX_ITERATIONS${NC}"
    echo -e "${BLUE}Story $current_story of $total_stories${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # Run Claude Code with the prompt
    if claude --dangerously-skip-permissions -p "$(cat "$PROMPT_FILE")" 2>&1 | tee -a "$LOG_FILE"; then
        echo -e "${GREEN}Iteration $iteration completed${NC}"
    else
        echo -e "${RED}Iteration $iteration failed${NC}"
    fi

    echo ""
}

# Main loop
main() {
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}  Ralph Wiggum Loop: pipeline-validator${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    check_files

    local total_stories
    total_stories=$(get_total_stories)
    echo -e "Total stories: ${BLUE}$total_stories${NC}"
    echo -e "Max iterations: ${BLUE}$MAX_ITERATIONS${NC}"
    echo -e "Log file: ${BLUE}$LOG_FILE${NC}"
    echo ""

    # Initialize log
    echo "=== Ralph Loop Started: $(date) ===" >> "$LOG_FILE"
    echo "Task: pipeline-validator" >> "$LOG_FILE"
    echo "" >> "$LOG_FILE"

    local iteration=1
    while [[ $iteration -le $MAX_ITERATIONS ]]; do
        # Check if all stories are complete
        if all_stories_complete; then
            echo ""
            echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
            echo -e "${GREEN}  ALL STORIES COMPLETE!${NC}"
            echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
            echo "=== All stories complete: $(date) ===" >> "$LOG_FILE"
            exit 0
        fi

        run_iteration $iteration

        # Brief pause between iterations
        sleep 2

        ((iteration++))
    done

    echo ""
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}  Max iterations reached ($MAX_ITERATIONS)${NC}"
    echo -e "${YELLOW}  Check progress.txt and prd.json for status${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "=== Max iterations reached: $(date) ===" >> "$LOG_FILE"
}

main "$@"
