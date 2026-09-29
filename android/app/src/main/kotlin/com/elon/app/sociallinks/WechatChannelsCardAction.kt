package com.elon.app.sociallinks

import android.content.Context

/** Fresh validated handoff; the shared card router owns lifecycle and failure behaviour. */
internal object WechatChannelsCardAction {
    fun resolve(context: Context, item: SocialLink): String {
        require(WechatChannelsPolicy.isChannels(item.url))
        val value = SocialLinkPreviewApi.wechatHandoff(context, item.url)
        val source = value.optString("source_url")
        val target = value.optString("launch_url")
        check(value.optInt("schema") == 1 && value.optLong("expires_at_ms") > System.currentTimeMillis() + 1000 &&
            WechatChannelsPolicy.sameContent(item.url, source) && WechatChannelsPolicy.allows(source, target))
        return target
    }
}
