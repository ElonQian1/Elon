package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.icu.text.BreakIterator
import android.net.Uri
import android.os.SystemClock
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.animation.PathInterpolator
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import com.elon.app.databinding.ActivityMainBinding

internal class MainEmojiActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val activeConversation: () -> AppConversation,
    private val emojiPanel: () -> LinearLayout?,
    private val emojiButton: () -> ImageButton?,
    private val collapseAttachmentPanel: () -> Unit,
    private val setKeyboardOverlayMode: (Boolean) -> Unit,
    private val replacementPanelHeight: (Int) -> Int,
    private val addPendingEmojiAttachment: (CustomEmojiItem) -> Boolean,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit
) {
    var isOpen = false
        private set

    private lateinit var packImportLauncher: ActivityResultLauncher<PickVisualMediaRequest>
    private lateinit var gifImportLauncher: ActivityResultLauncher<Array<String>>
    private var currentTab = EmojiTab.BUILT_IN
    private var contentFrame: FrameLayout? = null
    private var builtInTab: TextView? = null
    private var customTab: TextView? = null
    private var panelAnimator: ValueAnimator? = null
    private var keyboardVisibleOverEmojiPanel = false
    private var keyboardOverlayRequestedAt = 0L
    private val panelInterpolator = PathInterpolator(0.2f, 0f, 0f, 1f)

    private enum class EmojiButtonState {
        CLOSED,
        PANEL,
        KEYBOARD_OVER_PANEL
    }

    fun setupEmojiLaunchers() {
        packImportLauncher = activity.registerForActivityResult(
            ActivityResultContracts.PickMultipleVisualMedia(MAX_CUSTOM_EMOJI_IMPORT)
        ) { uris ->
            if (uris.isEmpty()) return@registerForActivityResult
            importCustomEmojis(uris, "表情包")
        }
        gifImportLauncher = activity.registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri == null) {
                Toast.makeText(activity, "已取消选择 GIF", Toast.LENGTH_SHORT).show()
                return@registerForActivityResult
            }
            runCatching {
                activity.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            importCustomEmojis(listOf(uri), "GIF")
        }
    }

    fun buildEmojiPanel(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0
            )
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor("#1E1E1E"))
            visibility = View.GONE
            addView(createTabRow())
            contentFrame = FrameLayout(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    0,
                    1f
                )
            }
            addView(requireNotNull(contentFrame))
            selectTab(EmojiTab.BUILT_IN)
        }
    }

    fun toggleEmojiPanel() {
        if (isOpen) toggleKeyboardOverEmojiPanel() else expandEmojiPanel()
    }

    fun isKeyboardVisibleOverEmojiPanel(): Boolean {
        val waitingForKeyboard = SystemClock.uptimeMillis() - keyboardOverlayRequestedAt < KEYBOARD_OVERLAY_GRACE_MS
        if (keyboardVisibleOverEmojiPanel && !waitingForKeyboard && !isKeyboardVisible()) {
            keyboardVisibleOverEmojiPanel = false
            keyboardOverlayRequestedAt = 0L
            if (isOpen) {
                updateEmojiButton(EmojiButtonState.PANEL)
            }
        }
        return isOpen && keyboardVisibleOverEmojiPanel
    }

    fun expandEmojiPanel() {
        if (activeConversation().ended) return
        collapseAttachmentPanel()
        val panel = emojiPanel() ?: return
        panelAnimator?.cancel()
        isOpen = true
        keyboardVisibleOverEmojiPanel = false
        keyboardOverlayRequestedAt = 0L
        panel.visibility = View.VISIBLE
        setPanelHeight(panel, 0)
        panel.alpha = 0f
        panel.translationY = dp(10).toFloat()
        hideKeyboard()
        animateEmojiPanel(panel, expand = true)
        updateEmojiButton(EmojiButtonState.PANEL)
    }

    fun collapseEmojiPanel() {
        collapseEmojiPanel(showKeyboard = false)
    }

    private fun collapseEmojiPanel(showKeyboard: Boolean) {
        val panel = emojiPanel() ?: return
        val wasOpen = isOpen || panel.visibility == View.VISIBLE
        if (keyboardVisibleOverEmojiPanel) hideKeyboard()
        keyboardVisibleOverEmojiPanel = false
        keyboardOverlayRequestedAt = 0L
        setKeyboardOverlayMode(false)
        isOpen = false
        if (wasOpen) {
            animateEmojiPanel(panel, expand = false, matchKeyboardTransition = showKeyboard)
            updateEmojiButton(EmojiButtonState.CLOSED)
        }
    }

    fun collapseEmojiPanelForBack(): Boolean {
        if (!isOpen && emojiPanel()?.visibility != View.VISIBLE) return false
        collapseEmojiPanel()
        return true
    }

    private fun animateEmojiPanel(
        panel: LinearLayout,
        expand: Boolean,
        matchKeyboardTransition: Boolean = false
    ) {
        val targetHeight = if (expand) targetEmojiPanelHeight() else 0
        val startHeight = maxOf(
            (panel.layoutParams?.height ?: 0).coerceAtLeast(0),
            panel.height.coerceAtLeast(0)
        )
        if (startHeight == targetHeight) {
            if (!expand) panel.visibility = View.GONE
            panel.alpha = 1f
            panel.translationY = 0f
            return
        }
        var cancelled = false
        panel.setLayerType(View.LAYER_TYPE_HARDWARE, null)
        panelAnimator?.cancel()
        panelAnimator = ValueAnimator.ofInt(startHeight, targetHeight).apply {
            duration = if (expand || matchKeyboardTransition) 220L else 170L
            interpolator = panelInterpolator
            addUpdateListener { animator ->
                val fraction = animator.animatedFraction
                setPanelHeight(panel, animator.animatedValue as Int)
                panel.alpha = if (expand) fraction else 1f - fraction
                panel.translationY = if (expand) {
                    dp(10) * (1f - fraction)
                } else {
                    dp(8) * fraction
                }
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: Animator) {
                    panel.setLayerType(View.LAYER_TYPE_NONE, null)
                    if (cancelled) return
                    setPanelHeight(panel, targetHeight)
                    panel.alpha = 1f
                    panel.translationY = 0f
                    if (!expand) panel.visibility = View.GONE
                    panelAnimator = null
                }
            })
            start()
        }
    }

    private fun targetEmojiPanelHeight(): Int {
        val fallbackHeight = dp(EMOJI_PANEL_HEIGHT_DP)
        return replacementPanelHeight(fallbackHeight)
            .takeIf { it > 0 }
            ?: fallbackHeight
    }

    private fun setPanelHeight(panel: View, height: Int) {
        val params = panel.layoutParams ?: return
        if (params.height == height) return
        params.height = height
        panel.layoutParams = params
    }

    private fun createTabRow(): LinearLayout {
        val row = LinearLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), dp(8), dp(6), dp(6))
        }
        builtInTab = createTab("表情") { selectTab(EmojiTab.BUILT_IN) }
        customTab = createTab("自定义") { selectTab(EmojiTab.CUSTOM) }
        row.addView(requireNotNull(builtInTab))
        row.addView(requireNotNull(customTab))
        val tabsScroll = HorizontalScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                1f
            )
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            addView(row)
        }
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(48)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(tabsScroll)
            addView(createPanelDeleteButton())
        }
    }

    private fun createPanelDeleteButton(): ImageButton {
        return ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(52), dp(42)).apply {
                marginEnd = dp(14)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_emoji_backspace)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(9), dp(9), dp(9), dp(9))
            alpha = 0.94f
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "Delete emoji"
            setOnClickListener { deleteBeforeCursor() }
        }
    }

    private fun createTab(label: String, onClick: () -> Unit): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(82), dp(34)).apply {
                marginEnd = dp(8)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = label
            textSize = 14f
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }
    }

    private fun selectTab(tab: EmojiTab) {
        currentTab = tab
        builtInTab?.applyTabStyle(selected = tab == EmojiTab.BUILT_IN)
        customTab?.applyTabStyle(selected = tab == EmojiTab.CUSTOM)
        contentFrame?.removeAllViews()
        val content = if (tab == EmojiTab.BUILT_IN) createBuiltInContent() else createCustomContent()
        contentFrame?.addView(content)
    }

    private fun TextView.applyTabStyle(selected: Boolean) {
        background = rounded("#2B2B2B", if (selected) "#D0D0D0" else "#3A3A3A")
        setTextColor(Color.parseColor(if (selected) "#FFFFFF" else "#A8A8A8"))
    }

    private fun createBuiltInContent(): View {
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(2), dp(12), dp(10))
        }
        BUILT_IN_EMOJIS.chunked(BUILT_IN_COLUMNS).forEach { rowItems ->
            body.addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                rowItems.forEach { emoji -> addView(createEmojiTile(emoji)) }
                repeat(BUILT_IN_COLUMNS - rowItems.size) { addView(spaceTile()) }
            })
        }
        body.addView(createBuiltInControlRow())
        return ScrollView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            overScrollMode = View.OVER_SCROLL_NEVER
            addView(body)
        }
    }

    private fun createEmojiTile(emoji: String): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, dp(44), 1f)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = emoji
            textSize = 25f
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { insertEmoji(emoji) }
        }
    }

    private fun spaceTile(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, dp(44), 1f)
        }
    }

    private fun createBuiltInControlRow(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(8), 0, 0)
            addView(createControlButton("删除") { deleteBeforeCursor() })
            addView(createControlButton("发送") { binding.sendButton.performClick() })
        }
    }

    private fun createControlButton(label: String, onClick: () -> Unit): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, dp(38), 1f).apply {
                marginEnd = dp(8)
            }
            background = rounded("#2A2A2A", "#3E3E3E")
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = label
            textSize = 14f
            setTextColor(Color.WHITE)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }
    }

    private fun createCustomContent(): View {
        val items = CustomEmojiStore.load(activity)
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(2), dp(12), dp(12))
        }
        val tiles = mutableListOf<View>().apply {
            add(createImportTile("表情包", "+") { openPackImport() })
            add(createImportTile("GIF", "GIF") { openGifImport() })
            items.forEach { item -> add(createCustomEmojiTile(item)) }
        }
        tiles.chunked(CUSTOM_COLUMNS).forEach { rowItems ->
            body.addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                rowItems.forEach { addView(it) }
                repeat(CUSTOM_COLUMNS - rowItems.size) { addView(customSpaceTile()) }
            })
        }
        if (items.isEmpty()) {
            body.addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(54)
                )
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "添加外部表情包或 GIF 后，会显示在这里"
                setTextColor(Color.parseColor("#8A8A8A"))
                textSize = 13f
            })
        }
        return ScrollView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            overScrollMode = View.OVER_SCROLL_NEVER
            addView(body)
        }
    }

    private fun createImportTile(label: String, icon: String, onClick: () -> Unit): View {
        return LinearLayout(activity).apply {
            layoutParams = customTileLayoutParams()
            background = rounded("#242424", "#464646")
            gravity = Gravity.CENTER
            orientation = LinearLayout.VERTICAL
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(context).apply {
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = icon
                textSize = if (icon == "GIF") 16f else 28f
                setTextColor(Color.WHITE)
            })
            addView(TextView(context).apply {
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = label
                textSize = 12f
                setTextColor(Color.parseColor("#D0D0D0"))
            })
        }
    }

    private fun createCustomEmojiTile(item: CustomEmojiItem): View {
        return FrameLayout(activity).apply {
            layoutParams = customTileLayoutParams()
            background = rounded("#242424", "#3E3E3E")
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "自定义表情 ${item.displayName}，长按删除"
            addView(ImageView(context).apply {
                layoutParams = FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                )
                scaleType = ImageView.ScaleType.CENTER_CROP
                setPadding(dp(4), dp(4), dp(4), dp(4))
                val bitmap = CustomEmojiStore.thumbnail(activity, item)
                if (bitmap != null) setImageBitmap(bitmap) else setImageResource(android.R.drawable.ic_menu_gallery)
            })
            if (item.mimeType == "image/gif") {
                addView(TextView(context).apply {
                    layoutParams = FrameLayout.LayoutParams(dp(34), dp(18), Gravity.BOTTOM or Gravity.END).apply {
                        marginEnd = dp(4)
                        bottomMargin = dp(4)
                    }
                    background = rounded("#CC000000", "#66FFFFFF")
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    text = "GIF"
                    textSize = 10f
                    setTextColor(Color.WHITE)
                })
            }
            setOnClickListener {
                if (addPendingEmojiAttachment(item)) {
                    Toast.makeText(activity, "已添加表情到发送栏", Toast.LENGTH_SHORT).show()
                }
            }
            setOnLongClickListener {
                confirmRemoveCustomEmoji(item)
                true
            }
        }
    }

    private fun customTileLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(0, dp(72), 1f).apply {
            marginEnd = dp(8)
            bottomMargin = dp(8)
        }
    }

    private fun customSpaceTile(): View {
        return View(activity).apply { layoutParams = customTileLayoutParams() }
    }

    private fun insertEmoji(emoji: String) {
        val editable = binding.inputEdit.text
        val start = binding.inputEdit.selectionStart.coerceAtLeast(0)
        val end = binding.inputEdit.selectionEnd.coerceAtLeast(0)
        val from = minOf(start, end)
        val to = maxOf(start, end)
        editable.replace(from, to, emoji)
        binding.inputEdit.setSelection(from + emoji.length)
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun deleteBeforeCursor() {
        val editable = binding.inputEdit.text
        val start = binding.inputEdit.selectionStart
        val end = binding.inputEdit.selectionEnd
        if (start < 0 || end < 0 || editable.isEmpty()) return
        if (start != end) {
            editable.delete(minOf(start, end), maxOf(start, end))
        } else if (start > 0) {
            editable.delete(graphemeStartBeforeCursor(editable, start), start)
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun graphemeStartBeforeCursor(text: CharSequence, cursor: Int): Int {
        val iterator = BreakIterator.getCharacterInstance()
        iterator.setText(text.toString())
        val boundary = iterator.preceding(cursor)
        return if (boundary != BreakIterator.DONE) {
            boundary.coerceIn(0, cursor)
        } else {
            Character.offsetByCodePoints(text, cursor, -1)
        }
    }

    private fun openPackImport() {
        runCatching {
            packImportLauncher.launch(
                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)
            )
        }.onFailure {
            Toast.makeText(activity, "无法打开表情包选择器", Toast.LENGTH_SHORT).show()
        }
    }

    private fun openGifImport() {
        runCatching {
            gifImportLauncher.launch(arrayOf("image/gif"))
        }.onFailure {
            Toast.makeText(activity, "无法打开 GIF 选择器", Toast.LENGTH_SHORT).show()
        }
    }

    private fun importCustomEmojis(uris: List<Uri>, label: String) {
        var imported = 0
        uris.forEach { uri ->
            val name = displayNameForUri(activity, uri) ?: "$label-${System.currentTimeMillis()}"
            val result = runCatching { CustomEmojiStore.import(activity, uri, name) }
            if (result.isSuccess) imported += 1
        }
        if (imported == 0) {
            Toast.makeText(activity, "没有可添加的表情图片", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "已添加 $imported 个自定义表情", Toast.LENGTH_SHORT).show()
        selectTab(EmojiTab.CUSTOM)
    }

    private fun confirmRemoveCustomEmoji(item: CustomEmojiItem) {
        AlertDialog.Builder(activity)
            .setTitle("删除自定义表情")
            .setMessage("确定删除「${item.displayName}」吗？")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ ->
                CustomEmojiStore.remove(activity, item.id)
                selectTab(EmojiTab.CUSTOM)
            }
            .show()
    }

    private fun toggleKeyboardOverEmojiPanel() {
        if (keyboardVisibleOverEmojiPanel && !isKeyboardVisible()) {
            keyboardVisibleOverEmojiPanel = false
            keyboardOverlayRequestedAt = 0L
        }
        if (keyboardVisibleOverEmojiPanel) {
            hideKeyboard()
            keyboardVisibleOverEmojiPanel = false
            keyboardOverlayRequestedAt = 0L
            updateEmojiButton(EmojiButtonState.PANEL)
            return
        }

        val panel = emojiPanel() ?: return
        panelAnimator?.cancel()
        isOpen = true
        setKeyboardOverlayMode(true)
        panel.visibility = View.VISIBLE
        setPanelHeight(panel, targetEmojiPanelHeight())
        panel.alpha = 1f
        panel.translationY = 0f
        binding.inputEdit.requestFocus()
        keyboardOverlayRequestedAt = SystemClock.uptimeMillis()
        binding.inputEdit.post {
            val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            imm?.showSoftInput(binding.inputEdit, InputMethodManager.SHOW_IMPLICIT)
        }
        keyboardVisibleOverEmojiPanel = true
        updateEmojiButton(EmojiButtonState.KEYBOARD_OVER_PANEL)
    }

    private fun isKeyboardVisible(): Boolean {
        return ViewCompat.getRootWindowInsets(binding.root)
            ?.isVisible(WindowInsetsCompat.Type.ime()) == true
    }

    private fun hideKeyboard() {
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.inputEdit.windowToken, 0)
    }

    private fun updateEmojiButton(state: EmojiButtonState) {
        emojiButton()?.let { button ->
            button.animate().cancel()
            val active = state != EmojiButtonState.CLOSED
            val showKeyboardIcon = state == EmojiButtonState.PANEL
            button.setImageResource(
                if (showKeyboardIcon) R.drawable.ic_input_keyboard_circle else R.drawable.ic_input_emoji_circle
            )
            button.contentDescription = when (state) {
                EmojiButtonState.CLOSED -> "Open emoji"
                EmojiButtonState.PANEL -> "Show keyboard"
                EmojiButtonState.KEYBOARD_OVER_PANEL -> "Hide keyboard"
            }
            button.animate()
                .alpha(if (active) 1f else 0.92f)
                .scaleX(if (active) 1.06f else 1f)
                .scaleY(if (active) 1.06f else 1f)
                .setDuration(160L)
                .setInterpolator(panelInterpolator)
                .start()
        }
    }

    private fun rounded(color: String, stroke: String): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = dp(10).toFloat()
            setColor(Color.parseColor(color))
            setStroke(dp(1), Color.parseColor(stroke))
        }
    }

    private enum class EmojiTab { BUILT_IN, CUSTOM }

    private companion object {
        private const val EMOJI_PANEL_HEIGHT_DP = 268
        private const val BUILT_IN_COLUMNS = 8
        private const val CUSTOM_COLUMNS = 4
        private const val KEYBOARD_OVERLAY_GRACE_MS = 700L
        private const val MAX_CUSTOM_EMOJI_IMPORT = 20
        private val BUILT_IN_EMOJIS = listOf(
            "😀", "😃", "😄", "😁", "😆", "😅", "😂", "🤣",
            "😊", "😇", "🙂", "🙃", "😉", "😍", "🥰", "😘",
            "😋", "😜", "🤪", "🤔", "🤨", "😎", "🥳", "😭",
            "😤", "😡", "👍", "👎", "👏", "🙌", "🙏", "💪",
            "🔥", "✨", "🎉", "❤️", "💔", "💯", "✅", "❌",
            "🌹", "🍀", "☕", "🍺", "🎁", "🚀", "💡", "📌"
        )
    }
}
