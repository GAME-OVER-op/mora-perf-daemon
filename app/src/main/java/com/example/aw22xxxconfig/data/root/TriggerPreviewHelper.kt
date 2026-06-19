package com.example.aw22xxxconfig.data.root

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import java.io.File

data class TriggerPreviewState(
    val running: Boolean = false,
    val leftPressed: Boolean = false,
    val rightPressed: Boolean = false,
    val error: String? = null,
)

class TriggerPreviewHelper(
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO),
) {
    private val _state = kotlinx.coroutines.flow.MutableStateFlow(TriggerPreviewState())
    val state: kotlinx.coroutines.flow.StateFlow<TriggerPreviewState> = _state

    private var sessionJob: Job? = null
    private var leftProcess: Process? = null
    private var rightProcess: Process? = null

    fun start() {
        if (sessionJob?.isActive == true) return
        sessionJob = scope.launch {
            try {
                val leftDevice = findTriggerDevice("sar0")
                val rightDevice = findTriggerDevice("sar1")
                setSystemTriggersEnabled(true)
                _state.value = TriggerPreviewState(running = true)
                val leftJob = launch { watchDevice(leftDevice, isLeft = true) }
                val rightJob = launch { watchDevice(rightDevice, isLeft = false) }
                leftJob.join()
                rightJob.join()
            } catch (t: Throwable) {
                _state.value = TriggerPreviewState(error = t.message ?: t.toString())
            } finally {
                runCatching { setSystemTriggersEnabled(false) }
                leftProcess = null
                rightProcess = null
                _state.value = TriggerPreviewState()
            }
        }
    }

    fun stop() {
        val job = sessionJob
        sessionJob = null
        scope.launch {
            runCatching { leftProcess?.destroy() }
            runCatching { rightProcess?.destroy() }
            leftProcess = null
            rightProcess = null
            if (job != null) {
                runCatching { job.cancel() }
                try {
                    job.join()
                } catch (_: Throwable) {
                }
            }
            runCatching { setSystemTriggersEnabled(false) }
            _state.value = TriggerPreviewState()
        }
    }

    private suspend fun watchDevice(devicePath: String, isLeft: Boolean) {
        val process = ProcessBuilder("su", "-c", "getevent -ql $devicePath")
            .redirectErrorStream(true)
            .start()
        if (isLeft) leftProcess = process else rightProcess = process
        process.inputStream.bufferedReader().useLines { lines ->
            lines.forEach { line ->
                parseLine(line, isLeft)
            }
        }
        process.waitFor()
    }

    private fun parseLine(line: String, isLeft: Boolean) {
        val lower = line.lowercase()
        val pressed = when {
            "key_f7" in lower || "key_f8" in lower -> when {
                " down" in lower -> true
                " up" in lower -> false
                else -> null
            }
            "abs_distance" in lower -> when {
                "00000000" in lower || lower.trim().endsWith(" 0") -> false
                else -> true
            }
            else -> null
        } ?: return

        _state.value = if (isLeft) {
            _state.value.copy(running = true, leftPressed = pressed, error = null)
        } else {
            _state.value.copy(running = true, rightPressed = pressed, error = null)
        }
    }

    private fun findTriggerDevice(marker: String): String {
        val text = File("/proc/bus/input/devices").readText()
        val block = text
            .split("\n\n")
            .firstOrNull { it.contains(marker, ignoreCase = true) }
            ?: error("Trigger device '$marker' not found")
        val event = Regex("event\\d+").find(block)?.value ?: error("Event node for '$marker' not found")
        return "/dev/input/$event"
    }

    private fun setSystemTriggersEnabled(enabled: Boolean) {
        val value = if (enabled) "1" else "0"
        val mode = if (enabled) "1" else "2"
        RootShell.exec(
            listOf(
                "settings put global nubia_parts_trigger_enable $value",
                "printf '$mode\\n' > /proc/nubia_key/sar0/mode_operation",
                "printf '$mode\\n' > /proc/nubia_key/sar1/mode_operation",
            ).joinToString("; ")
        ).getOrThrow()
    }
}
