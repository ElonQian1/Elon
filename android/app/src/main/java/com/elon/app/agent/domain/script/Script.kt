// src/domain/script/Script.kt
// module: script | layer: domain | role: script-model
// summary: 脚本领域模型 - 定义可复用的自动化脚本结构

package com.elon.app.agent.domain.script

import com.google.gson.annotations.SerializedName

/**
 * 🎯 脚本数据模型
 * 代表一个可复用的自动化操作序列
 */
data class Script(
    /** 脚本唯一ID */
    val id: String,
    
    /** 脚本名称 */
    val name: String,
    
    /** 原始用户目标 */
    val goal: String,
    
    /** 脚本版本 */
    val version: String = "1.0",
    
    /** 脚本步骤列表 */
    val steps: List<ScriptStep>,
    
    /** 预期输出 */
    val outputs: List<String> = emptyList(),
    
    /** 创建时间 */
    @SerializedName("created_at")
    val createdAt: Long = System.currentTimeMillis(),
    
    /** 最后执行时间 */
    @SerializedName("last_executed_at")
    val lastExecutedAt: Long? = null,
    
    /** 执行成功次数 */
    @SerializedName("success_count")
    val successCount: Int = 0,
    
    /** 执行失败次数 */
    @SerializedName("fail_count")
    val failCount: Int = 0
)

/**
 * 📝 脚本步骤
 */
data class ScriptStep(
    /** 步骤序号 */
    val index: Int,
    
    /** 步骤类型 */
    val type: StepType,
    
    /** 步骤描述 */
    val description: String,
    
    /** 步骤参数 */
    val params: Map<String, Any> = emptyMap(),
    
    /** 条件（可选） */
    val condition: StepCondition? = null,
    
    /** 失败时的回退策略 */
    @SerializedName("on_fail")
    val onFail: FailAction = FailAction.RETRY,
    
    /** 最大重试次数 */
    @SerializedName("max_retries")
    val maxRetries: Int = 3
)

/**
 * 🔧 步骤类型
 */
enum class StepType {
    /** 启动应用 */
    LAUNCH_APP,
    
    /** 点击元素 */
    TAP,
    
    /** 滑动操作 */
    SWIPE,
    
    /** 等待 */
    WAIT,
    
    /** 查找并点击（带条件） */
    FIND_AND_TAP,
    
    /** 滚动直到找到 */
    SCROLL_UNTIL_FIND,
    
    /** 提取数据 */
    EXTRACT_DATA,
    
    /** 输入文本 */
    INPUT_TEXT,
    
    /** 搜索（等同于FIND_AND_TAP） */
    SEARCH,
    
    /** 返回 */
    BACK,
    
    /** 断言/验证 */
    ASSERT,
    
    /** 循环 */
    LOOP,
    
    /** 条件分支 */
    IF_ELSE,
    
    /** AI 决策（动态） */
    AI_DECIDE
}

/**
 * 🎯 步骤条件
 */
data class StepCondition(
    /** 条件类型 */
    val type: ConditionType,
    
    /** 目标属性 */
    val target: String,
    
    /** 操作符 */
    val operator: String,
    
    /** 期望值 */
    val value: Any
)

enum class ConditionType {
    /** 元素存在 */
    ELEMENT_EXISTS,
    
    /** 文本匹配 */
    TEXT_MATCHES,
    
    /** 数值比较 */
    NUMBER_COMPARE,
    
    /** 包含文本 */
    TEXT_CONTAINS,
    
    /** 当前应用 */
    CURRENT_APP
}

/**
 * 失败时的动作
 */
enum class FailAction {
    /** 重试 */
    RETRY,
    
    /** 跳过 */
    SKIP,
    
    /** 中止脚本 */
    ABORT,
    
    /** AI 接管决策 */
    AI_TAKEOVER
}

/**
 * 📊 脚本执行结果
 */
data class ScriptExecutionResult(
    /** 是否成功 */
    val success: Boolean,
    
    /** 执行的步骤数 */
    val stepsExecuted: Int,
    
    /** 总步骤数 */
    val totalSteps: Int,
    
    /** 提取的数据 */
    val extractedData: Map<String, Any> = emptyMap(),
    
    /** 错误信息 */
    val error: String? = null,
    
    /** 失败的步骤索引 */
    val failedStepIndex: Int? = null,
    
    /** 执行日志 */
    val logs: List<String> = emptyList(),
    
    /** 改进建议（AI生成） */
    val improvementSuggestions: List<String> = emptyList(),
    
    /** 🛡️ 清理的弹窗数量（智能模式） */
    val popupsDismissed: Int = 0,
    
    /** 🤖 AI 介入次数 */
    val aiInterventions: Int = 0,
    
    /** 🆕 是否被用户取消 */
    val cancelled: Boolean = false
)

/**
 * 🛡️ 智能恢复结果
 */
data class SmartRecoveryResult(
    /** 是否恢复成功 */
    val recovered: Boolean,
    /** 恢复操作描述 */
    val action: String
)
