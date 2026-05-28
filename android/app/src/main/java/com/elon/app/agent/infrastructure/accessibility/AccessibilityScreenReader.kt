// infrastructure/accessibility/AccessibilityScreenReader.kt
package com.elon.app.agent.infrastructure.accessibility

import android.accessibilityservice.AccessibilityService
import com.elon.app.agent.application.ScreenReader
import com.elon.app.agent.domain.screen.UINode

/**
 * 基于无障碍服务的屏幕读取器
 */
class AccessibilityScreenReader(
    private val service: AccessibilityService
) : ScreenReader {
    
    private val parser = UITreeParser(service)
    
    override suspend fun readCurrentScreen(): UINode {
        return parser.readCurrentScreen() 
            ?: UINode(
                className = "Error",
                text = "无法读取屏幕",
                contentDescription = null,
                resourceId = null,
                bounds = android.graphics.Rect(),
                children = emptyList()
            )
    }
}
