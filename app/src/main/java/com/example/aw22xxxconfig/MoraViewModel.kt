package com.example.aw22xxxconfig

import android.content.Context
import android.os.Build
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.example.aw22xxxconfig.data.model.*
import com.example.aw22xxxconfig.data.repo.MoraRepository
import com.example.aw22xxxconfig.data.root.InstalledAppsProvider
import com.example.aw22xxxconfig.data.root.RootShell
import com.example.aw22xxxconfig.data.root.TokenReader
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch



data class AndroidFeatureState(
    val loading: Boolean = false,
    val doubleTapToWakeEnabled: Boolean = false,
    val rawDoubleTapToWake: String = "?",
    val rawAmbientTouchToWake: String = "?",
    val rawWakeGestureEnabled: String = "?",
    val rawDozeTapGesture: String = "?",
)

private val DEBLOAT_ITEMS = listOf(
    DebloatPackage("com.android.theme.icon_pack.filled.settings", "Filled settings icons"),
    DebloatPackage("org.lineageos.recorder", "LineageOS Recorder"),
    DebloatPackage("org.calyxos.bellis", "Bellis"),
    DebloatPackage("com.android.theme.icon.teardrop", "Teardrop icons"),
    DebloatPackage("com.android.theme.icon_pack.rounded.settings", "Rounded settings icons"),
    DebloatPackage("com.android.theme.icon_pack.kai.android", "Kai Android icons"),
    DebloatPackage("com.android.calllogbackup", "Call log backup"),
    DebloatPackage("com.android.systemui.accessibility.accessibilitymenu", "Accessibility menu"),
    DebloatPackage("com.android.dreams.phototable", "Photo table screensaver"),
    DebloatPackage("com.android.theme.icon_pack.rounded.android", "Rounded Android icons"),
    DebloatPackage("com.android.theme.icon_pack.kai.settings", "Kai settings icons"),
    DebloatPackage("com.android.dreams.basic", "Basic screensaver"),
    DebloatPackage("com.android.devicediagnostics.auto_generated_rro_product__", "Device diagnostics overlay"),
    DebloatPackage("com.android.theme.icon_pack.sam.launcher", "Sam launcher icons"),
    DebloatPackage("com.android.bookmarkprovider", "Bookmark provider"),
    DebloatPackage("com.android.apps.tag", "NFC tags"),
    DebloatPackage("com.android.DeviceAsWebcam", "Device as webcam"),
    DebloatPackage("com.android.printservice.recommendation", "Print recommendation service"),
    DebloatPackage("com.android.emergency.auto_generated_rro_product__", "Emergency overlay"),
    DebloatPackage("com.android.managedprovisioning", "Managed provisioning"),
    DebloatPackage("com.android.emergency", "Emergency"),
    DebloatPackage("com.android.theme.icon.vessel", "Vessel icons"),
    DebloatPackage("org.lineageos.overlay.font.rubik", "Rubik font overlay"),
    DebloatPackage("com.android.internal.display.cutout.emulation.double", "Double cutout emulation"),
    DebloatPackage("com.android.theme.font.notoserifsource", "Noto Serif font"),
    DebloatPackage("org.lineageos.overlay.font.lato", "Lato font overlay"),
    DebloatPackage("com.android.theme.icon.pebble", "Pebble icons"),
    DebloatPackage("com.android.role.notes.enabled", "Notes role"),
    DebloatPackage("com.android.theme.icon_pack.circular.settings", "Circular settings icons"),
    DebloatPackage("com.android.devicediagnostics", "Device diagnostics"),
    DebloatPackage("com.android.theme.icon_pack.victor.systemui", "Victor SystemUI icons"),
    DebloatPackage("com.android.avatarpicker", "Avatar picker"),
    DebloatPackage("com.android.theme.icon.roundedrect", "Rounded rectangle icons"),
    DebloatPackage("com.stevesoltys.seedvault", "Seedvault backup"),
    DebloatPackage("org.calyxos.backup.contacts", "Contacts backup"),
    DebloatPackage("com.android.wallpaperbackup", "Wallpaper backup"),
    DebloatPackage("com.android.egg", "Android Easter egg"),
    DebloatPackage("com.android.theme.icon_pack.circular.android", "Circular Android icons"),
    DebloatPackage("com.android.theme.icon.square", "Square icons"),
    DebloatPackage("com.android.theme.icon_pack.victor.launcher", "Victor launcher icons"),
    DebloatPackage("com.android.stk", "SIM Toolkit"),
    DebloatPackage("com.android.internal.display.cutout.emulation.hole", "Hole cutout emulation"),
    DebloatPackage("com.android.theme.icon.squircle", "Squircle icons"),
    DebloatPackage("com.android.internal.display.cutout.emulation.tall", "Tall cutout emulation"),
    DebloatPackage("com.android.theme.icon_pack.kai.launcher", "Kai launcher icons"),
    DebloatPackage("com.android.theme.icon_pack.circular.launcher", "Circular launcher icons"),
    DebloatPackage("com.android.theme.icon_pack.filled.launcher", "Filled launcher icons"),
    DebloatPackage("com.android.theme.icon_pack.rounded.launcher", "Rounded launcher icons"),
    DebloatPackage("org.lineageos.profiles", "LineageOS Profiles"),
    DebloatPackage("org.lineageos.backgrounds", "LineageOS Backgrounds"),
    DebloatPackage("com.android.providers.downloads.ui", "Downloads UI"),
    DebloatPackage("com.android.theme.icon_pack.victor.android", "Victor Android icons"),
    DebloatPackage("com.android.theme.icon_pack.circular.systemui", "Circular SystemUI icons"),
    DebloatPackage("org.lineageos.twelve", "LineageOS Music"),
    DebloatPackage("com.android.theme.icon_pack.sam.settings", "Sam settings icons"),
    DebloatPackage("com.android.simappdialog", "SIM app dialog"),
    DebloatPackage("com.android.wallpaper.livepicker", "Live wallpaper picker"),
    DebloatPackage("com.android.theme.icon_pack.kai.systemui", "Kai SystemUI icons"),
    DebloatPackage("com.android.theme.icon.taperedrect", "Tapered rectangle icons"),
    DebloatPackage("org.lineageos.jelly", "LineageOS Jelly browser"),
    DebloatPackage("com.android.internal.display.cutout.emulation.waterfall", "Waterfall cutout emulation"),
    DebloatPackage("com.dsi.ant.server", "ANT service"),
    DebloatPackage("com.android.cellbroadcastreceiver", "Cell broadcast receiver"),
    DebloatPackage("com.android.theme.icon_pack.sam.systemui", "Sam SystemUI icons"),
    DebloatPackage("com.android.systemui.plugin.globalactions.wallet", "Wallet power menu plugin"),
    DebloatPackage("com.android.theme.icon_pack.filled.systemui", "Filled SystemUI icons"),
    DebloatPackage("com.android.htmlviewer", "HTML Viewer"),
    DebloatPackage("org.lineageos.camelot", "LineageOS Camelot"),
    DebloatPackage("com.android.theme.icon_pack.rounded.systemui", "Rounded SystemUI icons"),
    DebloatPackage("com.android.providers.userdictionary", "User dictionary"),
    DebloatPackage("com.android.internal.display.cutout.emulation.corner", "Corner cutout emulation"),
    DebloatPackage("com.android.theme.icon_pack.filled.android", "Filled Android icons"),
    DebloatPackage("com.android.theme.icon_pack.victor.settings", "Victor settings icons"),
    DebloatPackage("com.android.dynsystem", "Dynamic System Updates"),
    DebloatPackage("com.android.inputdevices", "Input devices"),
    DebloatPackage("com.android.theme.icon_pack.sam.android", "Sam Android icons"),
    DebloatPackage("com.tencent.soter.soterserver", "Tencent Soter"),
    DebloatPackage("com.android.healthconnect.controller", "Health Connect"),
    DebloatPackage("org.lineageos.aperture", "LineageOS Aperture camera"),
    DebloatPackage("com.google.android.feedback", "Google Feedback"),
    DebloatPackage("com.android.bips", "Default print service"),
    DebloatPackage("com.google.android.marvin.talkback", "TalkBack"),
    DebloatPackage("com.google.android.apps.wellbeing", "Digital Wellbeing"),
    DebloatPackage("com.android.cellbroadcastreceiver.module", "Cell broadcast module"),
    DebloatPackage("com.google.android.projection.gearhead", "Android Auto"),
    DebloatPackage("org.lineageos.audiofx", "LineageOS AudioFX"),
)

