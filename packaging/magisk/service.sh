#!/system/bin/sh

WORK_DIR="/data/local/tmp/coreshift"
CONFIG_FILE="$WORK_DIR/corepolicy.conf"
UTENSIL_WORK_DIR="/data/local/tmp/utensil"
UTENSIL_CONFIG_FILE="$UTENSIL_WORK_DIR/utensil.conf"
LOG_FILE="$WORK_DIR/service.log"
LOG_ROTATED="$WORK_DIR/service.log.1"
MAX_LOG_BYTES=262144
PIDOF_BIN="/system/bin/pidof"
SLEEP_BIN="/system/bin/sleep"

RESTART_DELAY_SECS=5
RESTART_DELAY_MAX_SECS=60
RESTART_RESET_RUNTIME_SECS=300

DAEMON="${0%/*}/system/bin/corepolicy"
UTENSIL_DAEMON="${0%/*}/system/bin/utensil-poker"

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

mkdir -p "$WORK_DIR"
chmod 0755 "$WORK_DIR"
mkdir -p "$UTENSIL_WORK_DIR"
chmod 0755 "$UTENSIL_WORK_DIR"

export COREPOLICY_CONFIG="$CONFIG_FILE"
export UTENSIL_CONFIG="$UTENSIL_CONFIG_FILE"

if [ ! -f "$UTENSIL_CONFIG_FILE" ] && [ -f "${0%/*}/utensil.conf" ]; then
    cp "${0%/*}/utensil.conf" "$UTENSIL_CONFIG_FILE"
    chmod 0644 "$UTENSIL_CONFIG_FILE"
fi

rotate_logs

log_line "Waiting for SystemUI"

until "$PIDOF_BIN" com.android.systemui >/dev/null 2>&1; do
    "$SLEEP_BIN" 8
done

log_line "SystemUI ready; starting daemon"

if [ ! -x "$DAEMON" ]; then
    log_line "CoreShift Policy daemon missing or not executable: $DAEMON"
    exit 1
fi

if [ -x "$UTENSIL_DAEMON" ]; then
    (
        delay="$RESTART_DELAY_SECS"
        while true; do
            rotate_logs

            start_ts="$(date +%s 2>/dev/null || echo 0)"
            log_line "Starting Utensil Poker daemon"

            "$UTENSIL_DAEMON" daemon >> "$LOG_FILE" 2>&1 &
            utensil_pid="$!"

            log_line "Utensil Poker daemon started pid=$utensil_pid"

            wait "$utensil_pid"
            exit_code="$?"

            end_ts="$(date +%s 2>/dev/null || echo 0)"

            runtime_secs=0
            if [ "$end_ts" -ge "$start_ts" ] 2>/dev/null; then
                runtime_secs=$((end_ts - start_ts))
            fi

            rotate_logs
            log_line "Utensil Poker daemon exited code=$exit_code runtime_secs=$runtime_secs restarting_in=$delay"

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
    ) &
else
    log_line "Utensil Poker daemon missing or not executable: $UTENSIL_DAEMON"
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
