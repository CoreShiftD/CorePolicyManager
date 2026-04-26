#!/system/bin/sh

ui_print "***********************************"
ui_print "*        CoreShift Policy         *"
ui_print "***********************************"

ABI="$(getprop ro.product.cpu.abi)"
ABILIST="$(getprop ro.product.cpu.abilist)"

ui_print "- Detecting ABI..."
mkdir -p "$MODPATH/system/bin"

if echo "$ABILIST$ABI" | grep -q "arm64-v8a" && [ -f "$MODPATH/bin/arm64-v8a/corepolicy" ]; then
    ui_print "- Installing arm64-v8a binary"
    mv "$MODPATH/bin/arm64-v8a/corepolicy" "$MODPATH/system/bin/corepolicy"
elif echo "$ABILIST$ABI" | grep -q "armeabi-v7a" && [ -f "$MODPATH/bin/armeabi-v7a/corepolicy" ]; then
    ui_print "- Installing armeabi-v7a binary"
    mv "$MODPATH/bin/armeabi-v7a/corepolicy" "$MODPATH/system/bin/corepolicy"
else
    ui_print "! Error: Compatible CoreShift Policy binary not found for this device."
    exit 1
fi

ui_print "- Setting permissions..."
set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm "$MODPATH/system/bin/corepolicy" 0 0 0755
set_perm "$MODPATH/bin/arm64-v8a/corepolicy" 0 0 0755
set_perm "$MODPATH/bin/armeabi-v7a/corepolicy" 0 0 0755
set_perm "$MODPATH/service.sh" 0 0 0755
set_perm "$MODPATH/uninstall.sh" 0 0 0755

ui_print "- Creating working directory..."
mkdir -p /data/local/tmp/coreshift
chmod 0755 /data/local/tmp/coreshift

install_json_if_missing() {
    src="$1"
    dest="$2"
    name="$3"

    if [ -f "$dest" ]; then
        ui_print "- Preserving existing $name"
        return 0
    fi

    if [ ! -f "$src" ]; then
        ui_print "! Warning: bundled $name not found"
        return 0
    fi

    ui_print "- Installing default $name"
    cp "$src" "$dest"
    chmod 0644 "$dest"
}

ui_print "- Installing CoreShift data directory"
install_json_if_missing \
    "$MODPATH/profiles_category.json" \
    "/data/local/tmp/coreshift/profiles_category.json" \
    "profiles_category.json"
install_json_if_missing \
    "$MODPATH/foreground_blacklist.json" \
    "/data/local/tmp/coreshift/foreground_blacklist.json" \
    "foreground_blacklist.json"
install_json_if_missing \
    "$MODPATH/profile_rules.json" \
    "/data/local/tmp/coreshift/profile_rules.json" \
    "profile_rules.json"

ui_print "- Installation complete!"
