package com.example.aw22xxxconfig

import android.webkit.JavascriptInterface

class ConfigBridge(private val manager: RootConfigManager) {

    @JavascriptInterface
    fun testRoot(): Boolean = manager.testRoot()

    // ----- Mora API helpers -----

    @JavascriptInterface
    fun getApiBaseUrl(): String = "http://127.0.0.1:1004"

    @JavascriptInterface
    fun getApiToken(): String = manager.readApiToken()

    // ----- Root-proxy HTTP (fallback for devices where WebView/network stack can't reach localhost) -----

    @JavascriptInterface
    fun proxyGet(path: String): String = manager.proxyGet(path)

    @JavascriptInterface
    fun proxyPost(path: String, body: String): String = manager.proxyPost(path, body)
}
