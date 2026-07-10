package com.elon.uiruntime.view

import android.content.Context
import android.view.View
import java.util.concurrent.ConcurrentHashMap

data class UiRuntimePreviewRequest(
    val screenId: String,
    val scenario: String = "normal",
    val theme: String = "system",
    val fontScale: Float = 1f,
    val localeTag: String = "zh-CN",
)

interface UiRuntimePreviewScenario {
    val screenId: String
    val supportedScenarios: Set<String>

    fun createView(context: Context, request: UiRuntimePreviewRequest): View
}

object UiRuntimePreviewRegistry {
    private val scenarios = ConcurrentHashMap<String, UiRuntimePreviewScenario>()

    fun register(scenario: UiRuntimePreviewScenario) {
        require(scenario.screenId.isNotBlank()) { "Preview screenId 不能为空" }
        require(scenario.supportedScenarios.isNotEmpty()) { "Preview 至少需要一个场景" }
        scenarios[scenario.screenId] = scenario
    }

    fun unregister(screenId: String) {
        scenarios.remove(screenId)
    }

    fun find(screenId: String): UiRuntimePreviewScenario? = scenarios[screenId]

    fun summaries(): Map<String, Set<String>> = scenarios
        .toSortedMap()
        .mapValues { (_, scenario) -> scenario.supportedScenarios }
}
