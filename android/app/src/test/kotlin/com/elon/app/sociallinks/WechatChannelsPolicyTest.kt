package com.elon.app.sociallinks

import org.junit.Assert.*
import org.junit.Test

class WechatChannelsPolicyTest {
    private val source = "https://channels.weixin.qq.com/finder-preview/pages/sph?id=example"
    private val target = "weixin://biz/finder/openFinderFeed/exportId%3Dexport%2Fabcdefgh12345678%26actionType%3D0%26entryScene%3D64"

    @Test fun onlyOfficialPreviewCanLaunchFeed() {
        assertTrue(WechatChannelsPolicy.isChannels("https://weixin.qq.com/sph/example"))
        assertTrue(WechatChannelsPolicy.allows(source, target))
        for (bad in listOf(source.replace("https:", "http:"), source.replace(".qq.com", ".qq.com.evil.test"), source.replace("https://", "https://user:pass@"), "$source&id=other", "$source#fragment", "https://weixin.qq.com/sph/example")) assertFalse(bad, WechatChannelsPolicy.allows(bad, target))
    }

    @Test fun arbitrarySchemesAndInjectedParametersAreDenied() {
        for (bad in listOf("intent://scan/#Intent;scheme=weixin;end", "file:///private", target.replace("biz/", "evil/"), target.replace("actionType%3D0", "actionType%3D1"), "$target%26actionType%3D0", "$target%26unknown%3D1", "$target#fragment", target.replace("exportId%3D", "exportId%253D"), target.replace("entryScene%3D64", "entryScene%3D-1"))) assertFalse(bad, WechatChannelsPolicy.allows(source, bad))
    }
}
