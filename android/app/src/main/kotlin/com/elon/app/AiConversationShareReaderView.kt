package com.elon.app

import android.app.Dialog
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.WindowManager
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

internal class AiConversationShareReaderView(
    private val activity: AppCompatActivity,
    card: AiConversationShareCard,
    onDiscuss: (() -> Unit)?,
    private val onClose: (AiConversationShareReaderPosition?) -> Unit,
    retry: (() -> Unit)?,
) {
    val dialog = Dialog(activity).apply {
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        setOwnerActivity(activity)
        setCanceledOnTouchOutside(false)
    }
    private val root = LinearLayout(activity).apply {
        id = R.id.ai_conversation_share_reader
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(color(R.color.elon_bg_app))
    }
    private val layout = LinearLayoutManager(activity)
    val list = RecyclerView(activity).apply {
        id = R.id.ai_conversation_share_list
        layoutManager = layout
        itemAnimator = null
        clipToPadding = false
        setPadding(dp(12), dp(8), dp(12), dp(12))
    }
    private val title = text(card.title, 16f).apply { maxLines = 2; ellipsize = TextUtils.TruncateAt.END }
    private val subtitle = text("", 12f, R.color.elon_text_secondary)
    private val avatar = text("", 13f).apply { gravity = Gravity.CENTER }
    private val search = EditText(activity).apply {
        id = R.id.ai_conversation_share_search
        hint = activity.getString(R.string.ai_conversation_share_search)
        contentDescription = hint
        textSize = 15f
        setSingleLine(true)
        setTextColor(color(R.color.elon_text_primary))
        setHintTextColor(color(R.color.elon_text_tertiary))
        background = ColorDrawable(Color.TRANSPARENT)
        setPadding(dp(12), 0, dp(4), 0)
        imeOptions = android.view.inputmethod.EditorInfo.IME_ACTION_SEARCH
    }
    private val resultLabel = text("", 12f, R.color.elon_text_secondary).apply { gravity = Gravity.CENTER }
    private val results = LinearLayout(activity).apply { gravity = Gravity.CENTER_VERTICAL; visibility = View.GONE }
    private val previous = icon(R.drawable.ic_project_space_chevron_right, R.string.ai_conversation_share_previous) { move(-1) }
        .apply { rotation = 180f }
    private val next = icon(R.drawable.ic_project_space_chevron_right, R.string.ai_conversation_share_next) { move(1) }
    private val status = text("", 15f, R.color.elon_text_secondary).apply {
        id = R.id.ai_conversation_share_status
        gravity = Gravity.CENTER
        setPadding(dp(24), dp(20), dp(24), dp(20))
        accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
    }
    private val progress = ProgressBar(activity).apply { isIndeterminate = true }
    private val retryButton = text(activity.getString(R.string.ai_conversation_share_retry), 15f).apply {
        id = R.id.ai_conversation_share_retry
        gravity = Gravity.CENTER
        minHeight = dp(48)
        setOnClickListener { retry?.invoke() }
    }
    private val notice = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER
        addView(progress, LinearLayout.LayoutParams(dp(40), dp(40)))
        addView(status, LinearLayout.LayoutParams(-1, -2))
        addView(retryButton, LinearLayout.LayoutParams(-1, dp(48)))
    }
    private var messages = emptyList<ChatMessage>()
    private var finder = AiConversationShareReaderSearch(messages)
    private var beforeSearch: AiConversationShareReaderPosition? = null
    private val continueButton = text("继续私聊", 15f).apply {
        gravity = Gravity.CENTER
        minHeight = dp(48)
        visibility = View.GONE
        contentDescription = "ai-conversation-share-continue-private"
    }

    init {
        val header = LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(4), dp(12), dp(4))
            setBackgroundColor(color(R.color.elon_bg_chrome))
        }
        header.addView(icon(R.drawable.ic_project_space_chevron_right, R.string.ai_conversation_share_back) {
            dialog.dismiss()
        }.apply { id = R.id.ai_conversation_share_back; rotation = 180f })
        header.addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(title)
            addView(subtitle)
        }, LinearLayout.LayoutParams(0, -2, 1f))
        header.addView(avatar, LinearLayout.LayoutParams(dp(32), dp(32)).apply { marginStart = dp(8) })
        root.addView(header, LinearLayout.LayoutParams(-1, -2))
        val searchRow = LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(28).toFloat()
                setColor(color(R.color.elon_surface_search))
            }
            addView(search, LinearLayout.LayoutParams(0, -1, 1f))
            addView(icon(android.R.drawable.ic_menu_close_clear_cancel, R.string.ai_conversation_share_clear_search) {
                search.setText("")
            })
        }
        root.addView(searchRow, LinearLayout.LayoutParams(-1, dp(56)).apply {
            setMargins(dp(16), dp(12), dp(16), dp(8))
        })
        results.addView(resultLabel, LinearLayout.LayoutParams(0, dp(48), 1f))
        results.addView(previous)
        results.addView(next)
        root.addView(results)
        root.addView(FrameLayout(activity).apply {
            addView(list, FrameLayout.LayoutParams(-1, -1))
            addView(notice, FrameLayout.LayoutParams(-1, -1))
        }, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(continueButton, LinearLayout.LayoutParams(-1, dp(48)))
        if (onDiscuss != null) root.addView(text(activity.getString(R.string.ai_conversation_share_discuss), 15f).apply {
            id = R.id.ai_conversation_share_discuss
            gravity = Gravity.CENTER
            minHeight = dp(48)
            setCompoundDrawablesWithIntrinsicBounds(R.drawable.ic_popup_group, 0, 0, 0)
            compoundDrawablePadding = dp(8)
            setPadding(dp(20), 0, dp(20), 0)
            setOnClickListener { dialog.dismiss(); onDiscuss() }
        }, LinearLayout.LayoutParams(-1, dp(48)))
        dialog.setContentView(root)
        dialog.setOnDismissListener { onClose(position()); clear() }
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val safe = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime())
            view.setPadding(safe.left, safe.top, safe.right, safe.bottom)
            insets
        }
        search.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
            override fun afterTextChanged(value: Editable?) {
                if (finder.query.isEmpty() && !value.isNullOrBlank()) beforeSearch = position()
                val match = finder.update(value?.toString().orEmpty())
                if (finder.query.isEmpty()) { restore(beforeSearch); beforeSearch = null }
                else match?.let { layout.scrollToPositionWithOffset(it, 0) }
                renderMatches()
            }
        })
        search.setOnEditorActionListener { _, _, _ -> move(1); true }
        renderHeader(card)
        showNotice(R.string.ai_conversation_share_loading, loading = true)
    }

    fun show() {
        dialog.show()
        dialog.window?.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            clearFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
            setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
            setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE or WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN)
            WindowCompat.setDecorFitsSystemWindows(this, false)
        }
        root.isFocusableInTouchMode = true
        root.requestFocus()
        ViewCompat.requestApplyInsets(root)
    }

    fun render(snapshot: AiConversationShareSnapshot, rows: List<ChatMessage>, adapter: ChatAdapter,
               position: AiConversationShareReaderPosition?) {
        messages = rows
        finder = AiConversationShareReaderSearch(rows)
        list.adapter = adapter
        renderHeader(snapshot.card, rows.firstOrNull { it.role == "user" })
        notice.visibility = if (rows.isEmpty()) View.VISIBLE else View.GONE
        list.visibility = if (rows.isEmpty()) View.GONE else View.VISIBLE
        if (rows.isEmpty()) showNotice(R.string.ai_conversation_share_empty)
        restore(position)
        finder.update(search.text?.toString().orEmpty())
        renderMatches()
    }

    fun showNotice(message: Int, loading: Boolean = false, canRetry: Boolean = false) {
        notice.visibility = View.VISIBLE
        status.setText(message)
        progress.visibility = if (loading) View.VISIBLE else View.GONE
        retryButton.visibility = if (canRetry) View.VISIBLE else View.GONE
    }

    fun clear() {
        setContinueAction(null)
        list.adapter = null
        messages = emptyList()
        finder = AiConversationShareReaderSearch(messages)
        beforeSearch = null
        list.visibility = View.GONE
        search.setText("")
        avatar.background = null
        avatar.text = ""
    }

    fun setContinueAction(action: (() -> Unit)?) {
        continueButton.visibility = if (action == null) View.GONE else View.VISIBLE
        continueButton.setOnClickListener { action?.invoke() }
    }

    fun messageChanged(index: Int) {
        val anchor = position()
        list.adapter?.notifyItemChanged(index)
        restore(anchor)
    }

    fun position(): AiConversationShareReaderPosition? {
        val index = layout.findFirstVisibleItemPosition()
        val message = messages.getOrNull(index) ?: return null
        return AiConversationShareReaderPosition(message.id ?: "$index",
            (layout.findViewByPosition(index)?.top ?: list.paddingTop) - list.paddingTop)
    }

    private fun restore(position: AiConversationShareReaderPosition?) {
        position ?: return
        val index = messages.indexOfFirst { it.id == position.messageId }
        if (index >= 0) layout.scrollToPositionWithOffset(index, position.offset)
    }

    private fun renderHeader(card: AiConversationShareCard, sender: ChatMessage? = null) {
        title.text = card.title
        subtitle.text = activity.getString(R.string.ai_conversation_share_source, card.senderName, card.provider) +
            " · " + activity.getString(R.string.ai_conversation_share_message_count, card.messageCount)
        bindSenderAvatar(avatar, (sender ?: ChatMessage("user", "")).copy(senderLabel = card.senderName))
    }

    private fun move(delta: Int) {
        finder.move(delta)?.let { layout.scrollToPositionWithOffset(it, 0) }
        renderMatches()
    }

    private fun renderMatches() {
        results.visibility = if (finder.query.isEmpty()) View.GONE else View.VISIBLE
        resultLabel.text = if (finder.matches.isEmpty()) activity.getString(R.string.ai_conversation_share_no_matches)
        else activity.getString(R.string.ai_conversation_share_matches, finder.current + 1, finder.matches.size)
        listOf(previous, next).forEach { it.isEnabled = finder.matches.isNotEmpty(); it.alpha = if (it.isEnabled) 1f else 0.4f }
    }

    private fun text(value: String, size: Float, token: Int = R.color.elon_text_primary) = TextView(activity).apply {
        text = value; textSize = size; setTextColor(color(token))
    }
    private fun icon(drawable: Int, label: Int, action: () -> Unit) = ImageButton(activity).apply {
        layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
        setImageResource(drawable)
        setColorFilter(color(R.color.elon_text_primary))
        setPadding(dp(14), dp(14), dp(14), dp(14))
        background = ColorDrawable(Color.TRANSPARENT)
        contentDescription = activity.getString(label)
        tooltipText = contentDescription
        setOnClickListener { action() }
    }
    private fun color(token: Int) = ContextCompat.getColor(activity, token)
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
}
