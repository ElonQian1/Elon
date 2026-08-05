package com.elon.app.update

import android.content.Context
import android.content.SharedPreferences

internal class AppUpdateStore(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun latestVersion(): AppUpdateVersion? =
        prefs.getString(KEY_LATEST_VERSION, null)?.let(AppUpdateVersion::parse)

    fun saveLatestVersion(version: AppUpdateVersion) {
        prefs.edit().putString(KEY_LATEST_VERSION, version.toJson()).apply()
    }

    fun snapshot(): AppUpdateSnapshot? =
        AppUpdateSnapshot.parse(prefs.getString(KEY_SNAPSHOT, null))

    fun saveSnapshot(snapshot: AppUpdateSnapshot) {
        prefs.edit().putString(KEY_SNAPSHOT, snapshot.toJson()).apply()
    }

    fun pruneInstalledOrOlder(installedVersionCode: Int): Boolean {
        val latestCode = latestVersion()?.versionCode
        val snapshotCode = snapshot()?.versionCode
        val decision = appUpdatePruneDecision(installedVersionCode, latestCode, snapshotCode)
        if (!decision.changed) return false
        prefs.edit().apply {
            if (decision.clearLatestVersion) remove(KEY_LATEST_VERSION)
            if (decision.clearSnapshot) remove(KEY_SNAPSHOT)
            if (prefs.getInt(KEY_DISMISSED_CODE, 0) <= installedVersionCode) {
                remove(KEY_DISMISSED_CODE)
                remove(KEY_DISMISSED_AT)
            }
            if (prefs.getInt(KEY_PROMPT_CODE, 0) <= installedVersionCode) {
                remove(KEY_PROMPT_CODE)
                remove(KEY_PROMPT_AT)
            }
        }.apply()
        return true
    }

    fun clearSnapshot(versionCode: Int): Boolean {
        if (snapshot()?.versionCode != versionCode) return false
        prefs.edit().remove(KEY_SNAPSHOT).apply()
        return true
    }

    fun recordAvailable(version: AppUpdateVersion) {
        saveLatestVersion(version)
        val current = snapshot()
        if (current?.versionCode == version.versionCode &&
            current.phase in setOf(
                AppUpdatePhase.QUEUED,
                AppUpdatePhase.DOWNLOADING,
                AppUpdatePhase.VERIFYING,
                AppUpdatePhase.READY,
            )
        ) {
            return
        }
        saveSnapshot(
            AppUpdateSnapshot(
                versionCode = version.versionCode,
                versionName = version.versionName,
                phase = AppUpdatePhase.AVAILABLE,
                totalBytes = version.fileSize,
            )
        )
    }

    fun markCheckAttempt(now: Long = System.currentTimeMillis()) {
        prefs.edit().putLong(KEY_LAST_CHECK, now).apply()
    }

    fun shouldCheck(now: Long, minimumIntervalMs: Long): Boolean =
        now - prefs.getLong(KEY_LAST_CHECK, 0L) >= minimumIntervalMs

    fun dismiss(versionCode: Int, now: Long = System.currentTimeMillis()) {
        prefs.edit()
            .putInt(KEY_DISMISSED_CODE, versionCode)
            .putLong(KEY_DISMISSED_AT, now)
            .apply()
    }

    fun isDismissed(versionCode: Int, now: Long, expiryMs: Long): Boolean =
        prefs.getInt(KEY_DISMISSED_CODE, 0) == versionCode &&
            now - prefs.getLong(KEY_DISMISSED_AT, 0L) < expiryMs

    fun markPrompted(versionCode: Int, now: Long = System.currentTimeMillis()) {
        prefs.edit()
            .putInt(KEY_PROMPT_CODE, versionCode)
            .putLong(KEY_PROMPT_AT, now)
            .apply()
    }

    fun wasPromptedRecently(versionCode: Int, now: Long, dedupeMs: Long): Boolean =
        prefs.getInt(KEY_PROMPT_CODE, 0) == versionCode &&
            now - prefs.getLong(KEY_PROMPT_AT, 0L) < dedupeMs

    fun observeSnapshot(listener: (AppUpdateSnapshot?) -> Unit): () -> Unit {
        val observer = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
            if (key == KEY_SNAPSHOT) listener(snapshot())
        }
        prefs.registerOnSharedPreferenceChangeListener(observer)
        listener(snapshot())
        return { prefs.unregisterOnSharedPreferenceChangeListener(observer) }
    }

    companion object {
        private const val PREFS_NAME = "elon_update"
        private const val KEY_LATEST_VERSION = "latest_version_json"
        private const val KEY_SNAPSHOT = "download_snapshot_json"
        private const val KEY_DISMISSED_CODE = "dismissed_code"
        private const val KEY_DISMISSED_AT = "dismissed_at"
        private const val KEY_LAST_CHECK = "last_check"
        private const val KEY_PROMPT_CODE = "realtime_prompt_code"
        private const val KEY_PROMPT_AT = "realtime_prompt_at"
    }
}
