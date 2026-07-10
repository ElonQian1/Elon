package com.elon.uiruntime.view

import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView
import androidx.activity.ComponentActivity
import java.util.Locale

class UiRuntimePreviewHostActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        render(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        render(intent)
    }

    private fun render(intent: Intent) {
        // onNewIntent reuses this Activity, so renderer-owned nodes from the previous
        // Compose/View scenario must not remain addressable while content is replaced.
        UiRuntimeBridge.clear()
        val request = request(intent)
        val configuredContext = configuredContext(request)
        val scenario = UiRuntimePreviewRegistry.find(request.screenId)
        val view = if (scenario == null) {
            diagnosticView(configuredContext, request)
        } else {
            require(request.scenario in scenario.supportedScenarios) {
                "${request.screenId} 不支持场景 ${request.scenario}"
            }
            scenario.createView(configuredContext, request)
        }
        view.setTag(R.id.yilong_ui_node_id, "preview.${request.screenId}.root")
        setContentView(view)
    }

    private fun configuredContext(request: UiRuntimePreviewRequest): Context {
        val configuration = Configuration(resources.configuration)
        configuration.fontScale = request.fontScale.coerceIn(0.5f, 2f)
        configuration.setLocale(Locale.forLanguageTag(request.localeTag))
        val nightMask = when (request.theme.lowercase(Locale.ROOT)) {
            "light" -> Configuration.UI_MODE_NIGHT_NO
            "dark" -> Configuration.UI_MODE_NIGHT_YES
            else -> resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
        }
        configuration.uiMode =
            (configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or nightMask
        return createConfigurationContext(configuration)
    }

    private fun diagnosticView(context: Context, request: UiRuntimePreviewRequest) =
        LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(dp(context, 24), dp(context, 24), dp(context, 24), dp(context, 24))
            setBackgroundColor(if (request.theme == "dark") Color.rgb(18, 18, 18) else Color.WHITE)
            addView(TextView(context).apply {
                text = "未注册 Preview：${request.screenId}"
                textSize = 18f
                setTextColor(if (request.theme == "dark") Color.WHITE else Color.BLACK)
            })
            addView(TextView(context).apply {
                text = UiRuntimePreviewRegistry.summaries().entries.joinToString(", ") { (id, states) ->
                    "$id(${states.joinToString("/")})"
                }.ifBlank { "当前 Debug 包没有 Preview 场景" }
                setTextColor(if (request.theme == "dark") Color.LTGRAY else Color.DKGRAY)
            })
        }

    private fun request(intent: Intent) = UiRuntimePreviewRequest(
        screenId = intent.getStringExtra(EXTRA_SCREEN_ID)?.trim().orEmpty(),
        scenario = intent.getStringExtra(EXTRA_SCENARIO)?.trim().orEmpty().ifBlank { "normal" },
        theme = intent.getStringExtra(EXTRA_THEME)?.trim().orEmpty().ifBlank { "system" },
        fontScale = intent.getFloatExtra(EXTRA_FONT_SCALE, 1f),
        localeTag = intent.getStringExtra(EXTRA_LOCALE)?.trim().orEmpty().ifBlank { "zh-CN" },
    )

    companion object {
        const val EXTRA_SCREEN_ID = "screen_id"
        const val EXTRA_SCENARIO = "scenario"
        const val EXTRA_THEME = "theme"
        const val EXTRA_FONT_SCALE = "font_scale"
        const val EXTRA_LOCALE = "locale"

        private fun dp(context: Context, value: Int): Int =
            (value * context.resources.displayMetrics.density).toInt()
    }
}
