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

    fun appForPackage(packageName: String): InstalledApp? = _installedApps.value.firstOrNull { it.packageName == packageName }

    companion object {
        fun factory(context: Context): ViewModelProvider.Factory = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T = MoraViewModel(context.applicationContext) as T
        }
    }
}
