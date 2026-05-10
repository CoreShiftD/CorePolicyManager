#!/system/bin/sh

WORK_DIR="/data/local/tmp/coreshift"
CONFIG_FILE="$WORK_DIR/corepolicy.conf"
DEBUG_FILE="$WORK_DIR/debug"
LOG_FILE="$WORK_DIR/service.log"
LOG_ROTATED="$WORK_DIR/service.log.1"
MAX_LOG_BYTES=262144
RESTART_DELAY_SECS=5
RESTART_DELAY_MAX_SECS=60
RESTART_RESET_RUNTIME_SECS=300
DAEMON="${0%/*}/system/bin/corepolicy"

timestamp() {
    date '+%Y-%m-%dT%H:%M:%S%z' 2>/dev/null || echo "unknown-time"
}

rotate_logs() {
    if [ ! -f "$LOG_FILE" ]; then
        return 0
    fi

    size=$(wc -c < "$LOG_FILE" 2>/dev/null)
    if [ -z "$size" ] || [ "$size" -lt "$MAX_LOG_BYTES" ]; then
        return 0
    fi

    rm -f "$LOG_ROTATED"
    mv "$LOG_FILE" "$LOG_ROTATED"
}

log_line() {
    echo "$(timestamp) $*" >> "$LOG_FILE"
}

until [ "$(getprop sys.boot_completed)" = "1" ]; do
    sleep 5
done

mkdir -p "$WORK_DIR"
chmod 0755 "$WORK_DIR"

if [ -f "$DEBUG_FILE" ]; then
    export COREPOLICY_STDOUT_LOG=1
    export COREPOLICY_DEBUG_LOG=1
fi

export COREPOLICY_CONFIG="$CONFIG_FILE"

if [ ! -x "$DAEMON" ]; then
    rotate_logs
    log_line "CoreShift Policy daemon missing or not executable: $DAEMON"
    exit 1
fi

delay="$RESTART_DELAY_SECS"

while true; do
    rotate_logs
    start_ts=$(date +%s 2>/dev/null || echo 0)
    log_line "Starting CoreShift Policy daemon"
    "$DAEMON" daemon >> "$LOG_FILE" 2>&1
    exit_code=$?
    end_ts=$(date +%s 2>/dev/null || echo 0)

    runtime_secs=0
    if [ "$end_ts" -ge "$start_ts" ] 2>/dev/null; then
        runtime_secs=$((end_ts - start_ts))
    fi

    rotate_logs
    log_line "CoreShift Policy daemon exited code=$exit_code runtime_secs=$runtime_secs restarting_in=$delay"
    sleep "$delay"

    if [ "$runtime_secs" -ge "$RESTART_RESET_RUNTIME_SECS" ] 2>/dev/null; then
        delay="$RESTART_DELAY_SECS"
        continue
    fi

    if [ "$delay" -lt "$RESTART_DELAY_MAX_SECS" ]; then
        delay=$((delay * 2))
        if [ "$delay" -gt "$RESTART_DELAY_MAX_SECS" ]; then
            delay="$RESTART_DELAY_MAX_SECS"
        fi
    fi
done
