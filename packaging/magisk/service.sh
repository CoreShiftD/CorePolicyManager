#!/system/bin/sh

# Wait for boot completion
until [ "$(getprop sys.boot_completed)" = "1" ]; do
    sleep 10
done

# Create work dir
mkdir -p /data/local/tmp/coreshift
chmod 0755 /data/local/tmp/coreshift

DEBUG_FILE="/data/local/tmp/coreshift/debug"
LOG_FILE="/data/local/tmp/coreshift/service.log"

if [ -f "$DEBUG_FILE" ]; then
    export COREPOLICY_STDOUT_LOG=1
    export COREPOLICY_DEBUG_LOG=1
fi

# Run daemon
${0%/*}/system/bin/corepolicy -p >> "$LOG_FILE" 2>&1

# Log exit code
echo "CoreShift Policy daemon exited with code $?" >> "$LOG_FILE"
