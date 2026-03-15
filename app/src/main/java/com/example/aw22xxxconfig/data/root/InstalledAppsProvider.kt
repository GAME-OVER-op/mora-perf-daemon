package com.example.aw22xxxconfig.data.root

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import com.example.aw22xxxconfig.data.model.InstalledApp

class InstalledAppsProvider(private val context: Context) {
    fun load(): List<InstalledApp> {
        val pm = context.packageManager
        val launcherIntent = Intent(Intent.ACTION_MAIN, null).addCategory(Intent.CATEGORY_LAUNCHER)
        return pm.queryIntentActivities(launcherIntent, PackageManager.MATCH_ALL)
            .map {
                val info = it.activityInfo.applicationInfo
                InstalledApp(
                    label = pm.getApplicationLabel(info).toString(),
                    packageName = info.packageName,
                    icon = runCatching { pm.getApplicationIcon(info.packageName) }.getOrNull(),
                )
            }
            .distinctBy { it.packageName }
            .sortedBy { it.label.lowercase() }
    }
}
