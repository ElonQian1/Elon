// domain/tool/Tool.kt
package com.elon.app.agent.domain.tool

import com.elon.app.agent.domain.agent.ActionResult

/**
 * 工具定义接口
 */
interface Tool {
    val name: String
    val description: String
    val parameters: List<ToolParameter>
    
    suspend fun execute(params: Map<String, Any>): ActionResult
}

/**
 * 工具参数定义
 */
data class ToolParameter(
    val name: String,
    val type: ParameterType,
    val description: String,
    val required: Boolean = true,
    val defaultValue: Any? = null
)

enum class ParameterType {
    STRING, INT, FLOAT, BOOLEAN, ENUM
}

/**
 * 工具注册表
 */
class ToolRegistry {
    private val tools = mutableMapOf<String, Tool>()
    
    fun register(tool: Tool) {
        tools[tool.name] = tool
    }
    
    fun get(name: String): Tool? = tools[name]
    
    fun listAll(): List<Tool> = tools.values.toList()
    
    fun getToolDescriptions(): String {
        return tools.values.joinToString("\n") { tool ->
            val params = tool.parameters.joinToString(", ") { "${it.name}: ${it.type}" }
            "- ${tool.name}($params): ${tool.description}"
        }
    }
}
