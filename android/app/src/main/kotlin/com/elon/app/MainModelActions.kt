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
    private val openNodeSettings: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    var modelOptions: List<ModelOption> = emptyList()
        private set
    var codexCliOnly = true
        private set
    var selectedAgentName: String? = null
        private set
    var selectedRuntimeRoute: AiRuntimeRoute = AiRuntimeRoute.projectDefault
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

    fun selectedRuntimeRouteForRequest(): String? {
        return selectedRuntimeRoute.wireValue
    }

    fun restoreCachedModelSelection() {
        val label = cachedModelLabel() ?: return
        selectedAgentName = cachedModelAgentName()
        selectedRuntimeRoute = cachedRuntimeRoute()
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
                val userByokApiEnabled = json.optBoolean("user_byok_api_enabled", false)
                val config = json.optJSONObject("config") ?: JSONObject()
                val agents = json.optJSONArray("available_agents") ?: JSONArray()
                if (serverCodexCliOnly && !userByokApiEnabled) {
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
                        selectedRuntimeRoute = cachedRuntimeRoute()
                        currentModelLabel = label
                        cacheModelSelection(effectiveUseAgent, currentModelLabel, selectedRuntimeRoute)
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
                    selectedRuntimeRoute = cachedRuntimeRoute()
                    currentModelLabel = label
                    if (shouldSyncCache) {
                        cacheModelSelection(effectiveUseAgent, label, selectedRuntimeRoute)
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
        binding.modelButton.text = selectedRuntimeRoute.buttonLabel
        binding.modelButton.contentDescription = "AI方式：${selectedRuntimeRoute.title}；模型：$currentModelLabel"
        modelButtonShellProvider()?.contentDescription = "AI方式：${selectedRuntimeRoute.title}；模型：$currentModelLabel"
    }

    /// 软锁定回调：进入有首次 CLI 记录的会话时自动切换回对应的 agent
    fun switchToAgent(agentName: String) {
        if (selectedAgentName == agentName) return // 已经是正确的，不需要切换
        val option = modelOptions.firstOrNull { it.agentName == agentName } ?: return
        selectedAgentName = agentName
        currentModelLabel = option.label
        if (!option.matchesRuntimeRoute(selectedRuntimeRoute)) {
            selectedRuntimeRoute = if (option.isCliModelOption()) AiRuntimeRoute.MyPcAi else AiRuntimeRoute.PlatformAi
        }
        cacheModelSelection(agentName, option.label, selectedRuntimeRoute)
        updateModelButton()
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
            val backend = item.optString("backend", "")
            val reasoningEffort = jsonStringOrNull(item, "reasoning_effort")
            val reasoningSummary = jsonStringOrNull(item, "reasoning_summary")
            val verbosity = jsonStringOrNull(item, "verbosity")
            val displayModel = item.optString("display_model", "")
            val label = displayModel.trim().takeIf { it.isNotBlank() }
                ?: displayModelLabel(provider, model, item.optString("label", ""))
                    .withCodexRunMeta(provider, reasoningEffort, verbosity)
            val subtitle = codexOptionSubtitle(provider, model, reasoningEffort, reasoningSummary, verbosity)
            if (name.isNotBlank()) {
                options.add(
                    ModelOption(
                        label = label,
                        agentName = name,
                        provider = provider,
                        modelId = model,
                        backend = backend,
                        reasoningEffort = reasoningEffort,
                        reasoningSummary = reasoningSummary,
                        verbosity = verbosity,
                        subtitle = subtitle
                    )
                )
            }
        }
        return options
    }

    private fun String.withCodexRunMeta(
        provider: String,
        reasoningEffort: String?,
        verbosity: String?
    ): String {
        if (!provider.equals("codex", ignoreCase = true)) return this
        val parts = mutableListOf(this)
        reasoningEffort?.trim()?.takeIf { it.isNotBlank() }?.let { parts.add("推理 $it") }
        verbosity?.trim()?.takeIf { it.isNotBlank() }?.let { parts.add("输出 $it") }
        return parts.joinToString(" · ")
    }

    private fun codexOptionSubtitle(
        provider: String,
        model: String,
        reasoningEffort: String?,
        reasoningSummary: String?,
        verbosity: String?
    ): String? {
        if (!provider.equals("codex", ignoreCase = true)) return null
        val parts = mutableListOf<String>()
        model.trim().takeIf { it.isNotBlank() && !it.equals("default", ignoreCase = true) }
            ?.let { parts.add("模型 ${friendlyModelName(it)}") }
        reasoningEffort?.trim()?.takeIf { it.isNotBlank() }?.let { parts.add("推理 $it") }
        verbosity?.trim()?.takeIf { it.isNotBlank() }?.let { parts.add("输出 $it") }
        reasoningSummary?.trim()?.takeIf { it.isNotBlank() }?.let { parts.add("摘要 $it") }
        return parts.joinToString(" · ").takeIf { it.isNotBlank() }
    }

    private fun cacheModelSelection(
        agentName: String?,
        label: String,
        runtimeRoute: AiRuntimeRoute = selectedRuntimeRoute
    ) {
        prefs.edit().apply {
            if (agentName.isNullOrBlank()) remove(PREF_SELECTED_AGENT)
            else putString(PREF_SELECTED_AGENT, agentName)
            putString(PREF_SELECTED_MODEL_LABEL, label)
            putString(PREF_SELECTED_RUNTIME_ROUTE, runtimeRoute.wireValue ?: "auto")
            putBoolean(PREF_SELECTED_RUNTIME_ROUTE_DEFAULT_VERSION, true)
            putString(PREF_SELECTED_PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION, PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION)
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

    private fun cachedRuntimeRoute(): AiRuntimeRoute {
        val stored = prefs.getString(PREF_SELECTED_RUNTIME_ROUTE, null)
        val defaultVersionSeen = prefs.getBoolean(PREF_SELECTED_RUNTIME_ROUTE_DEFAULT_VERSION, false)
        val projectDefaultVersion = prefs.getString(PREF_SELECTED_PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION, null)
        if (stored.isNullOrBlank() || (!defaultVersionSeen && stored.equals("auto", ignoreCase = true))) {
            return AiRuntimeRoute.projectDefault
        }
        if (stored.equals(AiRuntimeRoute.default.wireValue, ignoreCase = true) &&
            projectDefaultVersion != PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION
        ) {
            return AiRuntimeRoute.projectDefault
        }
        return AiRuntimeRoute.fromStored(stored, AiRuntimeRoute.projectDefault)
    }

    private fun saveRuntimeRouteSelection(route: AiRuntimeRoute) {
        selectedRuntimeRoute = route
        val selectedOption = modelOptions.firstOrNull { isModelOptionSelected(it) }
        if (selectedOption != null && !selectedOption.matchesRuntimeRoute(route)) {
            val fallback = modelOptions.firstOrNull { it.matchesRuntimeRoute(route) && it.agentName == null }
                ?: modelOptions.firstOrNull { it.matchesRuntimeRoute(route) }
            selectedAgentName = fallback?.agentName
            currentModelLabel = fallback?.label ?: route.title
        }
        cacheModelSelection(selectedAgentName, currentModelLabel, route)
        updateModelButton()
        Toast.makeText(activity, "已切换为${route.title}", Toast.LENGTH_SHORT).show()
    }

    private fun configRowsForRuntimeRoute(): List<PopupRowItem.Action> {
        val rows = mutableListOf<PopupRowItem.Action>()
        when (selectedRuntimeRoute) {
            AiRuntimeRoute.MyKey -> {
                rows.add(PopupRowItem.Action("⚙ 配置我的Key", "保存 API Key 和模型，手机 AI 服务共用", openSettings))
            }
            AiRuntimeRoute.MyPcAi -> {
                rows.add(PopupRowItem.Action("⚙ 连接我的电脑", "安装或查看自己的 PC 节点", openNodeSettings))
            }
            AiRuntimeRoute.RemoteAi,
            AiRuntimeRoute.RemoteCodex -> {
                rows.add(PopupRowItem.Action("⚙ 选择远程 PC 节点", "查看在线节点、容量和可用 AI", openNodeSettings))
            }
            AiRuntimeRoute.Auto -> {
                rows.add(PopupRowItem.Action("⚙ 配置我的Key", "保存自己的 API Key 和模型", openSettings))
                rows.add(PopupRowItem.Action("⚙ 我的电脑 / 节点", "连接自己的 PC 或查看远程节点", openNodeSettings))
            }
            AiRuntimeRoute.PlatformAi -> {
                rows.add(PopupRowItem.Action("⚙ 平台模型设置", "切换平台模型或恢复默认", openSettings))
            }
        }
        if (
            !codexCliOnly &&
            selectedRuntimeRoute != AiRuntimeRoute.MyKey &&
            selectedRuntimeRoute != AiRuntimeRoute.Auto
        ) {
            rows.add(PopupRowItem.Action("我的Key设置", "需要自己付费模型时在这里配置", openSettings))
        }
        return rows
    }

    private fun showModelPopup(anchor: View) {
        getActionPopup()?.takeIf { it !== modelPopup }?.dismiss()
        modelPopup?.takeIf { it.isShowing }?.dismiss()
        modelPopup = null

        val selectableOptions = modelOptions.ifEmpty { listOf(ModelOption(currentModelLabel, selectedAgentName, "")) }

        // ── 构建带分组信息的行项目（section header + route rows + option rows）──────
        val items = mutableListOf<PopupRowItem>()
        items.add(PopupRowItem.Header("用谁的 AI"))
        AiRuntimeRoute.quickOptions.forEach { items.add(PopupRowItem.Route(it)) }

        val visibleModels = selectableOptions.filter { it.matchesRuntimeRoute(selectedRuntimeRoute) }
        if (visibleModels.isNotEmpty()) {
            items.add(PopupRowItem.Header("模型"))
            visibleModels.filter { it.agentName == null }.forEach { items.add(PopupRowItem.Option(it)) }
            val grouped = visibleModels
                .filter { it.agentName != null }
                .groupByTo(linkedMapOf()) { opt -> providerGroupTitle(opt.provider) }
            grouped.forEach { (header, opts) ->
                if (opts.isNotEmpty()) {
                    if (selectedRuntimeRoute == AiRuntimeRoute.Auto) items.add(PopupRowItem.Header(header))
                    opts.forEach { items.add(PopupRowItem.Option(it)) }
                }
            }
        }
        val actionRows = configRowsForRuntimeRoute()
        if (actionRows.isNotEmpty()) {
            items.add(PopupRowItem.Header("配置"))
            items.addAll(actionRows)
        }

        val headerHeight = dp(30)
        val rowHeight = dp(52)
        val availablePopupWidth = (activity.resources.displayMetrics.widthPixels - dp(24)).coerceAtLeast(dp(176))
        val popupWidth = dp(320).coerceAtMost(availablePopupWidth)
        val arrowHeight = dp(8)

        // 计算总高度（header 行更矮）
        var contentHeight = 0
        var dividerCount = 0
        items.forEachIndexed { idx, item ->
            contentHeight += if (item is PopupRowItem.Header) headerHeight else rowHeight
            val next = items.getOrNull(idx + 1)
            if (item !is PopupRowItem.Header && next !is PopupRowItem.Header && next != null) dividerCount++
        }
        val panelHeight = (contentHeight + dividerCount).coerceAtMost(dp(430))
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
                is PopupRowItem.Route -> {
                    panel.addView(createModelPopupRow(
                        item.route.title,
                        item.route.subtitle,
                        item.route == selectedRuntimeRoute
                    ) {
                        dismissModelPopup(animate = false)
                        saveRuntimeRouteSelection(item.route)
                    })
                    val next = items.getOrNull(idx + 1)
                    if (next !is PopupRowItem.Header && next != null) {
                        panel.addView(createPopupDivider(marginStart = dp(16)))
                    }
                }
                is PopupRowItem.Option -> {
                    panel.addView(createModelPopupRow(item.option.label, item.option.subtitle, isModelOptionSelected(item.option)) {
                        dismissModelPopup(animate = false)
                        saveModelSelection(item.option)
                    })
                    val next = items.getOrNull(idx + 1)
                    if (next !is PopupRowItem.Header && next != null) {
                        panel.addView(createPopupDivider(marginStart = dp(16)))
                    }
                }
                is PopupRowItem.Action -> {
                    panel.addView(createModelPopupRow(item.title, item.subtitle, false) {
                        dismissModelPopup(animate = false)
                        item.action()
                    })
                    val next = items.getOrNull(idx + 1)
                    if (next !is PopupRowItem.Header && next != null) {
                        panel.addView(createPopupDivider(marginStart = dp(16)))
                    }
                }
            }
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
            (scroll.layoutParams as FrameLayout.LayoutParams).topMargin = arrowHeight
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

    private fun createModelPopupRow(title: String, subtitle: String?, selected: Boolean, action: () -> Unit): View {
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

            val textColumn = LinearLayout(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_VERTICAL
            }
            textColumn.addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                text = title
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 14.5f
            })
            if (!subtitle.isNullOrBlank()) {
                textColumn.addView(TextView(context).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply {
                        topMargin = dp(4)
                    }
                    includeFontPadding = false
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    text = subtitle
                    setTextColor(Color.parseColor("#777777"))
                    textSize = 11.5f
                })
            }
            addView(textColumn)
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
        const val PREF_SELECTED_RUNTIME_ROUTE = "selected_runtime_route"
        const val PREF_SELECTED_RUNTIME_ROUTE_DEFAULT_VERSION = "selected_runtime_route_default_v2"
        const val PREF_SELECTED_PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION = "selected_project_runtime_route_default_version"
        const val PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION = "project-local-codex-default-20260702"
        const val MODEL_POPUP_REOPEN_SUPPRESS_MS = 260L
    }
}

private sealed class PopupRowItem {
    data class Header(val title: String) : PopupRowItem()
    data class Route(val route: AiRuntimeRoute) : PopupRowItem()
    data class Option(val option: ModelOption) : PopupRowItem()
    data class Action(val title: String, val subtitle: String?, val action: () -> Unit) : PopupRowItem()
}
