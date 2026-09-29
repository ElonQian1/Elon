package com.elon.app

import android.content.Context
import android.view.ContextThemeWrapper
import android.view.LayoutInflater
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.databinding.ActivityMainBinding
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode
import java.text.DateFormat

/** Real home header/rows/navigation with synthetic data and local-only callbacks. */
internal fun homeConversationPreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.home.conversations"
    override val supportedScenarios = setOf("normal", "empty")
    override fun createView(context: Context, request: UiRuntimePreviewRequest): View =
        HomeConversationPreview(ContextThemeWrapper(context, R.style.Theme_ElonApp), request.scenario)
            .binding.root.uiNode("home.conversations.root")
}

internal class HomeConversationPreview(context: Context, scenario: String) {
    val binding = ActivityMainBinding.inflate(LayoutInflater.from(context))
    private val dp: (Int) -> Int = { (it * context.resources.displayMetrics.density).toInt() }
    val list = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
    var selected = HomeListFilterMode.All
    var opened = ""
    private val rows = MainHomeRows(context, DateFormat.getTimeInstance(), { -1 }, {}, { _, _ -> }, {}, {}, dp, { null })
    val counts = if (scenario == "empty") HomeConversationCounts(0, 0, 0, 0, 0) else HomeConversationCounts(9, 3, 5, 1, 2)
    val header = HomeConversationHeaderView(context, dp) { null }
    init {
        for (i in 0 until binding.contentContainer.childCount) binding.contentContainer.getChildAt(i).visibility = View.GONE
        for (i in 0 until binding.bottomBarContainer.childCount) binding.bottomBarContainer.getChildAt(i).visibility = View.GONE
        binding.stageHintBar.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        binding.homeMenuButton.visibility = View.VISIBLE
        binding.backButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.searchButton.visibility = View.VISIBLE
        binding.topTitleText.text = "消息"
        binding.topTitleText.uiNode("home.conversations.title")
        val metrics = MainNavigationDesignMetrics(context, binding) { _, _ -> }
        metrics.apply()
        MainBottomNavigationSelectionState(binding, metrics::applyBottomTabAssetState).selectPage(binding.tabChat)
        val scroll = ScrollView(context).apply {
            isFillViewport = true
            clipToPadding = false
            setPadding(0, 0, 0, dp(96))
            addView(list)
        }
        binding.contentContainer.addView(scroll)
        fun render() {
            list.removeAllViews()
            list.addView(header.create(selected, counts, { selected = it; render() }, { opened = "summary" })
                .uiNode("home.conversations.header"))
            if (scenario == "empty") {
                list.addView(TextView(context).apply {
                    text = "暂无消息\n开始对话，或进入项目继续工作。"
                    textSize = 16f
                    setTextColor(MobileColors(context).muted)
                    setPadding(dp(24), dp(32), dp(24), dp(32))
                })
            } else {
                val fixtures = listOf("产品协作群", "移动端项目", "一龙量化交易", "设计讨论", "较长的项目名称用于检查信息截断")
                fixtures.forEachIndexed { index, name ->
                    val group = AppGroup("fixture-$index", name, 2,
                        listOf(AppGroupMember("fixture-lin", "林悦", null), AppGroupMember("fixture-li", "李明", null)),
                        0, "新版页面已整理好，先检查输入和阅读体验。", null, if (index == 0) 2 else 0)
                    list.addView(rows.createGroupRow(group, showProjectMarker = index in 1..2) { opened = group.id })
                }
            }
        }
        render()
    }
}
