package com.elon.app

import android.content.Context
import android.content.res.Configuration
import android.view.View
import android.widget.ScrollView
import com.elon.app.grid.share.GridShareModel
import com.elon.app.grid.share.GridSharePicker
import com.elon.app.grid.share.GridShareViews
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

/** Synthetic values only. No login, exchange read or group publication in a design preview. */
internal fun gridSharePreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.grid.share"
    override val supportedScenarios = setOf("profit", "loss", "unknown", "legacy", "details", "picker")
    override fun createView(context: Context, request: UiRuntimePreviewRequest): View {
        val config = Configuration(context.resources.configuration).apply {
            fontScale = request.fontScale
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                if (request.theme == "dark") Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        }
        val themed = android.view.ContextThemeWrapper(context.createConfigurationContext(config), R.style.Theme_ElonApp)
        val ui = GridShareViews(themed)
        val fields = mapOf("symbol" to "QNTUSDT", "direction" to "LONG", "status" to "WORKING", "leverage" to "10", "count" to "30", "spacing" to "ARITH",
            "lower" to "275.590000", "upper" to "326.290000", "markPrice" to "285.12345", "investment" to "1000.0000", "positionQty" to "12.34", "positionNotional" to "3518.423373",
            "profit" to if (request.scenario == "loss") "-123.450000" else "517.950000", "unrealizedPnl" to "-162.880000", "entryPrice" to "298.3225", "perGridQty" to "1.25")
        val grid = GridShareModel.project(if (request.scenario == "unknown") fields.filterKeys { it !in GridShareModel.amounts } else fields,
            1790852400000, request.scenario != "legacy")
        return ScrollView(themed).apply {
            setBackgroundColor(themed.getColor(R.color.mobile_surface))
            addView(ui.column().apply {
                addView(ui.text("网格分享 · 合成数据预览", quiet = true))
                addView(ui.summary(grid))
                if (request.scenario == "details") addView(ui.details(grid))
                if (request.scenario == "picker") addView(ui.button("打开网格选择器") {
                    GridSharePicker.show(themed, listOf(fields, fields + ("symbol" to "BTCUSDT") + ("profit" to "-12.34"), fields + ("symbol" to "TESTUSDT") + ("profit" to null))) { }
                })
            })
        }.uiNode("grid.share.preview")
    }
}
