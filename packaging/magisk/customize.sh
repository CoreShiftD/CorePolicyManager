#!/system/bin/sh

ui_print "***********************************"
ui_print "* CoreShift Policy                 *"
ui_print "***********************************"

ABI="$(getprop ro.product.cpu.abi)"
ABILIST="$(getprop ro.product.cpu.abilist)"

mkdir -p "$MODPATH/system/bin"

select_abi_dir() {
    if echo "$ABILIST $ABI" | grep -q "arm64-v8a" && [ -d "$MODPATH/bin/arm64-v8a" ]; then
        echo "$MODPATH/bin/arm64-v8a"
        return 0
    fi

    if echo "$ABILIST $ABI" | grep -q "armeabi-v7a" && [ -d "$MODPATH/bin/armeabi-v7a" ]; then
        echo "$MODPATH/bin/armeabi-v7a"
        return 0
    fi

    return 1
}

BIN_DIR="$(select_abi_dir)" || {
    ui_print "! Error: compatible binary directory not found for this device."
    exit 1
}

for bin in corepolicy inoi_refresh_idle utensil-poker utensil-webui; do
    if [ ! -f "$BIN_DIR/$bin" ]; then
        ui_print "! Error: missing $bin for this ABI."
        exit 1
    fi

    ui_print "- Installing $bin"
    mv "$BIN_DIR/$bin" "$MODPATH/system/bin/$bin"
done

rm -rf "$MODPATH/bin"

ui_print "- Setting permissions..."
set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm "$MODPATH/system/bin/corepolicy" 0 0 0755
set_perm "$MODPATH/system/bin/inoi_refresh_idle" 0 0 0755
set_perm "$MODPATH/system/bin/utensil-poker" 0 0 0755
set_perm "$MODPATH/system/bin/utensil-webui" 0 0 0755
set_perm "$MODPATH/customize.sh" 0 0 0755
set_perm "$MODPATH/service.sh" 0 0 0755
set_perm "$MODPATH/uninstall.sh" 0 0 0755

ui_print "- Creating working directory..."
mkdir -p /data/local/tmp/coreshift
chmod 0755 /data/local/tmp/coreshift
mkdir -p /data/local/tmp/utensil
chmod 0755 /data/local/tmp/utensil

ui_print "- Installation complete!"
