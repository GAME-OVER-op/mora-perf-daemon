package com.example.aw22xxxconfig.data.root

import com.example.aw22xxxconfig.BuildConstants
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

object TokenReader {
    @Serializable
    private data class ConfigToken(val api_token: String = "")

    fun readToken(): Result<String> = runCatching {
        val raw = RootShell.exec("cat ${BuildConstants.CONFIG_PATH}").getOrThrow()
        val token = Json { ignoreUnknownKeys = true }.decodeFromString<ConfigToken>(raw).api_token.trim()
        require(token.isNotEmpty()) { "api token is empty" }
        token
    }
}
