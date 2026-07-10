package com.elon.app

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.graphics.Color
import android.net.Uri
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import com.elon.uiruntime.compose.defaultComposeRuntimePreviewScenario
import com.elon.uiruntime.view.UiRuntimePreviewRegistry
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

class UiRuntimeDebugPreviewProvider : ContentProvider() {
    override fun onCreate(): Boolean {
        UiRuntimePreviewRegistry.register(viewGalleryScenario())
        UiRuntimePreviewRegistry.register(defaultComposeRuntimePreviewScenario())
        return true
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor? = null

    override fun getType(uri: Uri): String? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0
    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String>?,
    ): Int = 0

    private fun viewGalleryScenario() = object : UiRuntimePreviewScenario {
        override val screenId = "elon.view.gallery"
        override val supportedScenarios = SCENARIOS

        override fun createView(context: Context, request: UiRuntimePreviewRequest): View =
            LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER
                setPadding(dp(context, 24), dp(context, 24), dp(context, 24), dp(context, 24))
                setBackgroundColor(if (request.theme == "dark") Color.rgb(18, 18, 18) else Color.WHITE)
                addView(TextView(context).apply {
                    text = "View Runtime · ${request.scenario}"
                    textSize = 22f
                    setTextColor(if (request.theme == "dark") Color.WHITE else Color.BLACK)
                }.uiNode("preview.view.title"))
                when (request.scenario) {
                    "loading" -> addView(ProgressBar(context).uiNode("preview.view.loading"))
                    "empty" -> addView(TextView(context).apply { text = "暂无内容" }.uiNode("preview.view.empty"))
                    "error" -> addView(TextView(context).apply {
                        text = "加载失败，请重试"
                        setTextColor(Color.rgb(180, 35, 35))
                    }.uiNode("preview.view.error"))
                    else -> addView(Button(context).apply { text = "主要操作" }.uiNode("preview.view.primary_action"))
                }
            }
    }

    companion object {
        private val SCENARIOS = setOf("normal", "loading", "empty", "error")
        private fun dp(context: Context, value: Int): Int =
            (value * context.resources.displayMetrics.density).toInt()
    }
}
