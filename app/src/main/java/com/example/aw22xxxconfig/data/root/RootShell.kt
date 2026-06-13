package com.example.aw22xxxconfig.data.root

import java.io.BufferedReader
import java.io.InputStreamReader

object RootShell {
    fun exec(command: String): Result<String> = runCatching {
        val process = ProcessBuilder("su", "-c", command)
            .redirectErrorStream(true)
            .start()
        val output = BufferedReader(InputStreamReader(process.inputStream)).use { it.readText() }
        val exitCode = process.waitFor()
        if (exitCode != 0) error(output.ifBlank { "root command failed: $command" })
        output.trim()
    }
}
