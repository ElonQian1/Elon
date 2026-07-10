package com.elon.uiruntime.compose

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario

fun composePreviewScenario(
    screenId: String,
    supportedScenarios: Set<String>,
    content: @Composable (UiRuntimePreviewRequest) -> Unit,
): UiRuntimePreviewScenario = object : UiRuntimePreviewScenario {
    override val screenId = screenId
    override val supportedScenarios = supportedScenarios

    override fun createView(context: Context, request: UiRuntimePreviewRequest) =
        ComposeView(context).apply {
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
            setContent { content(request) }
        }
}
