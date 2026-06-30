package com.elon.app

import android.view.View
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainProjectViewActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val currentStage: () -> String,
    private val setCurrentStage: (String) -> Unit,
    private val setActiveProjectSubtitle: (String) -> Unit,
    private val currentProjectTitle: () -> String,
    private val projectEvents: () -> List<String>,
    private val currentTimeText: () -> String,
    private val saveProjects: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val renderProjectList: () -> Unit,
    private val updateStageHintShimmer: () -> Unit
) {
    fun updateStage(stage: String, hint: String) {
        setCurrentStage(stage)
        setActiveProjectSubtitle(hint)
        saveProjects()
        updateProjectViews(hint)
    }

    fun updateProjectViews(hint: String) {
        val stage = currentStage()
        val events = projectEvents()
        binding.currentStageText.text = stage
        binding.projectStatusText.text = "一龙开发助手"
        binding.stageHintText.text = hint
        binding.progressTitleText.text = "开发进度：$stage"
        binding.conversationTimeText.text = currentTimeText()
        binding.userInfoText.text = accountInfoText(activity)

        val recent = events.take(5).joinToString("\n")
        binding.projectOverviewText.text = buildString {
            append("项目管理\n")
            append("项目：${currentProjectTitle()}\n")
            append("阶段：$stage")
            if (recent.isNotBlank()) {
                append("\n\n最近记录\n")
                append(recent)
            }
        }
        binding.projectHistoryText.text = if (events.isEmpty()) {
            "暂无进度记录"
        } else {
            events.joinToString("\n")
        }
        binding.projectWorkflowText.text = projectWorkflowCardText(stage)
        updateStageLines(stage)
        renderConversationList()
        if (isProjectHomeVisible()) {
            renderProjectList()
        }
        updateStageHintShimmer()
    }

    private fun isProjectHomeVisible(): Boolean {
        return binding.projectPage.visibility == View.VISIBLE &&
            binding.pageTabs.visibility == View.VISIBLE
    }

    private fun updateStageLines(stage: String) {
        val active = when (stage) {
            "任务排队" -> 1
            "需求分析" -> 1
            "开发实现" -> 2
            "编译打包" -> 3
            "交付完成" -> 4
            "需要处理" -> -1
            else -> 0
        }
        binding.stagePlanText.text = stageLine(1, active, "需求分析")
        binding.stageCodeText.text = stageLine(2, active, "开发实现")
        binding.stageBuildText.text = stageLine(3, active, "编译打包")
        binding.stageDeliverText.text = stageLine(4, active, "交付下载")
    }
}
