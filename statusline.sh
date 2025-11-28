#!/bin/bash
# Claude Code Mux Statusline Script
# Shows actual model/provider used by CCM after routing decisions
#
# Installed via: ccm install-statusline
# File location: ~/.claude-code-mux/statusline.sh
#
# Displays: current (recent1 recent2) HH:MM:SS
# Example: minimax-m2@minimax (claude-sonnet@anthropic gpt-4@openai) 14:23:45

# Only show CCM info if Claude Code is using CCM (ANTHROPIC_BASE_URL set)
if [ -z "$ANTHROPIC_BASE_URL" ]; then
    exit 0
fi

# CCM writes routing info to this file (now includes history)
CCM_FILE="$HOME/.claude-code-mux/last_routing.json"

if [ -f "$CCM_FILE" ]; then
    # Read current routing info
    MODEL=$(jq -r '.model // "unknown"' "$CCM_FILE")
    PROVIDER=$(jq -r '.provider // "unknown"' "$CCM_FILE")
    ROUTE_TYPE=$(jq -r '.route_type // "default"' "$CCM_FILE")
    TIMESTAMP=$(jq -r '.timestamp // ""' "$CCM_FILE")

    # Read recent models (array, already deduplicated by CCM)
    RECENT_MODELS=$(jq -r '.recent // [] | join(" ")' "$CCM_FILE")

    if [ -n "$RECENT_MODELS" ]; then
        # Extract first model as current (already in current format)
        CURRENT_MODEL="${RECENT_MODELS%% *}"
        # Get the rest (excluding current)
        OTHER_MODELS="${RECENT_MODELS#* }"

        if [ -n "$OTHER_MODELS" ]; then
            # Format: current (recent1 recent2) HH:MM:SS
            echo "$CURRENT_MODEL ($OTHER_MODELS) $TIMESTAMP"
        else
            # Only one model so far
            echo "$CURRENT_MODEL $TIMESTAMP"
        fi
    else
        # Fallback to original format if no recent array
        echo "$MODEL@$PROVIDER ($ROUTE_TYPE) $TIMESTAMP"
    fi
else
    echo "CCM: no routing yet"
fi

# Customize this script! You can also use data from Claude Code's JSON stdin:
# input=$(cat)
# COST=$(echo "$input" | jq -r '.session.total_cost // "0"')
# echo "$MODEL@$PROVIDER ($ROUTE_TYPE) \$$COST $TIMESTAMP"
