package com.example.aw22xxxconfig

import android.annotation.SuppressLint
import android.os.Bundle
import android.view.View
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import com.example.aw22xxxconfig.databinding.ActivityMainBinding
import org.json.JSONObject
import java.net.URLConnection

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var config: RootConfigManager

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        config = RootConfigManager(this)

        val web: WebView = binding.web
        web.settings.javaScriptEnabled = true
        web.settings.domStorageEnabled = true
        web.settings.allowFileAccess = true
        web.settings.allowContentAccess = true

        // We load the UI under http://127.0.0.1:1004 via interception (same-origin with the daemon API),
        // so we don't need file:// cross-origin exemptions.

        web.webChromeClient = WebChromeClient()
        web.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? {
                val url = request.url ?: return null
                if (url.host != "127.0.0.1") return null
                if (url.port != 1004) return null

                val path = url.encodedPath ?: return null

                // Serve the UI from app assets under /ui/*.
                // This keeps the UI same-origin with the daemon API while the daemon itself stays API-only
                // for real browsers.
                if (path == "/" || path == "/ui" || path == "/ui/") {
                    return asset("index.html")
                }

                if (path.startsWith("/ui/")) {
                    val name = path.removePrefix("/ui/")
                        .substringAfterLast('/') // basic traversal protection
                        .trim()
                    if (name.isEmpty()) return asset("index.html")
                    return asset(name)
                }

                return null
            }

            override fun onPageFinished(view: WebView, url: String) {
                super.onPageFinished(view, url)
                injectApiConfig()
            }
        }

        // Bridge for fallback and debugging.
        web.addJavascriptInterface(ConfigBridge(config), "AndroidConfig")

        binding.retryButton.setOnClickListener {
            requestRootAndLoad()
        }

        requestRootAndLoad()
    }

    private fun requestRootAndLoad() {
        // Show gate while requesting root access (this triggers Magisk prompt on first run)
        binding.web.visibility = View.GONE
        binding.rootGate.visibility = View.VISIBLE
        binding.rootTitle.setText(R.string.root_required_title)
        binding.rootBody.text = getString(R.string.root_required_body)

        Thread {
            val ok = config.testRoot()
            runOnUiThread {
                if (ok) {
                    binding.rootGate.visibility = View.GONE
                    binding.web.visibility = View.VISIBLE
                    // Load UI under the same origin as the daemon API.
                    binding.web.loadUrl("http://127.0.0.1:1004/ui/index.html")
                } else {
                    binding.web.visibility = View.GONE
                    binding.rootGate.visibility = View.VISIBLE
                }
            }
        }.start()
    }

    private fun asset(name: String): WebResourceResponse? {
        return try {
            val mime = URLConnection.guessContentTypeFromName(name) ?: when {
                name.endsWith(".png", ignoreCase = true) -> "image/png"
                name.endsWith(".html", ignoreCase = true) -> "text/html"
                name.endsWith(".css", ignoreCase = true) -> "text/css"
                name.endsWith(".js", ignoreCase = true) -> "application/javascript"
                else -> "application/octet-stream"
            }
            WebResourceResponse(mime, "utf-8", assets.open(name))
        } catch (_: Throwable) {
            null
        }
    }

    private fun injectApiConfig() {
        val baseUrl = "http://127.0.0.1:1004"
        val token = config.readApiToken()

        // Inject into the page so JS can send auth headers.
        val js = """
            window.__MORA = {
              baseUrl: ${JSONObject.quote(baseUrl)},
              token: ${JSONObject.quote(token)}
            };
            if (window.__mora_onAndroidReady) window.__mora_onAndroidReady();
        """.trimIndent()

        binding.web.evaluateJavascript(js, null)
    }
}
