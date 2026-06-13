package com.example.aw22xxxconfig.data.root

import com.example.aw22xxxconfig.BuildConstants
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class RootHttpClient(
    private val json: Json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        encodeDefaults = true
    }
) {
    fun get(path: String, token: String): String = execute("GET", path, token, null)

    fun post(path: String, token: String, body: Any? = null): String {
        val payload = body?.let { json.encodeToString(it) }
        return execute("POST", path, token, payload)
    }

    private fun execute(method: String, path: String, token: String, body: String?): String {
        val url = BuildConstants.BASE_URL.trimEnd('/') + "/" + path.trimStart('/')
        val shellEscapedUrl = squote(url)
        val authHeader = squote("Authorization: Bearer $token")
        val contentHeader = squote("Content-Type: application/json")
        val bodyArg = if (body != null) " --data ${squote(body)}" else ""
        val cmd = """
            if command -v curl >/dev/null 2>&1; then
              curl -fsS -X $method -H $authHeader ${if (body != null) "-H $contentHeader" else ""}$bodyArg $shellEscapedUrl
            elif toybox wget --help >/dev/null 2>&1; then
              if [ "$method" = "GET" ]; then
                toybox wget -qO- --header=$authHeader $shellEscapedUrl
              else
                toybox wget -qO- --method POST --header=$authHeader --header=$contentHeader --body-data=${squote(body ?: "") } $shellEscapedUrl
              fi
            elif command -v wget >/dev/null 2>&1; then
              if [ "$method" = "GET" ]; then
                wget -qO- --header=$authHeader $shellEscapedUrl
              else
                wget -qO- --method POST --header=$authHeader --header=$contentHeader --body-data=${squote(body ?: "") } $shellEscapedUrl
              fi
            else
              echo 'No curl/wget available in root shell' >&2
              exit 127
            fi
        """.trimIndent().replace("\n", "; ")
        return RootShell.exec(cmd).getOrThrow()
    }

    private fun squote(value: String): String = "'" + value.replace("'", "'\\''") + "'"
}
