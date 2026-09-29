package com.elon.app.sociallinks

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

internal object SocialMediaAppLauncher {
    enum class Availability { AVAILABLE, MISSING, DISABLED }
    private val redirects by lazy {
        OkHttpClient.Builder().followRedirects(false).followSslRedirects(false)
            .connectTimeout(3, TimeUnit.SECONDS).readTimeout(3, TimeUnit.SECONDS)
            .callTimeout(3, TimeUnit.SECONDS).build()
    }
    fun availability(context: Context, platform: SocialMediaPlatform): Availability = try {
        val info = context.packageManager.getApplicationInfo(platform.packageName, 0)
        if (info.enabled && info.flags and android.content.pm.ApplicationInfo.FLAG_SUSPENDED == 0) Availability.AVAILABLE else Availability.DISABLED
    } catch (_: PackageManager.NameNotFoundException) { Availability.MISSING }
      catch (_: SecurityException) { Availability.DISABLED }
    fun intent(platform: SocialMediaPlatform, url: String) = Intent(Intent.ACTION_VIEW, Uri.parse(url))
        .addCategory(Intent.CATEGORY_BROWSABLE).setPackage(platform.packageName)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    fun open(context: Context, item: SocialLink): Boolean {
        val platform = SocialMediaOpenPolicy.platform(item) ?: return false
        for (target in SocialMediaOpenPolicy.candidates(item)) {
            val request = intent(platform, target)
            val info = context.packageManager.resolveActivity(request, PackageManager.MATCH_DEFAULT_ONLY)?.activityInfo
            if (info?.packageName != platform.packageName || !info.exported || !info.enabled) continue
            try { context.startActivity(request); return true }
            catch (_: android.content.ActivityNotFoundException) { /* Try the next validated content URL. */ }
            catch (_: SecurityException) { /* Disabled or denied by the device policy. */ }
        }
        return false
    }
    /** Only short-link redirects, no HTML/JS evaluation, automatic redirects, credentials or persistent cache. */
    fun resolveShort(item: SocialLink): SocialLink? {
        if (!SocialMediaOpenPolicy.isShort(item)) return null
        var url = item.url
        val visited = mutableSetOf<String>()
        val deadline = android.os.SystemClock.elapsedRealtime() + 6500
        repeat(4) {
            if (!visited.add(url) || android.os.SystemClock.elapsedRealtime() >= deadline) return null
            val remaining = deadline - android.os.SystemClock.elapsedRealtime()
            val call = redirects.newCall(Request.Builder().url(url).get().build())
            call.timeout().timeout(remaining.coerceAtLeast(1), TimeUnit.MILLISECONDS)
            val next = call.execute().use { response ->
                if (response.code !in setOf(301, 302, 303, 307, 308)) return null
                SocialMediaOpenPolicy.redirect(item, url, response.header("Location") ?: return null)
            } ?: return null
            SocialMediaOpenPolicy.destination(item, next)?.let { return it }
            url = next
        }
        return null
    }
}
