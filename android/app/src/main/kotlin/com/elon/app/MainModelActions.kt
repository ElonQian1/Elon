package com.elon.app

import android.content.SharedPreferences
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.os.SystemClock
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.view.animation.PathInterpolator
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject

internal class MainModelActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val prefs: SharedPreferences,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val userIdProvider: () -> String,
    private val modelButtonShellProvider: () -> FrameLayout?,
    private val modelChevronProvider: () -> View?,
    private val inputBarContainerProvider: () -> LinearLayout?,
    private val getActionPopup: () -> PopupWindow?,
    private val setActionPopup: (PopupWindow?) -> Unit,
    private val openSettings: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    var modelOptions: List<ModelOption> = emptyList()
        private set
    var codexCliOnly = true
        private set
    var selectedAgentName: String? = null
        private set
    var currentModelLabel = "默认"
        private set

    private var modelPopup: PopupWindow? = null
    private var modelPopupShowsAbove = true
    private var lastModelPopupDismissedAt = 0L
    private val chevronInterpolator = PathInterpolator(0.2f, 0f, 0f, 1f)

    fun selectedAgentForRequest(): String? {
        return selectedAgentName?.takeIf { it.isNotBlank() }
    }

    fun restoreCachedModelSelection() {
        val label = cachedModelLabel() ?: return
        selectedAgentName = cachedModelAgentName()
        currentModelLabel = label
        updateModelButton()
    }

    fun loadModelOptions(afterLoad: (() -> Unit)? = null) {
        Thread {
            try {
                val response = http.newCall(
                    Request.Builder()
                        .url("$serverUrl/api/user/${userIdProvider()}/agent")
                        .get()
                        .build()
                ).execute()
                val body = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(body.ifBlank { "HTTP ${response.code}" })

                val json = JSONObject(body)
                val serverCodexCliOnly = json.optBoolean("codex_cli_only", false)
                val config = json.optJSONObject("config") ?: JSONObject()
                val agents = json.optJSONArray("available_agents") ?: JSONArray()
                if (serverCodexCliOnly) {
                    val options = parseModelOptions(agents, includeDefault = false)
                        .ifEmpty { listOf(ModelOption("Codex", "codex_cli", "codex")) }
                    val serverUseAgent = jsonStringOrNull(config, "use_agent")
                        ?.takeIf { configured -> options.any { it.agentName == configured } }
                    val cachedAgent = cachedModelAgentName()
                    val effectiveUseAgent = serverUseAgent ?: cachedAgent?.takeIf { cached ->
                        options.any { it.agentName == cached }
                    } ?: options.firstOrNull()?.agentName
                    val label = options.firstOrNull { it.agentName == effectiveUseAgent }?.label
                        ?: options.first().label
                    activity.runOnUiThread {
                        codexCliOnly = true
                        modelOptions = options
                        selectedAgentName = effectiveUseAgent
                        currentModelLabel = label
                        cacheModelSelection(effectiveUseAgent, currentModelLabel)
                        updateModelButton()
                        afterLoad?.invoke()
                    }
                    return@Thread
                }

                val options = parseModelOptions(agents, includeDefault = true)

                val serverUseAgent = jsonStringOrNull(config, "use_agent")
                    ?.takeIf { configured -> options.any { it.agentName == configured } }
                val customModel = jsonStringOrNull(config, "model").orEmpty()
                val customBase = jsonStringOrNull(config, "api_base").orEmpty()
                val cachedAgent = cachedModelAgentName()
                val effectiveUseAgent = serverUseAgent ?: cachedAgent?.takeIf { cached ->
                    options.any { it.agentName == cached }
                }
                val hasCustomConfig = customModel.isNotBlank() || customBase.isNotBlank()
                val label = when {
                    hasCustomConfig -> "自定义模型"
                    effectiveUseAgent != null -> options.firstOrNull { it.agentName == effectiveUseAgent }?.label ?: effectiveUseAgent
                    else -> "服务器默认"
                }
                val shouldSyncCache = serverUseAgent != null ||
                    hasCustomConfig ||
                    cachedAgent == null ||
                    effectiveUseAgent == null

                activity.runOnUiThread {
                    codexCliOnly = false
                    modelOptions = options
                    selectedAgentName = effectiveUseAgent
                    currentModelLabel = label
                    if (shouldSyncCache) {
                        cacheModelSelection(effectiveUseAgent, label)
                    }
                    updateModelButton()
                    afterLoad?.invoke()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "模型列表加载失败: ${e.message}", Toast.LENGTH_SHORT).show()
                    afterLoad?.invoke()
                }
            }
        }.start()
    }

    fun showModelPopupOrLoad() {
        if (modelPopup?.isShowing == true) {
            dismissModelPopup(animate = true)
            return
        }
        if (recentlyDismissedModelPopup()) {
            return
        }
        if (modelOptions.isEmpty()) {
            Toast.makeText(activity, "正在加载模型列表...", Toast.LENGTH_SHORT).show()
            loadModelOptions { showModelPopupOrLoad() }
            return
        }
        showModelPopup(modelButtonShellProvider() ?: binding.modelButton)
    }

    fun updateModelButton() {
        binding.modelButton.text = shortModelLabel(currentModelLabel)
        binding.modelButton.contentDescription = "选择模型：$currentModelLabel"
        modelButtonShellProvider()?.contentDescription = "选择模型：$currentModelLabel"
    }

    private fun parseModelOptions(agents: JSONArray, includeDefault: Boolean): MutableList<ModelOption> {
        val options = mutableListOf<ModelOption>()
        if (includeDefault) {
            options.add(ModelOption("服务器默认", null, ""))
        }
        for (i in 0 until agents.length()) {
            val item = agents.getJSONObject(i)
            val name = item.optString("name", "")
            val model = item.optString("model", "")
            val provider = item.optString("provider", "")
            val label = displayModelLabel(provider, model, item.optString("label", ""))
            if (name.isNotBlank()) {
                options.add(ModelOption(label, name, provider))
            }
        }
        return options
    }

    private fun cacheModelSelection(agentName: String?, label: String) {
        prefs.edit().apply {
            if (agentName.isNullOrBlank()) remove(PREF_SELECTED_AGENT)
            else putString(PREF_SELECTED_AGENT, agentName)
            putString(PREF_SELECTED_MODEL_LABEL, label)
        }.apply()
    }

    private fun cachedModelAgentName(): String? {
        return prefs.getString(PREF_SELECTED_AGENT, null)
            ?.trim()
            ?.takeIf { it.isNotBlank() && it != "null" }
    }

    private fun cachedModelLabel(): String? {
        return prefs.getString(PREF_SELECTED_MODEL_LABEL, null)
            ?.trim()
            ?.takeIf { it.isNotBlank() && it != "null" }
    }

    private fun showModelPopup(anchor: View) {
        getActionPopup()?.takeIf { it !== modelPopup }?.dismiss()
        modelPopup?.takeIf { it.isShowing }?.dismiss()
        modelPopup = null

        val selectableOptions = modelOptions.ifEmpty { listOf(ModelOption(currentModelLabel, selectedAgentName, "")) }
        val showCustomRow = !codexCliOnly

        // ── 构建带分组信息的行项目（section header + option rows）──────────────────
        val items = mutableListOf<PopupRowItem>()
        // "服务器默认" 不分组，直接排最前
        selectableOptions.filter { it.agentName == null }.forEach { items.add(PopupRowItem.Option(it)) }
        // 其余按 provider 分组
        val grouped = selectableOptions
            .filter { it.agentName != null }
            .groupByTo(linkedMapOf()) { opt -> providerGroupTitle(opt.provider) }
        grouped.forEach { (header, opts) ->
            if (opts.isNotEmpty()) {
                items.add(PopupRowItem.Header(header))
                opts.forEach { items.add(PopupRowItem.Option(it)) }
            }
        }

        val headerHeight = dp(30)
        val rowHeight = dp(52)
        val availablePopupWidth = (activity.resources.displayMetrics.widthPixels - dp(24)).coerceAtLeast(dp(176))
        val popupWidth = dp(296).coerceAtMost(availablePopupWidth)
        val arrowHeight = dp(8)

        // 计算总高度（header 行更矮）
        var contentHeight = 0
        var dividerCount = 0
        items.forEachIndexed { idx, item ->
            contentHeight += if (item is PopupRowItem.Header) headerHeight else rowHeight
            // header 上方不加 divider；option 与下一个 option 之间加 divider
            val next = items.getOrNull(idx + 1)
            if (item is PopupRowItem.Option && next is PopupRowItem.Option) dividerCount++
        }
        if (showCustomRow) {
            if (items.isNotEmpty()) dividerCount++
            contentHeight += rowHeight
        }
        val panelHeight = (contentHeight + dividerCount).coerceAtMost(dp(380))
        val totalHeight = panelHeight + arrowHeight

        val root = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, totalHeight)
            alpha = 0f
            scaleX = 0.97f
            scaleY = 0.97f
        }

        val panel = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        val scroll = android.widget.ScrollView(activity).apply {
            isVerticalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            background = GradientDrawable().apply {
                cornerRadius = dp(10).toFloat()
                setColor(Color.parseColor(WECHAT_POPUP_PANEL_COLOR))
            }
        }
        scroll.addView(panel)
        root.addView(scroll, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            panelHeight
        ))

        items.forEachIndexed { idx, item ->
            when (item) {
                is PopupRowItem.Header -> {
                    panel.addView(createSectionHeaderView(item.title))
                }
                is PopupRowItem.Option -> {
                    panel.addView(createModelPopupRow(item.option.label, isModelOptionSelected(item.option)) {
                        dismissModelPopup(animate = false)
                        saveModelSelection(item.option)
                    })
                    val next = items.getOrNull(idx + 1)
                    if (next is PopupRowItem.Option) {
                        panel.addView(createPopupDivider(marginStart = dp(16)))
                    }
                }
            }
        }

        if (showCustomRow) {
            if (items.isNotEmpty()) panel.addView(createPopupDivider(marginStart = dp(16)))
            panel.addView(createModelPopupRow("自定义模型", currentModelLabel.startsWith("自定义")) {
                dismissModelPopup(animate = false)
                openSettings()
            })
        }

        val anchorLocation = IntArray(2)
        anchor.getLocationOnScreen(anchorLocation)
        val inputBarContainer = inputBarContainerProvider()
        val modelButtonShell = modelButtonShellProvider()
        val verticalAnchorLocation = IntArray(2)
        val verticalAnchorTop = if (
            inputBarContainer != null &&
            modelButtonShell != null &&
            anchor === modelButtonShell &&
            inputBarContainer.isShown
        ) {
            inputBarContainer.getLocationOnScreen(verticalAnchorLocation)
            verticalAnchorLocation[1] + modelButtonShell.top
        } else {
            anchorLocation[1]
        }
        val anchorCenterX = anchorLocation[0] + anchor.width / 2
        val aboveY = verticalAnchorTop - totalHeight - dp(8)
        val showAbove = aboveY > dp(72)
        modelPopupShowsAbove = showAbove
        val popupX = (anchorCenterX - popupWidth / 2)
            .coerceIn(dp(12), activity.resources.displayMetrics.widthPixels - popupWidth - dp(12))
        val popupY = if (showAbove) aboveY else verticalAnchorTop + anchor.height + dp(8)
        val arrowX = (anchorCenterX - popupX - dp(8)).coerceIn(dp(16), popupWidth - dp(32))

        root.addView(
            createPopupArrowView(pointsUp = !showAbove, color = Color.parseColor(WECHAT_POPUP_PANEL_COLOR)),
            FrameLayout.LayoutParams(dp(16), arrowHeight).apply {
                gravity = if (showAbove) Gravity.BOTTOM or Gravity.START else Gravity.TOP or Gravity.START
                leftMargin = arrowX
            }
        )
        if (!showAbove) {
            (panel.layoutParams as FrameLayout.LayoutParams).topMargin = arrowHeight
        }

        val popup = PopupWindow(root, popupWidth, totalHeight, false).apply {
            isOutsideTouchable = true
            inputMethodMode = PopupWindow.INPUT_METHOD_NOT_NEEDED
            softInputMode = WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAtLocation(binding.root, Gravity.NO_GRAVITY, popupX, popupY)
        }
        modelPopup = popup
        popup.setOnDismissListener {
            lastModelPopupDismissedAt = SystemClock.uptimeMillis()
            if (modelPopup === popup) {
                modelPopup = null
            }
            if (getActionPopup() === popup) {
                setActionPopup(null)
            }
            animateModelChevron(expanded = false)
        }
        setActionPopup(popup)
        animateModelChevron(expanded = true, popupAbove = showAbove)
        root.pivotX = (anchorCenterX - popupX).toFloat()
        root.pivotY = if (showAbove) totalHeight.toFloat() else 0f
        root.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .setDuration(120L)
            .start()
    }

    private fun recentlyDismissedModelPopup(): Boolean {
        val elapsed = SystemClock.uptimeMillis() - lastModelPopupDismissedAt
        return elapsed in 0L..MODEL_POPUP_REOPEN_SUPPRESS_MS
    }

    private fun dismissModelPopup(animate: Boolean) {
        val popup = modelPopup ?: return
        if (!popup.isShowing || !animate) {
            popup.dismiss()
            return
        }
        val content = popup.contentView ?: run {
            popup.dismiss()
            return
        }
        content.animate().cancel()
        content.animate()
            .alpha(0f)
            .scaleX(0.97f)
            .scaleY(0.97f)
            .setDuration(90L)
            .withEndAction {
                if (popup.isShowing) {
                    popup.dismiss()
                }
            }
            .start()
    }

    private fun animateModelChevron(
        expanded: Boolean,
        popupAbove: Boolean = modelPopupShowsAbove
    ) {
        val targetRotation = if (expanded && popupAbove) 180f else 0f
        modelChevronProvider()?.animate()
            ?.rotation(targetRotation)
            ?.setDuration(140L)
            ?.setInterpolator(chevronInterpolator)
            ?.start()
    }

    private fun isModelOptionSelected(option: ModelOption): Boolean {
        if (currentModelLabel == option.label) return true
        return selectedAgentName == option.agentName && !currentModelLabel.startsWith("自定义")
    }

    private fun createSectionHeaderView(title: String): View {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(30)
            )
            setPadding(dp(18), 0, dp(14), 0)
            gravity = Gravity.CENTER_VERTICAL
            text = title
            setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
            alpha = 0.5f
            textSize = 11f
        }
    }

    private fun createModelPopupRow(title: String, selected: Boolean, action: () -> Unit): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(52)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(18), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()

            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                includeFontPadding = false
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                text = title
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 14.5f
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(22), LinearLayout.LayoutParams.WRAP_CONTENT)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = if (selected) "✓" else ""
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 16f
            })
            setOnClickListener { action() }
        }
    }

    private fun saveModelSelection(option: ModelOption) {
        if (codexCliOnly && option.agentName.isNullOrBlank()) {
            Toast.makeText(activity, "当前已锁定使用 Codex CLI", Toast.LENGTH_SHORT).show()
            return
        }
        binding.modelButton.isEnabled = false
        modelButtonShellProvider()?.isEnabled = false
        Thread {
            try {
                val payload = JSONObject().apply {
                    put("use_agent", option.agentName ?: JSONObject.NULL)
                    put("api_base", JSONObject.NULL)
                    put("api_key", JSONObject.NULL)
                    put("model", JSONObject.NULL)
                }
                val body = payload.toString().toRequestBody("application/json".toMediaType())
                val response = http.newCall(
                    Request.Builder()
                        .url("$serverUrl/api/user/${userIdProvider()}/agent")
                        .put(body)
                        .build()
                ).execute()
                val responseBody = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(responseBody.ifBlank { "HTTP ${response.code}" })

                activity.runOnUiThread {
                    selectedAgentName = option.agentName
                    currentModelLabel = option.label
                    cacheModelSelection(option.agentName, option.label)
                    updateModelButton()
                    Toast.makeText(activity, "已切换模型: ${option.label}", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "模型切换失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            } finally {
                activity.runOnUiThread {
                    binding.modelButton.isEnabled = true
                    modelButtonShellProvider()?.isEnabled = true
                }
            }
        }.start()
    }

    private fun createPopupDivider(marginStart: Int = 0): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                this.marginStart = marginStart
            }
            alpha = 0.55f
            setBackgroundColor(Color.parseColor(WECHAT_POPUP_DIVIDER_COLOR))
        }
    }

    private fun createPopupArrowView(
        pointsUp: Boolean = true,
        color: Int = Color.parseColor(WECHAT_POPUP_PANEL_COLOR)
    ): View {
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            this.color = color
            style = Paint.Style.FILL
        }
        return object : View(activity) {
            override fun onDraw(canvas: Canvas) {
                super.onDraw(canvas)
                val path = Path().apply {
                    if (pointsUp) {
                        moveTo(width / 2f, 0f)
                        lineTo(width.toFloat(), height.toFloat())
                        lineTo(0f, height.toFloat())
                    } else {
                        moveTo(0f, 0f)
                        lineTo(width.toFloat(), 0f)
                        lineTo(width / 2f, height.toFloat())
                    }
                    close()
                }
                canvas.drawPath(path, paint)
            }
        }
    }

    private companion object {
        const val PREF_SELECTED_AGENT = "selected_agent_name"
        const val PREF_SELECTED_MODEL_LABEL = "selected_model_label"
        const val MODEL_POPUP_REOPEN_SUPPRESS_MS = 260L
    }
}

private sealed class PopupRowItem {
    data class Header(val title: String) : PopupRowItem()
    data class Option(val option: ModelOption) : PopupRowItem()
}
