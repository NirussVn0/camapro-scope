#!/usr/bin/env bash
# G5 Linux release candidate measurement script.
# Records FPS, RSS, latency samples during a timed preview session.
# Usage: ./scripts/g5-measure.sh --duration 1800 --host <ip> --port 8100 --token <tok>
set -euo pipefail

DURATION=1800
HOST=""
PORT=8100
TOKEN=""
OUT_DIR="docs/evidence/g5-runs/$(date +%Y%m%d_%H%M%S)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration) DURATION="$2"; shift 2 ;;
    --host) HOST="$2"; shift 2 ;;
    --port) PORT="$2"; shift 2 ;;
    --token) TOKEN="$2"; shift 2 ;;
    *) echo "Unknown: $1"; exit 1 ;;
  esac
done

if [[ -z "$HOST" || -z "$TOKEN" ]]; then
  echo "Usage: $0 --host <ip> --token <tok> [--port 8100] [--duration 1800]"
  exit 1
fi

mkdir -p "$OUT_DIR"
echo "=== G5 Measurement Run ===" | tee "$OUT_DIR/meta.txt"
echo "Date: $(date -Iseconds)" | tee -a "$OUT_DIR/meta.txt"
echo "Host: $HOST:$PORT" | tee -a "$OUT_DIR/meta.txt"
echo "Duration: ${DURATION}s" | tee -a "$OUT_DIR/meta.txt"
echo "Binary: $(readlink -f desktop/src-tauri/target/release/camapro-scope 2>/dev/null || echo MISSING)" | tee -a "$OUT_DIR/meta.txt"
echo "" | tee -a "$OUT_DIR/meta.txt"

BINARY="desktop/src-tauri/target/release/camapro-scope"
if [[ ! -x "$BINARY" ]]; then
  echo "ERROR: Release binary not found at $BINARY"
  echo "Run: cd desktop && pnpm tauri build --bundles none"
  exit 1
fi

# Start preview in background, capture stdout for frame counting
"$BINARY" --preview-smoke "$HOST:$PORT" "$TOKEN" "$DURATION" > "$OUT_DIR/preview.log" 2>&1 &
PREVIEW_PID=$!

# Sample RSS every 30s
echo "timestamp,rss_kb" > "$OUT_DIR/rss.csv"
START=$(date +%s)
while kill -0 "$PREVIEW_PID" 2>/dev/null; do
  NOW=$(date +%s)
  ELAPSED=$((NOW - START))
  RSS=$(ps -o rss= -p "$PREVIEW_PID" 2>/dev/null || echo 0)
  echo "$ELAPSED,$RSS" >> "$OUT_DIR/rss.csv"
  sleep 30
done

wait "$PREVIEW_PID"
EXIT_CODE=$?

echo "" | tee -a "$OUT_DIR/meta.txt"
echo "Exit code: $EXIT_CODE" | tee -a "$OUT_DIR/meta.txt"
echo "Preview log tail:" | tee -a "$OUT_DIR/meta.txt"
tail -5 "$OUT_DIR/preview.log" | tee -a "$OUT_DIR/meta.txt"

echo ""
echo "=== Results in $OUT_DIR ==="
echo "  meta.txt     — run metadata + exit status"
echo "  preview.log  — full preview output (frames, fps)"
echo "  rss.csv      — RSS samples (timestamp_s, rss_kb)"
echo ""
echo "Evaluate against G5 thresholds in ROADMAP.md before declaring pass."
