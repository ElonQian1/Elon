// domain/agent/AgentAction.kt
package com.elon.app.agent.domain.agent

/**
 * Agent 动作类型
 */
sealed class AgentAction {
    /** 点击坐标 */
    data class Tap(val x: Int, val y: Int) : AgentAction()
    
    /** 点击元素 */
    data class TapElement(val text: String) : AgentAction()
    
    /** 滑动 */
    data class Swipe(
        val direction: SwipeDirection,
        val distance: SwipeDistance = SwipeDistance.MEDIUM
    ) : AgentAction()
    
    /** 输入文字 */
    data class InputText(val text: String) : AgentAction()
    
    /** 按键 */
    data class PressKey(val key: KeyCode) : AgentAction()
    
    /** 打开应用 */
    data class LaunchApp(val packageName: String) : AgentAction()
    
    /** 等待 */
    data class Wait(val milliseconds: Long) : AgentAction()
    
    /** 执行 Shell 命令（需要权限） */
    data class RunCommand(val command: String) : AgentAction()
}

enum class SwipeDirection {
    UP, DOWN, LEFT, RIGHT
}

enum class SwipeDistance {
    SHORT, MEDIUM, LONG
}

enum class KeyCode {
    BACK, HOME, MENU, ENTER, DELETE
}

/**
 * 动作执行结果
 */
data class ActionResult(
    val success: Boolean,
    val message: String,
    val timestamp: Long = System.currentTimeMillis()
)
