#!/system/bin/sh

WORK_DIR="/data/local/tmp/coreshift"
CONFIG_FILE="$WORK_DIR/corepolicy.conf"

ui_print "***********************************"
ui_print "*        CoreShift Policy         *"
ui_print "***********************************"

ABI="$(getprop ro.product.cpu.abi 2>/dev/null)"
ABILIST="$(getprop ro.product.cpu.abilist 2>/dev/null)"

ui_print "- Detecting ABI..."
mkdir -p "$MODPATH/system/bin"

selected_binary=""

if echo "$ABILIST $ABI" | grep -q "arm64-v8a" && [ -f "$MODPATH/bin/arm64-v8a/corepolicy" ]; then
    ui_print "- Installing arm64-v8a binary"
    selected_binary="$MODPATH/bin/arm64-v8a/corepolicy"
elif echo "$ABILIST $ABI" | grep -q "armeabi-v7a" && [ -f "$MODPATH/bin/armeabi-v7a/corepolicy" ]; then
    ui_print "- Installing armeabi-v7a binary"
    selected_binary="$MODPATH/bin/armeabi-v7a/corepolicy"
else
    ui_print "! Error: Compatible CoreShift Policy binary not found for this device."
    exit 1
fi

mv "$selected_binary" "$MODPATH/system/bin/corepolicy"
rm -rf "$MODPATH/bin"

ui_print "- Setting permissions..."
set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm "$MODPATH/system/bin/corepolicy" 0 0 0755
set_perm "$MODPATH/customize.sh" 0 0 0755
set_perm "$MODPATH/service.sh" 0 0 0755
set_perm "$MODPATH/uninstall.sh" 0 0 0755

ui_print "- Preparing working directory..."
mkdir -p "$WORK_DIR"
chmod 0755 "$WORK_DIR"

if [ -f "$CONFIG_FILE" ]; then
    ui_print "- Preserving existing corepolicy.conf"
else
    ui_print "- Installing default corepolicy.conf"
    cp "$MODPATH/corepolicy.conf" "$CONFIG_FILE"
    chmod 0644 "$CONFIG_FILE"
fi

rm -f "$MODPATH/corepolicy.conf"

ui_print "- Installation complete!"
