#!/system/bin/sh

WORK_DIR="/data/local/tmp/coreshift"
CONFIG_FILE="$WORK_DIR/corepolicy.conf"
LOG_FILE="$WORK_DIR/service.log"
LOG_ROTATED="$WORK_DIR/service.log.1"
MAX_LOG_BYTES=262144
GETPROP_BIN="/system/bin/getprop"
PIDOF_BIN="/system/bin/pidof"
SLEEP_BIN="/system/bin/sleep"

RESTART_DELAY_SECS=5
RESTART_DELAY_MAX_SECS=60
RESTART_RESET_RUNTIME_SECS=300

DAEMON="${0%/*}/system/bin/corepolicy"

timestamp() {
    date '+%Y-%m-%dT%H:%M:%S%z' 2>/dev/null || echo "unknown-time"
}

rotate_logs() {
    [ -f "$LOG_FILE" ] || return 0

    size="$(wc -c < "$LOG_FILE" 2>/dev/null)"
    [ -n "$size" ] || return 0

    if [ "$size" -ge "$MAX_LOG_BYTES" ]; then
        rm -f "$LOG_ROTATED"
        mv "$LOG_FILE" "$LOG_ROTATED"
    fi
}

log_line() {
    echo "$(timestamp) $*" >> "$LOG_FILE"
}

until [ "$("$GETPROP_BIN" sys.boot_completed 2>/dev/null)" = "1" ]; do
    "$SLEEP_BIN" 5
done

mkdir -p "$WORK_DIR"
chmod 0755 "$WORK_DIR"

export COREPOLICY_CONFIG="$CONFIG_FILE"

rotate_logs

log_line "Boot completed; waiting for SystemUI"

until "$PIDOF_BIN" com.android.systemui >/dev/null 2>&1; do
    "$SLEEP_BIN" 2
done

log_line "SystemUI ready; starting daemon"

if [ ! -x "$DAEMON" ]; then
    log_line "CoreShift Policy daemon missing or not executable: $DAEMON"
    exit 1
fi

delay="$RESTART_DELAY_SECS"

while true; do
    rotate_logs

    start_ts="$(date +%s 2>/dev/null || echo 0)"
    log_line "Starting CoreShift Policy daemon"

    "$DAEMON" daemon >> "$LOG_FILE" 2>&1 &
    daemon_pid="$!"

    log_line "CoreShift Policy daemon started pid=$daemon_pid"

    wait "$daemon_pid"
    exit_code="$?"

    end_ts="$(date +%s 2>/dev/null || echo 0)"

    runtime_secs=0
    if [ "$end_ts" -ge "$start_ts" ] 2>/dev/null; then
        runtime_secs=$((end_ts - start_ts))
    fi

    rotate_logs
    log_line "CoreShift Policy daemon exited code=$exit_code runtime_secs=$runtime_secs restarting_in=$delay"

    "$SLEEP_BIN" "$delay"

    if [ "$runtime_secs" -ge "$RESTART_RESET_RUNTIME_SECS" ] 2>/dev/null; then
        delay="$RESTART_DELAY_SECS"
    elif [ "$delay" -lt "$RESTART_DELAY_MAX_SECS" ]; then
        delay=$((delay * 2))
        if [ "$delay" -gt "$RESTART_DELAY_MAX_SECS" ]; then
            delay="$RESTART_DELAY_MAX_SECS"
        fi
    fi
done
