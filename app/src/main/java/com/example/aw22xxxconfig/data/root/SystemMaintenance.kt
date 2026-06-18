package com.example.aw22xxxconfig.data.root

import android.os.Build
import com.example.aw22xxxconfig.BuildConstants

object SystemMaintenance {
    val debloatPackages = listOf(
        "com.android.theme.icon_pack.filled.settings",
        "org.lineageos.recorder",
        "org.calyxos.bellis",
        "com.android.theme.icon.teardrop",
        "com.android.theme.icon_pack.rounded.settings",
        "com.android.theme.icon_pack.kai.android",
        "com.android.calllogbackup",
        "com.android.systemui.accessibility.accessibilitymenu",
        "com.android.dreams.phototable",
        "com.android.theme.icon_pack.rounded.android",
        "com.android.theme.icon_pack.kai.settings",
        "com.android.dreams.basic",
        "com.android.devicediagnostics.auto_generated_rro_product__",
        "com.android.theme.icon_pack.sam.launcher",
        "com.android.bookmarkprovider",
        "com.android.apps.tag",
        "com.android.DeviceAsWebcam",
        "com.android.printservice.recommendation",
        "com.android.emergency.auto_generated_rro_product__",
        "com.android.managedprovisioning",
        "com.android.emergency",
        "com.android.theme.icon.vessel",
        "org.lineageos.overlay.font.rubik",
        "com.android.internal.display.cutout.emulation.double",
        "com.android.theme.font.notoserifsource",
        "org.lineageos.overlay.font.lato",
        "com.android.theme.icon.pebble",
        "com.android.role.notes.enabled",
        "com.android.theme.icon_pack.circular.settings",
        "com.android.devicediagnostics",
        "com.android.theme.icon_pack.victor.systemui",
        "com.android.avatarpicker",
        "com.android.theme.icon.roundedrect",
        "com.stevesoltys.seedvault",
        "org.calyxos.backup.contacts",
        "com.android.wallpaperbackup",
        "com.android.egg",
        "com.android.theme.icon_pack.circular.android",
        "com.android.theme.icon.square",
        "com.android.theme.icon_pack.victor.launcher",
        "com.android.stk",
        "com.android.internal.display.cutout.emulation.hole",
        "com.android.theme.icon.squircle",
        "com.android.internal.display.cutout.emulation.tall",
        "com.android.theme.icon_pack.kai.launcher",
        "com.android.theme.icon_pack.circular.launcher",
        "com.android.theme.icon_pack.filled.launcher",
        "com.android.theme.icon_pack.rounded.launcher",
        "org.lineageos.profiles",
        "org.lineageos.backgrounds",
        "com.android.providers.downloads.ui",
        "com.android.theme.icon_pack.victor.android",
        "com.android.theme.icon_pack.circular.systemui",
        "org.lineageos.twelve",
        "com.android.theme.icon_pack.sam.settings",
        "com.android.simappdialog",
        "com.android.wallpaper.livepicker",
        "com.android.theme.icon_pack.kai.systemui",
        "com.android.theme.icon.taperedrect",
        "org.lineageos.jelly",
        "com.android.internal.display.cutout.emulation.waterfall",
        "com.dsi.ant.server",
        "com.android.cellbroadcastreceiver",
        "com.android.theme.icon_pack.sam.systemui",
        "com.android.systemui.plugin.globalactions.wallet",
        "com.android.theme.icon_pack.filled.systemui",
        "com.android.htmlviewer",
        "org.lineageos.camelot",
        "com.android.theme.icon_pack.rounded.systemui",
        "com.android.providers.userdictionary",
        "com.android.internal.display.cutout.emulation.corner",
        "com.android.theme.icon_pack.filled.android",
        "com.android.theme.icon_pack.victor.settings",
        "com.android.dynsystem",
        "com.android.inputdevices",
        "com.android.theme.icon_pack.sam.android",
        "com.tencent.soter.soterserver",
        "com.android.healthconnect.controller",
        "org.lineageos.aperture",
        "com.google.android.feedback",
        "com.android.bips",
        "com.google.android.marvin.talkback",
        "com.google.android.apps.wellbeing",
        "com.android.cellbroadcastreceiver.module",
        "com.google.android.projection.gearhead",
        "org.lineageos.audiofx",
    )

    const val keyboardPackage = "com.android.inputmethod.latin"

    fun checkDebloatTargets(includeKeyboard: Boolean): Result<String> = RootShell.exec(
        buildPackageLoopCommand(includeKeyboard, restore = false, checkOnly = true)
    )

    fun runDebloat(includeKeyboard: Boolean): Result<String> = RootShell.exec(
        buildPackageLoopCommand(includeKeyboard, restore = false, checkOnly = false)
    )

