package com.elon.uiruntime.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario

fun defaultComposeRuntimePreviewScenario(): UiRuntimePreviewScenario = composePreviewScenario(
    screenId = "elon.compose.gallery",
    supportedScenarios = setOf("normal", "loading", "empty", "error"),
    content = { request -> ComposeRuntimeGallery(request) },
)

@Composable
private fun ComposeRuntimeGallery(request: UiRuntimePreviewRequest) {
    val node = rememberUiNode(
        spec = UiNodeSpec(
            id = "preview.compose.primary_card",
            screenId = "elon.compose.gallery",
            kind = "card",
            source = UiSourceSymbol(
                module = ":ui-runtime-compose-debug",
                fullyQualifiedName = "com.elon.uiruntime.compose.ComposeRuntimeGallery",
                relativeFile = "ui-runtime-compose-debug/src/main/kotlin/com/elon/uiruntime/compose/UiRuntimeComposeGallery.kt",
            ),
            editableProperties = listOf(
                editable("width", 120.0, 360.0),
                editable("padding.start", 0.0, 48.0),
                editable("padding.top", 0.0, 48.0),
                editable("padding.end", 0.0, 48.0),
                editable("padding.bottom", 0.0, 48.0),
                editable("backgroundColor"),
                editable("contentColor"),
                editable("cornerRadius.all", 0.0, 48.0),
                editable("text"),
                editable("textSize", 10.0, 36.0),
                editable("opacity", 0.0, 1.0, 0.05),
            ),
        ),
        declaredStyle = UiStyle(
            width = 280.dp,
            paddingStart = 20.dp,
            paddingTop = 16.dp,
            paddingEnd = 20.dp,
            paddingBottom = 16.dp,
            backgroundColor = Color(0xFF5D3FD3),
            contentColor = Color.White,
            cornerRadius = 16.dp,
            text = when (request.scenario) {
                "loading" -> "正在加载…"
                "empty" -> "暂无内容"
                "error" -> "加载失败，请重试"
                else -> "Compose Runtime 已连接"
            },
            textSize = 18.sp,
            opacity = 1f,
        ),
    )
    val style = node.style
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(if (request.theme == "dark") Color(0xFF121212) else Color.White)
            .padding(32.dp),
    ) {
        Box(
            modifier = Modifier
                .width(style.width ?: 280.dp)
                .alpha(style.opacity ?: 1f)
                .background(
                    color = style.backgroundColor ?: Color(0xFF5D3FD3),
                    shape = RoundedCornerShape(style.cornerRadius ?: 16.dp),
                )
                .padding(
                    start = style.paddingStart ?: 20.dp,
                    top = style.paddingTop ?: 16.dp,
                    end = style.paddingEnd ?: 20.dp,
                    bottom = style.paddingBottom ?: 16.dp,
                )
                .uiNode(node),
        ) {
            BasicText(
                text = style.text.orEmpty(),
                style = TextStyle(
                    color = style.contentColor ?: Color.White,
                    fontSize = style.textSize ?: 18.sp,
                ),
            )
        }
    }
}

private fun editable(
    key: String,
    minimum: Double? = null,
    maximum: Double? = null,
    step: Double? = null,
) = UiEditableProperty(
    key = key,
    commitMode = "SESSION_ONLY",
    binding = UiSourceBinding.SessionOnly,
    minimum = minimum,
    maximum = maximum,
    step = step,
)
