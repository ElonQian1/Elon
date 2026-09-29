package com.elon.app.sociallinks

import android.content.Context
import android.content.ContextWrapper
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.WeakHashMap

/** One navigation per explicit tap, with foreground/owner checks after asynchronous resolution. */
internal object SocialMediaCardAction {
    private val pending = WeakHashMap<AppCompatActivity, Job>()
    private val pendingUrls = WeakHashMap<AppCompatActivity, String>()
    private fun activity(context: Context): AppCompatActivity? = when (context) {
        is AppCompatActivity -> context
        is ContextWrapper -> if (context.baseContext !== context) activity(context.baseContext) else null
        else -> null
    }
    fun open(view: View, item: SocialLink, requested: SocialMediaOpenMode? = null) {
        val context = view.context
        val platform = SocialMediaOpenPolicy.platform(item)
        val preferences = SocialMediaOpenPreferences(context)
        val mode = SocialMediaOpenPolicy.requestedMode(platform,
            requested ?: platform?.let(preferences::get) ?: SocialMediaOpenMode.READER)
        val activity = activity(context)
        if (activity != null && mode == SocialMediaOpenMode.APP && pending[activity]?.isActive == true && pendingUrls[activity] == item.url) return
        activity?.let { pending.remove(it)?.cancel() }
        if (mode == SocialMediaOpenMode.READER || platform == null) {
            SocialLinkBrowserActivity.open(context, item); return
        }
        fun reader(reason: String) {
            if (!SocialMediaOpenPolicy.readerOffered(platform)) {
                Toast.makeText(context, "$reason，请重试或长按卡片复制链接到微信", Toast.LENGTH_LONG).show()
                return
            }
            Toast.makeText(context, "$reason，已在一龙内打开", Toast.LENGTH_SHORT).show()
            SocialLinkBrowserActivity.open(context, item)
        }
        if (SocialMediaAppLauncher.availability(context, platform) != SocialMediaAppLauncher.Availability.AVAILABLE) {
            reader("${platform.appLabel}未安装或不可用"); return
        }
        if (platform != SocialMediaPlatform.WECHAT && SocialMediaAppLauncher.open(context, item)) return
        if (activity == null) { reader("暂时无法跳转"); return }
        var interrupted = false
        fun current() = view.isAttachedToWindow && !activity.isFinishing && !activity.isDestroyed && !interrupted &&
            preferences.current() && activity.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)
        if (!current()) return
        val observer = object : DefaultLifecycleObserver {
            override fun onPause(owner: LifecycleOwner) { interrupted = true; pending.remove(activity)?.cancel() }
        }
        activity.lifecycle.addObserver(observer)
        Toast.makeText(activity, "正在打开${platform.appLabel}…", Toast.LENGTH_SHORT).show()
        val job = activity.lifecycleScope.launch {
            try {
                if (platform == SocialMediaPlatform.WECHAT) {
                    val target = withContext(Dispatchers.IO) { WechatChannelsCardAction.resolve(activity.applicationContext, item) }
                    if (current()) activity.startActivity(WechatChannelsHandoff.intent(target))
                } else {
                    val resolved = withContext(Dispatchers.IO) { SocialMediaAppLauncher.resolveShort(item) }
                    if (current() && (resolved == null || !SocialMediaAppLauncher.open(activity, resolved))) reader("该链接暂时无法跳转")
                }
            } catch (error: CancellationException) { throw error }
              catch (_: Exception) { if (current()) reader("暂时无法跳转") }
            finally { activity.lifecycle.removeObserver(observer) }
        }
        pending[activity] = job
        pendingUrls[activity] = item.url
        job.invokeOnCompletion { if (pending[activity] === job) { pending.remove(activity); pendingUrls.remove(activity) } }
    }
}
