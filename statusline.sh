#!/bin/bash
# Claude Code Mux Statusline Script
# Shows models used in recent requests with sparkline bars
#
# Installed via: ccm install-statusline
# File location: ~/.claude-code-mux/statusline.sh
#
# Displays: model@provider ████ model2@provider ██
# Each █ = 1 request (out of last 20)

# Only show CCM info if Claude Code is using CCM (ANTHROPIC_BASE_URL set)
if [ -z "$ANTHROPIC_BASE_URL" ]; then
    exit 0
fi

CCM_FILE="$HOME/.claude-code-mux/last_routing.json"

if [ ! -f "$CCM_FILE" ]; then
    echo "CCM: no routing yet"
    exit 0
fi

# Read recent requests array
RECENT=$(jq -r '.recent // []' "$CCM_FILE")

if [ "$RECENT" = "[]" ] || [ -z "$RECENT" ]; then
    # Fallback: show current model
    MODEL=$(jq -r '.model // "unknown"' "$CCM_FILE")
    PROVIDER=$(jq -r '.provider // "unknown"' "$CCM_FILE")
    echo "$MODEL@$PROVIDER"
    exit 0
fi

# Strip date suffixes and get models in recency order (most recent first)
STRIPPED=$(echo "$RECENT" | jq -r '.[]' | sed 's/-[0-9]\{8\}@/@/')
UNIQUE_MODELS=$(echo "$STRIPPED" | awk 'seen[$0]==0 {print; seen[$0]=1; count++} count>=3 {exit}')

# Build output: models in recency order, fixed-width bars show proportion
BAR_WIDTH=10
TOTAL=20
OUTPUT=""
while read -r MODEL; do
    [ -z "$MODEL" ] && continue
    COUNT=$(echo "$STRIPPED" | grep -cx "$MODEL")

    # Calculate filled portion (at least 1 if count > 0)
    FILLED=$(( (COUNT * BAR_WIDTH + TOTAL - 1) / TOTAL ))  # round up
    [ "$FILLED" -gt "$BAR_WIDTH" ] && FILLED=$BAR_WIDTH
    [ "$FILLED" -lt 1 ] && FILLED=1
    HOLLOW=$((BAR_WIDTH - FILLED))

    BAR=$(printf '█%.0s' $(seq 1 $FILLED))$(printf '░%.0s' $(seq 1 $HOLLOW) 2>/dev/null)

    if [ -n "$OUTPUT" ]; then
        OUTPUT="$OUTPUT $MODEL $BAR"
    else
        OUTPUT="$MODEL $BAR"
    fi
done <<< "$UNIQUE_MODELS"

echo "$OUTPUT"
