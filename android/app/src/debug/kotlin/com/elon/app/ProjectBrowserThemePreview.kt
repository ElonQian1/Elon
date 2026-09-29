package com.elon.app

import android.content.Context
import android.view.ContextThemeWrapper
import android.view.LayoutInflater
import android.view.View
import android.widget.EditText
import com.elon.app.databinding.ActivityMainBinding
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

/** Production sheet, navigation and resources with local data; no project/network mutations. */
internal fun projectBrowserThemePreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.project.browser"
    override val supportedScenarios = setOf("normal", "empty", "search")
    override fun createView(context: Context, request: UiRuntimePreviewRequest): View =
        ProjectBrowserThemePreview(ContextThemeWrapper(context, R.style.Theme_ElonApp), request.scenario)
            .binding.root.uiNode("project.browser.root")
}

internal class ProjectBrowserThemePreview(context: Context, scenario: String) {
    val binding = ActivityMainBinding.inflate(LayoutInflater.from(context))
    private val dp: (Int) -> Int = { (it * context.resources.displayMetrics.density).toInt() }
    val projects = if (scenario == "empty") emptyList() else List(20) { index ->
        AppProject("offline-$index", listOf("手机控制", "聊天记忆", "移动端项目", "较长的项目名称，用于检查字体放大")[index % 4],
            "离线布局示例", index.toLong(), isJointProject = index >= 10)
    }
    var openedProjectIndex: Int? = null
    private val metrics = MainNavigationDesignMetrics(context, binding) { _, _ -> }
    private val navigation = MainBottomNavigationSelectionState(binding, metrics::applyBottomTabAssetState)
    val controller = ProjectBrowserSheetController(context, binding, dp,
        ProjectBrowserSheetDependencies({ projects }, { openedProjectIndex = it }, { null }), navigation::setProjectBrowserOpen)

    init {
        for (index in 0 until binding.contentContainer.childCount) binding.contentContainer.getChildAt(index).visibility = View.GONE
        for (index in 0 until binding.bottomBarContainer.childCount) binding.bottomBarContainer.getChildAt(index).visibility = View.GONE
        binding.topTitleText.text = "项目"
        binding.stageHintBar.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        metrics.apply()
        navigation.selectPage(binding.tabChat)
        controller.setup()
        binding.projectBrowserSheet.uiNode("project.browser.sheet")
        binding.root.findViewById<EditText>(R.id.projectBrowserSearchInput).uiNode("project.browser.search")
        binding.bottomMenuButton.setOnClickListener { controller.toggle() }
        controller.open()
        if (scenario == "search") binding.root.post {
            binding.root.findViewById<EditText>(R.id.projectBrowserSearchInput).setText("手机")
        }
    }
}
