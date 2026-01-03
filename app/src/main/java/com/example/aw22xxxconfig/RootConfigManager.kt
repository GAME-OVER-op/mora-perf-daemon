package com.example.aw22xxxconfig

import android.content.Context
import android.content.SharedPreferences
import android.util.Base64
import com.topjohnwu.superuser.Shell
import org.json.JSONObject
import java.security.SecureRandom

class RootConfigManager(private val context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences("mora_panel", Context.MODE_PRIVATE)

    /**
     * Default mora config path (Magisk module).
     * Some builds use "daemon" vs "deamon" in the module folder name, so we probe a few options.
     */
    private val defaultPath = "/data/adb/modules/mora_perf_deamon/config/config.json"

    fun getConfigPath(): String = prefs.getString("config_path", defaultPath) ?: defaultPath

    fun setConfigPath(path: String) {
        prefs.edit().putString("config_path", path.trim()).apply()
    }

    private fun candidateConfigPaths(): List<String> {
        val p = getConfigPath()
        return listOf(
            p,
            "/data/adb/modules/mora_perf_deamon/config/config.json",
            "/data/adb/modules/mora_perf_daemon/config/config.json",
            "/data/adb/modules/mora/config/config.json",
            "/data/adb/modules/mora/config.json",
        ).distinct()
    }

    /**
     * Triggers Magisk prompt on first call.
     */
    fun testRoot(): Boolean {
        val r = Shell.cmd("id -u").exec()
        return r.isSuccess && r.out.joinToString("\n").trim() == "0"
    }

    private fun shQuote(s: String): String {
        // safe single-quote for sh: ' -> '\''
        return "'" + s.replace("'", "'\\''") + "'"
    }

    private fun readConfigAt(path: String): String {
        val cmd = "cat ${shQuote(path)} 2>/dev/null || true"
        val r = Shell.cmd(cmd).exec()
        val text = r.out.joinToString("\n")
        return if (text.isNotBlank()) text else ""
    }

    fun readConfig(): String = readConfigAt(getConfigPath())

    fun writeConfig(text: String): Boolean {
        val path = getConfigPath()

        // base64 encode to avoid quoting issues
        val b64 = Base64.encodeToString(text.toByteArray(Charsets.UTF_8), Base64.NO_WRAP)

        // Try toybox/base64 first (most Android builds have it). Then fallback to heredoc.
        val cmdB64 = "echo ${shQuote(b64)} | base64 -d > ${shQuote(path)}"
        val r1 = Shell.cmd(cmdB64).exec()
        if (r1.isSuccess) return true

        // fallback heredoc (might fail on very large configs)
        val heredoc = buildString {
            append("cat > ")
            append(shQuote(path))
            append(" <<'EOF'\n")
            append(text)
            if (!text.endsWith("\n")) append("\n")
            append("EOF\n")
        }
        val r2 = Shell.cmd(heredoc).exec()
        return r2.isSuccess
    }

    // ----- Root-proxy HTTP (fallback) -----

    fun proxyGet(path: String): String = proxyRequest("GET", path, "")

    fun proxyPost(path: String, body: String): String = proxyRequest("POST", path, body)

    private fun proxyRequest(method: String, path: String, body: String): String {
        val token = readApiToken()
        val url = "http://127.0.0.1:1004$path"

        val res = JSONObject()
        if (token.isBlank()) {
            res.put("code", 0)
            res.put("body", "")
            res.put("error", "token_missing")
            return res.toString()
        }

        val hdr1 = shQuote("X-Api-Key: $token")
        val hdr2 = shQuote("Authorization: Bearer $token")

        // We prefer curl/base64 if present. On some ROMs they might be missing from root PATH,
        // so we also try Termux curl and toybox base64 as fallbacks.
        val cmd = if (method.uppercase() == "GET") {
            "(curl -s -m 3 -H $hdr1 -H $hdr2 -o - -w '\\n__HTTP__%{http_code}' ${shQuote(url)}" +
                    " || /data/data/com.termux/files/usr/bin/curl -s -m 3 -H $hdr1 -H $hdr2 -o - -w '\\n__HTTP__%{http_code}' ${shQuote(url)})"
        } else {
            val b64 = Base64.encodeToString(body.toByteArray(Charsets.UTF_8), Base64.NO_WRAP)
            "echo ${shQuote(b64)} | (base64 -d 2>/dev/null || /system/bin/toybox base64 -d 2>/dev/null) | " +
                    "(curl -s -m 3 -H $hdr1 -H $hdr2 -H ${shQuote("Content-Type: application/json")} -X ${method.uppercase()} --data-binary @- -o - -w '\\n__HTTP__%{http_code}' ${shQuote(url)}" +
                    " || /data/data/com.termux/files/usr/bin/curl -s -m 3 -H $hdr1 -H $hdr2 -H ${shQuote("Content-Type: application/json")} -X ${method.uppercase()} --data-binary @- -o - -w '\\n__HTTP__%{http_code}' ${shQuote(url)})"
        }

        val r = Shell.cmd(cmd).exec()
        val out = r.out.joinToString("\n")

        val marker = "\n__HTTP__"
        val idx = out.lastIndexOf(marker)
        val bodyText: String
        val code: Int
        if (idx >= 0) {
            bodyText = out.substring(0, idx)
            code = out.substring(idx + marker.length).trim().toIntOrNull() ?: 0
        } else {
            bodyText = out
            code = if (r.isSuccess) 200 else 0
        }

        res.put("code", code)
        res.put("body", bodyText)
        if (!r.isSuccess && r.err.isNotEmpty()) {
            res.put("shell_error", r.err.joinToString("\n"))
        }
        return res.toString()
    }

    /**
     * Reads api_token from mora config.json (root required).
     */
    fun readApiToken(): String {
        // 1) Probe a few likely locations (module folder name differs across builds).
        for (path in candidateConfigPaths()) {
            val raw = readConfigAt(path)
            if (raw.isBlank()) continue

            // Try parse existing token.
            val t = extractToken(raw)
            if (t.isNotEmpty()) {
                if (path != getConfigPath()) setConfigPath(path)
                return t
            }

            // Token missing: generate and persist to this config.
            val generated = generateToken()
            val updated = try {
                val obj = JSONObject(raw)
                obj.put("api_token", generated)
                obj.toString(2)
            } catch (_: Throwable) {
                // If config is not valid JSON, don't try to rewrite it.
                ""
            }

            if (updated.isNotBlank()) {
                setConfigPath(path)
                if (writeConfig(updated)) {
                    return generated
                }
            }
        }

        // 2) Try discover module folder by name.
        try {
            val r = Shell.cmd("ls -1 /data/adb/modules 2>/dev/null | grep -i mora | head -n 1").exec()
            val mod = r.out.joinToString("\n").trim()
            if (mod.isNotEmpty()) {
                val path = "/data/adb/modules/$mod/config/config.json"
                val raw = readConfigAt(path)
                val t = extractToken(raw)
                if (t.isNotEmpty()) {
                    setConfigPath(path)
                    return t
                }
            }
        } catch (_: Throwable) {
            // ignore
        }

        return ""
    }

    private fun extractToken(raw: String): String {
        if (raw.isBlank()) return ""

        try {
            val obj = JSONObject(raw)
            val t = obj.optString("api_token", "").trim()
            if (t.isNotEmpty()) return t
        } catch (_: Throwable) {
            // ignore
        }

        val re = Regex("\"api_token\"\\s*:\\s*\"([^\"]+)\"")
        val m = re.find(raw)
        return m?.groupValues?.getOrNull(1)?.trim() ?: ""
    }

    private fun generateToken(): String {
        val rnd = SecureRandom()
        val bytes = ByteArray(32)
        rnd.nextBytes(bytes)
        // URL-safe token for headers, no padding/no wraps.
        return Base64.encodeToString(bytes, Base64.NO_WRAP or Base64.URL_SAFE).trim()
    }
}