    fun restoreDebloat(includeKeyboard: Boolean): Result<String> = RootShell.exec(
        buildPackageLoopCommand(includeKeyboard, restore = true, checkOnly = false)
    )

    private fun buildPackageLoopCommand(includeKeyboard: Boolean, restore: Boolean, checkOnly: Boolean): String {
        val packages = if (includeKeyboard) debloatPackages + keyboardPackage else debloatPackages
        val body = packages.joinToString("\n")
        val action = when {
            checkOnly -> "check"
            restore -> "restore"
            else -> "debloat"
        }
        return """
TOTAL=0; OK=0; FAIL=0; FOUND=0
printf '%s\n' 'Mora system cleanup: $action'
while IFS= read -r PKG; do
  [ -z "${'$'}PKG" ] && continue
  TOTAL=${'$'}((TOTAL + 1))
  if pm list packages "${'$'}PKG" | grep -q "package:${'$'}PKG"; then
    FOUND=${'$'}((FOUND + 1))
  else
    echo "[${'$'}TOTAL] ${'$'}PKG — not installed"
    continue
  fi
  if [ "$checkOnly" = "true" ]; then
    STATE=${'$'}(pm list packages -d "${'$'}PKG" | grep -q "package:${'$'}PKG" && echo disabled || echo enabled)
    echo "[${'$'}TOTAL] ${'$'}PKG — ${'$'}STATE"
    OK=${'$'}((OK + 1))
  elif [ "$restore" = "true" ]; then
    pm enable --user 0 "${'$'}PKG" >/dev/null 2>&1
    RC=${'$'}?
    if [ "${'$'}RC" -eq 0 ]; then OK=${'$'}((OK + 1)); echo "[${'$'}TOTAL] ${'$'}PKG — enabled"; else FAIL=${'$'}((FAIL + 1)); echo "[${'$'}TOTAL] ${'$'}PKG — enable failed"; fi
  else
    pm clear "${'$'}PKG" >/dev/null 2>&1
    am force-stop "${'$'}PKG" >/dev/null 2>&1
    pm disable-user --user 0 "${'$'}PKG" >/dev/null 2>&1
    RC=${'$'}?
    if [ "${'$'}RC" -eq 0 ]; then OK=${'$'}((OK + 1)); echo "[${'$'}TOTAL] ${'$'}PKG — disabled"; else FAIL=${'$'}((FAIL + 1)); echo "[${'$'}TOTAL] ${'$'}PKG — disable failed"; fi
  fi
done <<'EOF'
$body
EOF
echo
echo "Total: ${'$'}TOTAL"
echo "Found: ${'$'}FOUND"
echo "OK: ${'$'}OK"
echo "Failed: ${'$'}FAIL"
"""
    }
}

object VendorBootFlasher {
    private const val SLOT_A = "/dev/block/by-name/vendor_boot_a"
    private const val SLOT_B = "/dev/block/by-name/vendor_boot_b"

    fun isSupportedDevice(): Boolean {
        val device = Build.DEVICE.orEmpty()
        val model = Build.MODEL.orEmpty()
        val joined = "$device $model".lowercase()
        val isNx769j = device.equals("NX769J", ignoreCase = true) || model.contains("NX769J", ignoreCase = true)
        val blocked = listOf("9s", "pro+", "pro plus", "plus").any { joined.contains(it) }
        return isNx769j && !blocked
    }

    fun flash(): Result<String> {
        if (!isSupportedDevice()) {
            return Result.failure(IllegalStateException("vendor_boot flashing is only allowed on Red Magic 9 Pro / NX769J"))
        }
        val image = BuildConstants.VENDOR_BOOT_IMAGE_PATH
        val command = """
set -e
echo "Checking device..."
DEV=${'$'}(getprop ro.product.device)
MODEL=${'$'}(getprop ro.product.model)
NAME="${'$'}DEV ${'$'}MODEL"
case "${'$'}DEV" in
  NX769J|nx769j) ;;
  *) echo "Unsupported device: ${'$'}NAME"; exit 10 ;;
esac
case "${'$'}NAME" in
  *9S*|*9s*|*Pro+*|*pro+*|*Plus*|*plus*) echo "Blocked model: ${'$'}NAME"; exit 14 ;;
esac
echo "Checking image: $image"
[ -s "$image" ] || { echo "Image not found or empty: $image"; exit 11; }
[ -b "$SLOT_A" ] || { echo "Missing $SLOT_A"; exit 12; }
[ -b "$SLOT_B" ] || { echo "Missing $SLOT_B"; exit 13; }
echo "Flashing vendor_boot_a..."
dd if="$image" of="$SLOT_A" bs=4M conv=fsync
echo "Flashing vendor_boot_b..."
dd if="$image" of="$SLOT_B" bs=4M conv=fsync
sync
echo "Done. Rebooting..."
reboot
"""
        return RootShell.exec(command)
    }
}
