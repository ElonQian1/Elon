package com.elon.uiruntime.compose

import android.content.Context
import android.graphics.Color.parseColor
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import org.json.JSONObject

fun defaultComposeRuntimePreviewScenario(): UiRuntimePreviewScenario = composePreviewScenario(
    screenId = "elon.compose.gallery",
    supportedScenarios = setOf("normal", "loading", "empty", "error"),
    content = { request -> ComposeRuntimeGallery(request) },
)

@Composable
private fun ComposeRuntimeGallery(request: UiRuntimePreviewRequest) {
    val context = LocalContext.current
    val baseline = remember { loadGalleryStyle(context) }
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
                styleEditable("width", "/primaryCard/width", 120.0, 360.0),
                styleEditable("padding.start", "/primaryCard/paddingStart", 0.0, 48.0),
                styleEditable("padding.top", "/primaryCard/paddingTop", 0.0, 48.0),
                styleEditable("padding.end", "/primaryCard/paddingEnd", 0.0, 48.0),
                styleEditable("padding.bottom", "/primaryCard/paddingBottom", 0.0, 48.0),
                styleEditable("backgroundColor", "/primaryCard/backgroundColor"),
                styleEditable("contentColor", "/primaryCard/contentColor"),
                styleEditable("cornerRadius.all", "/primaryCard/cornerRadius", 0.0, 48.0),
                sessionEditable("text"),
                styleEditable("textSize", "/primaryCard/textSize", 10.0, 36.0),
                styleEditable("opacity", "/primaryCard/opacity", 0.0, 1.0, 0.05),
            ),
        ),
        declaredStyle = UiStyle(
            width = baseline.width.dp,
            paddingStart = baseline.paddingStart.dp,
            paddingTop = baseline.paddingTop.dp,
            paddingEnd = baseline.paddingEnd.dp,
            paddingBottom = baseline.paddingBottom.dp,
            backgroundColor = parseComposeColor(baseline.backgroundColor, Color(0xFF5D3FD3)),
            contentColor = parseComposeColor(baseline.contentColor, Color.White),
            cornerRadius = baseline.cornerRadius.dp,
            text = when (request.scenario) {
                "loading" -> "正在加载…"
                "empty" -> "暂无内容"
                "error" -> "加载失败，请重试"
                else -> "Compose Runtime 已连接"
            },
            textSize = baseline.textSize.sp,
            opacity = baseline.opacity,
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
                // Bind geometry at the component boundary. Placing uiNode after the
                // inner padding would report content width instead of card width.
                .uiNode(node)
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
                ),
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

private fun styleEditable(
    key: String,
    jsonPointer: String,
    minimum: Double? = null,
    maximum: Double? = null,
    step: Double? = null,
) = UiEditableProperty(
    key = key,
    commitMode = "DETERMINISTIC",
    binding = UiSourceBinding.StyleJson(STYLE_SOURCE_FILE, jsonPointer),
    minimum = minimum,
    maximum = maximum,
    step = step,
)

private fun sessionEditable(key: String) = UiEditableProperty(
    key = key,
    commitMode = "SESSION_ONLY",
    binding = UiSourceBinding.SessionOnly,
)

private data class ComposeGalleryStyle(
    val width: Float = 280f,
    val paddingStart: Float = 20f,
    val paddingTop: Float = 16f,
    val paddingEnd: Float = 20f,
    val paddingBottom: Float = 16f,
    val backgroundColor: String = "#FF5D3FD3",
    val contentColor: String = "#FFFFFFFF",
    val cornerRadius: Float = 16f,
    val textSize: Float = 18f,
    val opacity: Float = 1f,
)

private fun loadGalleryStyle(context: Context): ComposeGalleryStyle = runCatching {
    val document = context.assets.open(STYLE_ASSET_PATH).bufferedReader().use { it.readText() }
    val card = JSONObject(document).getJSONObject("primaryCard")
    ComposeGalleryStyle(
        width = card.getDouble("width").toFloat(),
        paddingStart = card.getDouble("paddingStart").toFloat(),
        paddingTop = card.getDouble("paddingTop").toFloat(),
        paddingEnd = card.getDouble("paddingEnd").toFloat(),
        paddingBottom = card.getDouble("paddingBottom").toFloat(),
        backgroundColor = card.getString("backgroundColor"),
        contentColor = card.getString("contentColor"),
        cornerRadius = card.getDouble("cornerRadius").toFloat(),
        textSize = card.getDouble("textSize").toFloat(),
        opacity = card.getDouble("opacity").toFloat().coerceIn(0f, 1f),
    )
}.getOrDefault(ComposeGalleryStyle())

private fun parseComposeColor(value: String, fallback: Color): Color = runCatching {
    Color(parseColor(value))
}.getOrDefault(fallback)

private const val STYLE_ASSET_PATH = "yilong/ui-runtime-gallery.styles.json"
private const val STYLE_SOURCE_FILE =
    "android/ui-runtime-compose-debug/src/main/assets/yilong/ui-runtime-gallery.styles.json"
