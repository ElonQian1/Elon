package com.elon.app.chatgptweb

import android.graphics.Color
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.ImageButton
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.R
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal class ChatGptNativeFeatureHubController(
    private val activity: AppCompatActivity,
    private val trigger: ImageButton,
    private val onRequestFeatures: () -> Unit,
    private val onSelectFeature: (String) -> Unit,
    private val onDismissNavigation: () -> Unit,
    private val onOpenOfficial: () -> Unit,
) {
    private val content = LayoutInflater.from(activity).inflate(
        R.layout.sheet_chatgpt_features,
        null,
        false,
    )
    private val dialog = BottomSheetDialog(activity)
    private val closeButton = content.findViewById<ImageButton>(R.id.chatGptFeatureClose)
    private val officialButton = content.findViewById<Button>(R.id.chatGptFeatureOfficial)
    private val stateView = content.findViewById<TextView>(R.id.chatGptFeatureState)
    private val listView = content.findViewById<RecyclerView>(R.id.chatGptFeatureList)
    private val listAdapter = FeatureAdapter(::selectFeature, ::kindLabel)
    private var bridgeReady = false
    private var navigationSupported = false
    private var suppressDismissCommand = false

    init {
        dialog.setContentView(content)
        dialog.window?.navigationBarColor = ContextCompat.getColor(activity, R.color.elon_bg_chrome)
        dialog.setOnShowListener { expandSheet() }
        dialog.setOnDismissListener {
            if (!suppressDismissCommand) onDismissNavigation()
        }
        trigger.setOnClickListener { show() }
        closeButton.setOnClickListener { dialog.dismiss() }
        officialButton.setOnClickListener {
            dismissWithoutCommand()
            onOpenOfficial()
        }
        listView.layoutManager = LinearLayoutManager(activity)
        listView.adapter = listAdapter
        updateTrigger()
    }

    fun renderCapabilities(capabilities: ChatGptWebCapabilities) {
        navigationSupported = capabilities.supports(ChatGptWebCapabilityId.FEATURE_NAVIGATION)
        updateTrigger()
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        bridgeReady = state == ChatGptWebPageAdapter.State.READY
        updateTrigger()
        if (!bridgeReady && dialog.isShowing) dismissWithoutCommand()
    }

    fun render(features: List<ChatGptWebFeature>) {
        listAdapter.submit(features)
        val hasFeatures = features.isNotEmpty()
        listView.visibility = if (hasFeatures) View.VISIBLE else View.GONE
        stateView.visibility = if (hasFeatures) View.GONE else View.VISIBLE
        if (!hasFeatures) stateView.setText(R.string.chatgpt_features_empty)
        listView.isEnabled = true
    }

    fun onCommandResult(event: ChatGptWebEvent.CommandResult): Boolean = when (event.action) {
        "list_navigation", "collect_navigation" -> {
            if (!event.ok) showState(event.detail.ifBlank {
                activity.getString(R.string.chatgpt_features_failed)
            })
            true
        }
        "select_navigation" -> {
            if (event.ok) dismissWithoutCommand()
            else showState(event.detail.ifBlank {
                activity.getString(R.string.chatgpt_features_failed)
            })
            true
        }
        "dismiss_navigation" -> true
        else -> false
    }

    fun dispose() {
        trigger.setOnClickListener(null)
        suppressDismissCommand = true
        if (dialog.isShowing) dialog.dismiss()
        listView.adapter = null
    }

    private fun show() {
        if (!bridgeReady || !navigationSupported) return
        listAdapter.submit(emptyList())
        listView.visibility = View.GONE
        showState(activity.getString(R.string.chatgpt_features_loading))
        dialog.show()
        onRequestFeatures()
    }

    private fun selectFeature(feature: ChatGptWebFeature) {
        if (feature.selected) {
            dialog.dismiss()
            return
        }
        listView.isEnabled = false
        showState(activity.getString(R.string.chatgpt_features_opening))
        onSelectFeature(feature.id)
    }

    private fun showState(message: String) {
        stateView.text = message
        stateView.visibility = View.VISIBLE
        listView.visibility = View.GONE
    }

    private fun dismissWithoutCommand() {
        suppressDismissCommand = true
        if (dialog.isShowing) dialog.dismiss()
        suppressDismissCommand = false
    }

    private fun updateTrigger() {
        val enabled = bridgeReady && navigationSupported
        trigger.isEnabled = enabled
        trigger.alpha = if (enabled) 1f else DISABLED_ALPHA
    }

    private fun kindLabel(kind: String): String = activity.getString(
        when (kind) {
            "library" -> R.string.chatgpt_feature_kind_library
            "tasks" -> R.string.chatgpt_feature_kind_tasks
            "projects" -> R.string.chatgpt_feature_kind_projects
            "gpts" -> R.string.chatgpt_feature_kind_gpts
            "memory" -> R.string.chatgpt_feature_kind_memory
            "apps" -> R.string.chatgpt_feature_kind_apps
            "settings" -> R.string.chatgpt_feature_kind_settings
            "more" -> R.string.chatgpt_feature_kind_more
            else -> R.string.chatgpt_feature_kind_navigation
        },
    )

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

    private class FeatureAdapter(
        private val onClick: (ChatGptWebFeature) -> Unit,
        private val kindLabel: (String) -> String,
    ) : RecyclerView.Adapter<FeatureViewHolder>() {
        private var items: List<ChatGptWebFeature> = emptyList()

        fun submit(next: List<ChatGptWebFeature>) {
            items = next
            notifyDataSetChanged()
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): FeatureViewHolder =
            FeatureViewHolder(
                LayoutInflater.from(parent.context).inflate(
                    R.layout.item_chatgpt_feature,
                    parent,
                    false,
                ),
                onClick,
                kindLabel,
            )

        override fun onBindViewHolder(holder: FeatureViewHolder, position: Int) {
            holder.bind(items[position])
        }

        override fun getItemCount(): Int = items.size
    }

    private class FeatureViewHolder(
        itemView: View,
        private val onClick: (ChatGptWebFeature) -> Unit,
        private val kindLabel: (String) -> String,
    ) : RecyclerView.ViewHolder(itemView) {
        private val label = itemView.findViewById<TextView>(R.id.chatGptFeatureLabel)
        private val kind = itemView.findViewById<TextView>(R.id.chatGptFeatureKind)
        private val selected = itemView.findViewById<TextView>(R.id.chatGptFeatureSelected)

        fun bind(item: ChatGptWebFeature) {
            label.text = item.label
            kind.text = kindLabel(item.kind)
            selected.visibility = if (item.selected) View.VISIBLE else View.GONE
            itemView.contentDescription = ChatGptNativeNavigationSelector.feature(item)
            itemView.tag = item.id
            itemView.setOnClickListener { onClick(item) }
        }
    }

    private companion object {
        const val DISABLED_ALPHA = 0.4f
        const val SHEET_HEIGHT_RATIO = 0.78f
        const val MAX_SHEET_HEIGHT_DP = 680
    }
}
