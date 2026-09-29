package com.elon.app.sociallinks

import android.content.Context
import android.content.SharedPreferences
import android.view.Gravity
import android.view.ViewTreeObserver
import android.widget.Button
import android.widget.LinearLayout
import com.elon.app.MobileColors
import com.elon.app.R
import com.google.android.material.snackbar.Snackbar

/** Fixed app/reader order; changing the choice also opens it. Chat still owns long-press/selection. */
internal class SocialMediaOpenBar(
    context: Context,
    private val platform: SocialMediaPlatform,
    private val current: () -> SocialLink,
    private val navigate: (SocialMediaOpenMode) -> Unit,
) : LinearLayout(context) {
    private val palette = MobileColors(context)
    private val store = SocialMediaOpenPreferences(context)
    private val readerOffered = SocialMediaOpenPolicy.readerOffered(platform)
    private var notice: Pair<SocialMediaOpenMode, SocialMediaOpenMode>? = null
    private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
    private fun button(label: String, semanticId: Int, mode: SocialMediaOpenMode) =
        Button(context, null, android.R.attr.borderlessButtonStyle).apply {
            id = semanticId; tag = if (mode == SocialMediaOpenMode.APP) "social-media-open-app" else "social-media-open-reader"
            text = label; textSize = 14f; isAllCaps = false; minimumWidth = 0; minWidth = 0; minLines = 2
            minHeight = dp(56); gravity = Gravity.CENTER; setPadding(dp(4), dp(4), dp(4), dp(4))
            setTextColor(palette.primary); setOnClickListener { select(mode) }
        }
    val appButton = button(platform.appLabel, R.id.social_media_open_app, SocialMediaOpenMode.APP)
    val readerButton = button("在一龙内打开", R.id.social_media_open_reader, SocialMediaOpenMode.READER).apply {
        visibility = if (readerOffered) VISIBLE else GONE
    }
    private val preferenceListener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ -> refresh() }
    private val focusListener = ViewTreeObserver.OnWindowFocusChangeListener { focus ->
        if (focus) { refresh(); showNotice() }
    }
    init {
        tag = "social-media-open-bar"; orientation = HORIZONTAL; isBaselineAligned = false; gravity = Gravity.CENTER_VERTICAL
        addView(appButton, LayoutParams(0, -2, 1f)); addView(readerButton, LayoutParams(0, -2, 1f))
        refresh()
    }
    private fun select(mode: SocialMediaOpenMode) {
        if (!store.current()) return
        if (mode == SocialMediaOpenMode.READER && !readerOffered) return
        if (mode == SocialMediaOpenMode.APP && SocialMediaAppLauncher.availability(context, platform) != SocialMediaAppLauncher.Availability.AVAILABLE) {
            refresh(); return
        }
        val before = store.get(platform)
        store.set(platform, mode)
        if (before != mode && readerOffered) notice = before to mode
        refresh(); navigate(mode)
    }
    private fun showNotice() {
        val (before, selected) = notice ?: return
        notice = null
        if (!store.current() || store.get(platform) != selected) return
        val destination = if (selected == SocialMediaOpenMode.APP) platform.appLabel else "一龙内"
        Snackbar.make(this, "${current().site}链接已默认在${destination}打开", Snackbar.LENGTH_LONG)
            .setAction("撤销") { if (store.current() && store.get(platform) == selected) store.set(platform, before) }.show()
    }
    internal fun refresh() {
        val availability = SocialMediaAppLauncher.availability(context, platform)
        val available = availability == SocialMediaAppLauncher.Availability.AVAILABLE
        val mode = SocialMediaOpenPolicy.requestedMode(platform, SocialMediaOpenPolicy.effective(store.get(platform), available))
        val state = when (availability) {
            SocialMediaAppLauncher.Availability.MISSING -> "未安装"
            SocialMediaAppLauncher.Availability.DISABLED -> "不可用"
            else -> if (mode == SocialMediaOpenMode.APP && readerOffered) "默认" else ""
        }
        appButton.text = platform.appLabel + if (state.isNotEmpty()) "\n$state" else ""
        readerButton.text = "在一龙内打开" + if (mode == SocialMediaOpenMode.READER) "\n默认" else ""
        appButton.isEnabled = available && store.current(); readerButton.isEnabled = store.current()
        appButton.isSelected = mode == SocialMediaOpenMode.APP; readerButton.isSelected = mode == SocialMediaOpenMode.READER
        appButton.setTextColor(if (appButton.isEnabled) palette.primary else palette.muted)
    }
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val required = maxOf(appButton.paint.measureText(platform.appLabel), readerButton.paint.measureText("在一龙内打开")) + dp(12)
        val next = if (readerOffered && width / 2f < required) VERTICAL else HORIZONTAL
        if (orientation != next) {
            orientation = next
            for (button in listOf(appButton, readerButton)) button.layoutParams =
                if (next == HORIZONTAL) LayoutParams(0, -2, 1f) else LayoutParams(-1, -2)
        }
        super.onMeasure(widthMeasureSpec, heightMeasureSpec)
    }
    override fun onAttachedToWindow() {
        super.onAttachedToWindow(); store.preferences.registerOnSharedPreferenceChangeListener(preferenceListener)
        viewTreeObserver.addOnWindowFocusChangeListener(focusListener); refresh()
    }
    override fun onDetachedFromWindow() {
        store.preferences.unregisterOnSharedPreferenceChangeListener(preferenceListener)
        if (viewTreeObserver.isAlive) viewTreeObserver.removeOnWindowFocusChangeListener(focusListener)
        super.onDetachedFromWindow()
    }
}