private const val FLASH_IMAGES_DIR = "/data/adb/modules/mora_perf_deamon/images"
private const val VENDOR_BOOT_OC_IMAGE = "$FLASH_IMAGES_DIR/vendor_boot_oc.img"
private const val ORANGEFOX_RECOVERY_IMAGE = "$FLASH_IMAGES_DIR/orangefox_recovery.img"

class MoraViewModel(
    private val appContext: Context,
    private val repository: MoraRepository = MoraRepository(),
    private val installedAppsProvider: InstalledAppsProvider = InstalledAppsProvider(appContext),
) : ViewModel() {

    private val _connection = MutableStateFlow<ConnectionState>(ConnectionState.Loading)
    val connection: StateFlow<ConnectionState> = _connection.asStateFlow()

    private val _state = MutableStateFlow(StateResponse())
    val state: StateFlow<StateResponse> = _state.asStateFlow()

    private val _config = MutableStateFlow(UserConfig())
    val config: StateFlow<UserConfig> = _config.asStateFlow()

    private val _games = MutableStateFlow<List<GameEntry>>(emptyList())
    val games: StateFlow<List<GameEntry>> = _games.asStateFlow()

    private val _installedApps = MutableStateFlow<List<InstalledApp>>(emptyList())
    val installedApps: StateFlow<List<InstalledApp>> = _installedApps.asStateFlow()

    private val _message = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> = _message.asStateFlow()

    private val _androidFeatures = MutableStateFlow(AndroidFeatureState(loading = true))
    val androidFeatures: StateFlow<AndroidFeatureState> = _androidFeatures.asStateFlow()

    private val _debloatPackages = MutableStateFlow(DEBLOAT_ITEMS.map { DebloatPackageState(it) })
    val debloatPackages: StateFlow<List<DebloatPackageState>> = _debloatPackages.asStateFlow()

    private val _debloatLoading = MutableStateFlow(false)
    val debloatLoading: StateFlow<Boolean> = _debloatLoading.asStateFlow()

    private val _flashImageState = MutableStateFlow(FlashImageState())
    val flashImageState: StateFlow<FlashImageState> = _flashImageState.asStateFlow()

    private var refreshJob: Job? = null

    init {
        bootstrap()
    }

    fun clearMessage() { _message.value = null }

    fun bootstrap() {
        viewModelScope.launch {
            _connection.value = ConnectionState.Loading
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                _connection.value = ConnectionState.Error("Android 14 or newer is required")
                return@launch
            }
            val token = TokenReader.readToken().getOrElse {
                _connection.value = ConnectionState.Error("Failed to read API token through root: ${it.message}")
                return@launch
            }
            repository.connect(token)
            _installedApps.value = installedAppsProvider.load()
            runCatching {
                refreshAll()
                refreshAndroidFeatures()
                _connection.value = ConnectionState.Ready(BuildConstants.CONFIG_PATH)
                startPolling()
            }.onFailure {
                _connection.value = ConnectionState.Error("Failed to connect to daemon. Normal local connection failed and root fallback also failed: ${it.message}")
            }
        }
    }

    private fun startPolling() {
        refreshJob?.cancel()
        refreshJob = viewModelScope.launch {
            while (true) {
                runCatching { _state.value = repository.state() }
                delay(5_000)
            }
        }
    }

    suspend fun refreshAll() {
        _state.value = repository.state()
        _config.value = repository.config()
        _games.value = repository.games().games
    }

    fun refreshAndroidFeatures() {
        viewModelScope.launch {
            _androidFeatures.value = _androidFeatures.value.copy(loading = true)
            runCatching {
                val d = readSetting("secure", "double_tap_to_wake")
                val a = readSetting("global", "ambient_touch_to_wake")
                val w = readSetting("secure", "wake_gesture_enabled")
                val g = readSetting("secure", "doze_tap_gesture")
                _androidFeatures.value = AndroidFeatureState(
                    loading = false,
                    doubleTapToWakeEnabled = listOf(d, a, w, g).any { it == "1" },
                    rawDoubleTapToWake = d,
                    rawAmbientTouchToWake = a,
                    rawWakeGestureEnabled = w,
                    rawDozeTapGesture = g,
                )
            }.onFailure {
                _androidFeatures.value = _androidFeatures.value.copy(loading = false)
                _message.value = it.message
            }
        }
    }

    fun setDoubleTapToWakeEnabled(enabled: Boolean) {
        viewModelScope.launch {
            _androidFeatures.value = _androidFeatures.value.copy(loading = true)
            runCatching {
                RootShell.exec(buildDoubleTapToWakeCommand(enabled))
                refreshAndroidFeatures()
            }.onFailure {
                _androidFeatures.value = _androidFeatures.value.copy(loading = false)
                _message.value = it.message
            }
        }
    }

    private fun readSetting(namespace: String, key: String): String =
        RootShell.exec("settings get $namespace $key").getOrThrow().trim().ifBlank { "null" }

    private fun buildDoubleTapToWakeCommand(enabled: Boolean): String {
        val value = if (enabled) "1" else "0"
        return listOf(
            "settings put secure double_tap_to_wake $value",
            "settings put global ambient_touch_to_wake $value",
            "settings put secure wake_gesture_enabled $value",
            "settings put secure doze_pulse_on_double_tap $value",
            "settings put secure doze_tap_gesture $value",
            if (enabled) "settings put global ambient_tilt_to_wake 1" else null,
            "killall com.android.systemui 2>/dev/null || pkill -f com.android.systemui 2>/dev/null || true",
        ).filterNotNull().joinToString("; ")
    }

    fun refresh() {
        viewModelScope.launch {
            runCatching { refreshAll() }.onFailure { _message.value = it.message }
        }
    }

    fun setDaemonNotifications(enabled: Boolean) {
        viewModelScope.launch {
            runCatching {
                repository.setDaemonNotifications(enabled)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setBatterySaver(enabled: Boolean) {
        viewModelScope.launch {
            runCatching {
                repository.setBatterySaver(enabled)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setUsePhoneCooler(enabled: Boolean) {
        viewModelScope.launch {
            runCatching {
                repository.setUsePhoneCooler(enabled)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun saveProfiles(payload: UiSavePayload) {
        viewModelScope.launch {
            runCatching {
                repository.saveConfig(payload)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun addGame(app: InstalledApp) {
        viewModelScope.launch {
            runCatching {
                repository.addGame(GameAddPayload(packageName = app.packageName))
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun removeGame(packageName: String) {
        viewModelScope.launch {
            runCatching {
                repository.removeGame(packageName)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setGameDriver(packageName: String, enabled: Boolean) {
        viewModelScope.launch {
            runCatching {
                repository.setDriver(packageName, enabled)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setGameGpuTurbo(packageName: String, enabled: Boolean) {
        viewModelScope.launch {
            runCatching {
                repository.setGpuTurbo(packageName, enabled)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setGameFanMin(packageName: String, level: Int) {
        viewModelScope.launch {
            runCatching {
                repository.setFanMin(packageName, level)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setGameTriggers(packageName: String, triggers: TriggersConfig) {
        viewModelScope.launch {
            runCatching {
                repository.setTriggers(packageName, triggers)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setGameSplitCharge(packageName: String, splitCharge: SplitChargeConfig) {
        viewModelScope.launch {
            runCatching {
                repository.setSplitCharge(packageName, splitCharge.copy(stopBatteryPercent = splitCharge.stopBatteryPercent.coerceIn(0, 100)))
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun setGameDisableThermalLimit(packageName: String, enabled: Boolean) {
        viewModelScope.launch {
            runCatching {
                repository.setDisableThermalLimit(packageName, enabled)
                refreshAll()
            }.onFailure { _message.value = it.message }
        }
    }

    fun refreshDebloatPackages() {
        viewModelScope.launch {
            _debloatLoading.value = true
            runCatching {
                val installed = RootShell.exec("pm list packages").getOrThrow()
                    .lineSequence()
                    .mapNotNull { it.trim().removePrefix("package:").takeIf(String::isNotBlank) }
                    .toSet()
                val disabled = RootShell.exec("pm list packages -d").getOrElse { "" }
                    .lineSequence()
                    .mapNotNull { it.trim().removePrefix("package:").takeIf(String::isNotBlank) }
                    .toSet()
                _debloatPackages.value = DEBLOAT_ITEMS.map { item ->
                    DebloatPackageState(
                        item = item,
                        installed = item.packageName in installed || item.packageName in disabled,
                        enabled = item.packageName !in disabled,
                    )
                }
            }.onFailure {
                _message.value = it.message
            }
            _debloatLoading.value = false
        }
    }

    fun setDebloatPackageEnabled(packageName: String, enabled: Boolean) {
        viewModelScope.launch {
            _debloatLoading.value = true
            runCatching {
                if (enabled) {
                    RootShell.exec("pm enable ${squote(packageName)}").getOrThrow()
                } else {
                    RootShell.exec(
                        listOf(
                            "pm clear ${squote(packageName)}",
                            "am force-stop ${squote(packageName)}",
                            "pm disable-user --user 0 ${squote(packageName)}",
                        ).joinToString("; ")
                    ).getOrThrow()
                }
                refreshDebloatPackagesNow()
            }.onFailure {
                _message.value = it.message
                refreshDebloatPackagesNow(errorPackage = packageName, error = it.message)
            }
            _debloatLoading.value = false
        }
    }

    fun setAllDebloatPackagesEnabled(enabled: Boolean) {
        viewModelScope.launch {
            _debloatLoading.value = true
            runCatching {
                val targets = _debloatPackages.value.filter { it.installed && it.enabled != enabled }
                for (state in targets) {
                    val pkg = state.item.packageName
                    if (enabled) {
                        RootShell.exec("pm enable ${squote(pkg)}").getOrThrow()
                    } else {
                        RootShell.exec(
                            listOf(
                                "pm clear ${squote(pkg)}",
                                "am force-stop ${squote(pkg)}",
                                "pm disable-user --user 0 ${squote(pkg)}",
                            ).joinToString("; ")
                        ).getOrThrow()
                    }
                }
                refreshDebloatPackagesNow()
            }.onFailure {
                _message.value = it.message
                refreshDebloatPackagesNow(error = it.message)
            }
            _debloatLoading.value = false
        }
    }

    private fun refreshDebloatPackagesNow(errorPackage: String? = null, error: String? = null) {
        val installed = RootShell.exec("pm list packages").getOrElse { "" }
            .lineSequence()
            .mapNotNull { it.trim().removePrefix("package:").takeIf(String::isNotBlank) }
            .toSet()
        val disabled = RootShell.exec("pm list packages -d").getOrElse { "" }
            .lineSequence()
            .mapNotNull { it.trim().removePrefix("package:").takeIf(String::isNotBlank) }
            .toSet()
        _debloatPackages.value = DEBLOAT_ITEMS.map { item ->
            DebloatPackageState(
                item = item,
                installed = item.packageName in installed || item.packageName in disabled,
                enabled = item.packageName !in disabled,
                error = if (errorPackage == null || errorPackage == item.packageName) error else null,
            )
        }
    }

    private fun squote(value: String): String = "'" + value.replace("'", "'\\''") + "'"

    fun flashOverclockVendorBoot() {
        flashImage(
            imagePath = VENDOR_BOOT_OC_IMAGE,
            firstBlock = "/dev/block/by-name/vendor_boot_a",
            secondBlock = "/dev/block/by-name/vendor_boot_b",
            rebootCommand = "svc power reboot || reboot",
        )
    }

    fun flashOrangeFoxRecovery() {
        flashImage(
            imagePath = ORANGEFOX_RECOVERY_IMAGE,
            firstBlock = "/dev/block/by-name/recovery_a",
            secondBlock = "/dev/block/by-name/recovery_b",
            rebootCommand = "svc power reboot recovery || reboot recovery",
        )
    }

    private fun flashImage(
        imagePath: String,
        firstBlock: String,
        secondBlock: String,
        rebootCommand: String,
    ) {
        viewModelScope.launch {
            _flashImageState.value = FlashImageState(loading = true, message = "Flashing image...")
            runCatching {
                RootShell.exec(buildFlashImageCommand(imagePath, firstBlock, secondBlock, rebootCommand)).getOrThrow()
            }.onSuccess {
                _flashImageState.value = FlashImageState(
                    loading = false,
                    message = "Image flashed successfully. Rebooting in 5 seconds.",
                )
            }.onFailure {
                _flashImageState.value = FlashImageState(loading = false, message = it.message)
                _message.value = it.message
            }
        }
    }

    private fun buildFlashImageCommand(
        imagePath: String,
        firstBlock: String,
        secondBlock: String,
        rebootCommand: String,
    ): String = """
        IMG=${squote(imagePath)}
        FIRST=${squote(firstBlock)}
        SECOND=${squote(secondBlock)}
        [ -f "${'$'}IMG" ] || { echo "Image not found: ${'$'}IMG"; exit 1; }
        [ -s "${'$'}IMG" ] || { echo "Image is empty: ${'$'}IMG"; exit 1; }
        [ -b "${'$'}FIRST" ] || { echo "Block device not found: ${'$'}FIRST"; exit 1; }
        [ -b "${'$'}SECOND" ] || { echo "Block device not found: ${'$'}SECOND"; exit 1; }
        dd if="${'$'}IMG" of="${'$'}FIRST" bs=4M conv=fsync || exit 1
        dd if="${'$'}IMG" of="${'$'}SECOND" bs=4M conv=fsync || exit 1
        sync
        ( sleep 5; $rebootCommand ) >/dev/null 2>&1 &
        echo "Image flashed successfully. Rebooting in 5 seconds."
    """.trimIndent()

    fun appForPackage(packageName: String): InstalledApp? = _installedApps.value.firstOrNull { it.packageName == packageName }

    companion object {
        fun factory(context: Context): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T = MoraViewModel(context.applicationContext) as T
        }
    }
}
