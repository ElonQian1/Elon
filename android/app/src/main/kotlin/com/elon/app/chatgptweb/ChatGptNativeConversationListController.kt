package com.elon.app.chatgptweb

import android.graphics.Color
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.ImageButton
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.widget.doAfterTextChanged
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.R
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal class ChatGptNativeConversationListController(
    private val activity: AppCompatActivity,
    private val trigger: ImageButton,
    private val onRequestList: () -> Unit,
    private val onOpenConversation: (String) -> Unit,
) {
    private val content = LayoutInflater.from(activity).inflate(
        R.layout.sheet_chatgpt_conversations,
        null,
        false,
    )
    private val dialog = BottomSheetDialog(activity)
    private val closeButton = content.findViewById<ImageButton>(R.id.chatGptConversationClose)
    private val countView = content.findViewById<TextView>(R.id.chatGptConversationCount)
    private val searchView = content.findViewById<EditText>(R.id.chatGptConversationSearch)
    private val stateView = content.findViewById<TextView>(R.id.chatGptConversationState)
    private val listView = content.findViewById<RecyclerView>(R.id.chatGptConversationList)
    private val listAdapter = ConversationAdapter(::selectConversation)
    private var conversations: List<ChatGptWebConversation> = emptyList()
    private var bridgeReady = false
    private var listSupported = false

    init {
        dialog.setContentView(content)
        dialog.window?.navigationBarColor = ContextCompat.getColor(activity, R.color.elon_bg_chrome)
        dialog.setOnShowListener { expandSheet() }
        trigger.setOnClickListener { show() }
        closeButton.setOnClickListener { dialog.dismiss() }
        searchView.doAfterTextChanged { renderFilteredList() }
        listView.layoutManager = LinearLayoutManager(activity)
        listView.adapter = listAdapter
        updateTrigger()
    }

    fun renderCapabilities(capabilities: ChatGptWebCapabilities) {
        listSupported = capabilities.supports(ChatGptWebCapabilityId.CONVERSATION_LIST)
        updateTrigger()
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        bridgeReady = state == ChatGptWebPageAdapter.State.READY
        updateTrigger()
        if (!bridgeReady && dialog.isShowing) dialog.dismiss()
    }

    fun render(items: List<ChatGptWebConversation>) {
        conversations = items
        countView.text = activity.getString(R.string.chatgpt_conversations_count, items.size)
        renderFilteredList()
    }

    fun onCommandResult(event: ChatGptWebEvent.CommandResult): Boolean = when (event.action) {
        "list_conversations" -> {
            if (!event.ok) showState(event.detail.ifBlank {
                activity.getString(R.string.chatgpt_conversations_failed)
            })
            true
        }
        "open_conversation" -> {
            if (event.ok) {
                dialog.dismiss()
            } else {
                showState(event.detail.ifBlank {
                    activity.getString(R.string.chatgpt_conversations_open_failed)
                })
            }
            true
        }
        else -> false
    }

    fun dispose() {
        trigger.setOnClickListener(null)
        if (dialog.isShowing) dialog.dismiss()
        listView.adapter = null
    }

    private fun show() {
        if (!bridgeReady || !listSupported) return
        conversations = emptyList()
        listAdapter.submit(emptyList())
        countView.text = ""
        searchView.setText("")
        searchView.isEnabled = false
        showState(activity.getString(R.string.chatgpt_conversations_loading))
        dialog.show()
        onRequestList()
    }

    private fun selectConversation(conversation: ChatGptWebConversation) {
        if (conversation.active) {
            dialog.dismiss()
            return
        }
        searchView.isEnabled = false
        listView.isEnabled = false
        showState(activity.getString(R.string.chatgpt_conversations_opening))
        onOpenConversation(conversation.path)
    }

    private fun renderFilteredList() {
        if (conversations.isEmpty()) {
            listAdapter.submit(emptyList())
            showState(activity.getString(R.string.chatgpt_conversations_empty))
            searchView.isEnabled = true
            return
        }
        val filtered = ChatGptConversationFilter.apply(conversations, searchView.text.toString())
        listAdapter.submit(filtered)
        listView.visibility = if (filtered.isEmpty()) View.GONE else View.VISIBLE
        stateView.visibility = if (filtered.isEmpty()) View.VISIBLE else View.GONE
        if (filtered.isEmpty()) stateView.setText(R.string.chatgpt_conversations_no_results)
        searchView.isEnabled = true
        listView.isEnabled = true
    }

    private fun showState(message: String) {
        stateView.text = message
        stateView.visibility = View.VISIBLE
        listView.visibility = View.GONE
    }

    private fun updateTrigger() {
        val enabled = bridgeReady && listSupported
        trigger.isEnabled = enabled
        trigger.alpha = if (enabled) 1f else DISABLED_ALPHA
    }

    private fun expandSheet() {
        val bottomSheet = dialog.findViewById<View>(com.google.android.material.R.id.design_bottom_sheet)
            ?: return
        bottomSheet.setBackgroundColor(Color.TRANSPARENT)
        bottomSheet.layoutParams.height = minOf(
            (activity.resources.displayMetrics.heightPixels * SHEET_HEIGHT_RATIO).toInt(),
            dp(MAX_SHEET_HEIGHT_DP),
        )
        BottomSheetBehavior.from(bottomSheet).apply {
            state = BottomSheetBehavior.STATE_EXPANDED
            skipCollapsed = true
        }
    }

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private class ConversationAdapter(
        private val onClick: (ChatGptWebConversation) -> Unit,
    ) : RecyclerView.Adapter<ConversationViewHolder>() {
        private var items: List<ChatGptWebConversation> = emptyList()

        fun submit(next: List<ChatGptWebConversation>) {
            items = next
            notifyDataSetChanged()
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ConversationViewHolder =
            ConversationViewHolder(
                LayoutInflater.from(parent.context).inflate(
                    R.layout.item_chatgpt_conversation,
                    parent,
                    false,
                ),
                onClick,
            )

        override fun onBindViewHolder(holder: ConversationViewHolder, position: Int) {
            holder.bind(items[position])
        }

        override fun getItemCount(): Int = items.size
    }

    private class ConversationViewHolder(
        itemView: View,
        private val onClick: (ChatGptWebConversation) -> Unit,
    ) : RecyclerView.ViewHolder(itemView) {
        private val indicator = itemView.findViewById<View>(R.id.chatGptConversationActiveIndicator)
        private val title = itemView.findViewById<TextView>(R.id.chatGptConversationItemTitle)
        private val current = itemView.findViewById<TextView>(R.id.chatGptConversationCurrent)

        fun bind(item: ChatGptWebConversation) {
            title.text = item.title
            indicator.visibility = if (item.active) View.VISIBLE else View.INVISIBLE
            current.visibility = if (item.active) View.VISIBLE else View.GONE
            itemView.contentDescription = ChatGptNativeNavigationSelector.conversation(item)
            itemView.tag = item.id
            itemView.setOnClickListener { onClick(item) }
        }
    }

    private companion object {
        const val DISABLED_ALPHA = 0.4f
        const val SHEET_HEIGHT_RATIO = 0.82f
        const val MAX_SHEET_HEIGHT_DP = 680
    }
}
