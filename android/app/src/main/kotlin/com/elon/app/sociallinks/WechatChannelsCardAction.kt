package com.elon.app.sociallinks

import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.ContextWrapper
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.WeakHashMap

/** A chat tap resolves a fresh public handoff without creating a reader or background WebView. */
internal object WechatChannelsCardAction {
    private val pending = WeakHashMap<AppCompatActivity, Boolean>()
    private fun activity(context: Context): AppCompatActivity? = when (context) {
        is AppCompatActivity -> context
        is ContextWrapper -> if (context.baseContext !== context) activity(context.baseContext) else null
        else -> null
    }

    fun open(view: View, item: SocialLink) {
        if (!WechatChannelsPolicy.isChannels(item.url)) { SocialLinkBrowserActivity.open(view.context, item); return }
        val activity = activity(view.context) ?: return
        if (pending[activity] == true) return
        val owner = AuthManager.userId(activity); val server = ServerUrlManager.getActive(activity)
        var interrupted = false
        fun current() = view.isAttachedToWindow && !activity.isFinishing && !activity.isDestroyed &&
            !interrupted && activity.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED) && AuthManager.userId(activity) == owner && ServerUrlManager.getActive(activity) == server
        if (!current()) return
        pending[activity] = true
        val observer = object : DefaultLifecycleObserver { override fun onPause(owner: LifecycleOwner) { interrupted = true } }
        activity.lifecycle.addObserver(observer)
        Toast.makeText(activity, "正在打开微信…", Toast.LENGTH_SHORT).show()
        activity.lifecycleScope.launch {
            try {
                val value = withContext(Dispatchers.IO) { SocialLinkPreviewApi.wechatHandoff(activity.applicationContext, item.url) }
                if (!current()) return@launch
                val source = value.optString("source_url"); val target = value.optString("launch_url")
                check(value.optInt("schema") == 1 && value.optLong("expires_at_ms") > System.currentTimeMillis() + 1000 &&
                    WechatChannelsPolicy.sameContent(item.url, source) && WechatChannelsPolicy.allows(source, target)) { "微信跳转链接无效或已过期。" }
                activity.startActivity(WechatChannelsHandoff.intent(target))
            } catch (error: Exception) {
                if (!current()) return@launch
                AlertDialog.Builder(activity).setTitle("未能打开微信")
                    .setMessage("请确认微信已安装并联网。可以重试，或查看原网页。")
                    .setPositiveButton("重试") { _, _ -> open(view, item) }
                    .setNeutralButton("查看原网页") { _, _ -> SocialLinkBrowserActivity.open(activity, item) }
                    .setNegativeButton("复制链接") { _, _ ->
                        (activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("视频号", item.url))
                    }.show()
            } finally { pending.remove(activity); activity.lifecycle.removeObserver(observer) }
        }
    }
}
