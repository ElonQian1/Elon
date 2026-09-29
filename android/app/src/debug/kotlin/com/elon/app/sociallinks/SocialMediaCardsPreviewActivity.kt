package com.elon.app.sociallinks

import android.content.res.Configuration
import android.os.Bundle
import android.util.Log
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.MobileColors
import com.elon.app.R

/** Isolated debug-package acceptance. Fixed public samples, production views/router, no chat writes. */
class SocialMediaCardsPreviewActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        val scale = intent.getFloatExtra("font_scale", 1f).coerceIn(1f, 2f)
        applyOverrideConfiguration(Configuration().apply { fontScale = scale })
        setTheme(R.style.Theme_ElonApp)
        super.onCreate(savedInstanceState)
        val samples = listOf(
            "https://weixin.qq.com/sph/Aur6t4pfk3" to "微信视频号",
            "https://v.douyin.com/_XMEsxVKKOY/" to "抖音视频",
            "https://www.xiaohongshu.com/explore/6a6e8951000000002402c81f" to "小红书笔记",
            "https://www.bilibili.com/video/BV19eYH6NEsC/?t=80" to "B站视频",
        )
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val pad = (16 * resources.displayMetrics.density).toInt(); setPadding(pad, pad, pad, pad)
            setBackgroundColor(MobileColors(this@SocialMediaCardsPreviewActivity).surface)
        }
        val only = intent.getIntExtra("card", -1)
        val bars = mutableListOf<SocialMediaOpenBar>()
        for ((index, sample) in samples.withIndex()) {
            if (only >= 0 && only != index) continue
            val item = SocialLinkPolicy.link(sample.first)!!.copy(title = sample.second, author = "公开内容")
            val host = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
            val card = SocialLinkCardView(this, true) { SocialMediaCardAction.open(host, item) }.apply { bind(item) }
            host.addView(card)
            val bar = SocialMediaOpenBar(this, SocialMediaOpenPolicy.platform(item)!!, { item }) { mode ->
                Log.i("SocialMediaAcceptance", "platform=${SocialMediaOpenPolicy.platform(item)?.key} selected=$mode")
                SocialMediaCardAction.open(host, item, mode)
            }
            bars += bar; host.addView(bar)
            val density = resources.displayMetrics.density
            val width = minOf((280 * density).toInt(), (resources.displayMetrics.widthPixels - 104 * density).toInt().coerceAtLeast(1))
            column.addView(host, LinearLayout.LayoutParams(width, -2).apply { bottomMargin = (16 * density).toInt() })
        }
        val scroll = ScrollView(this).apply { addView(column) }
        setContentView(scroll)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(scroll) { view, insets ->
            val barsInset = insets.getInsets(androidx.core.view.WindowInsetsCompat.Type.systemBars())
            view.setPadding(barsInset.left, barsInset.top, barsInset.right, barsInset.bottom); insets
        }
        scroll.postDelayed({
            for (bar in bars) {
                val children = listOf(bar.appButton, bar.readerButton)
                check(children.all { it.height >= 48 * resources.displayMetrics.density && it.right <= bar.width && it.bottom <= bar.height })
                Log.i("SocialMediaAcceptance", "layout=passed orientation=${bar.orientation} fontScale=$scale")
            }
            if (savedInstanceState == null) when (intent.getStringExtra("choose")) {
                "app" -> bars.firstOrNull()?.appButton?.performClick()
                "reader" -> bars.firstOrNull()?.readerButton?.performClick()
                "card" -> (bars.firstOrNull()?.parent as? LinearLayout)?.getChildAt(0)?.performClick()
            }
        }, 700)
    }
}
