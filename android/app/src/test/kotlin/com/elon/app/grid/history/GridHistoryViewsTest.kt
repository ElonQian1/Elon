package com.elon.app.grid.history

import android.content.res.Configuration
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import com.elon.app.R
import com.elon.app.grid.share.GridShareModel
import com.elon.app.grid.share.GridShareViews
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class GridHistoryViewsTest {
    @Test fun chineseHistoryShowsUnknownOrNegativeTotalWithoutClippingAndQuotesTheHistoricalVersion() {
        val fields = mapOf("symbol" to "龙虾USDT", "recordKind" to "HISTORY", "status" to "CANCELED", "profit" to "479.92684725",
            "created" to "1790800000000", "end" to "1790900000000", "settlement" to "UNKNOWN", "positionState" to "UNKNOWN", "investment" to "1200.001")
        for(dark in listOf(false, true)) for(scale in listOf(1f,2f)) for(total in listOf(null,"-301.50","0")) {
            val base = RuntimeEnvironment.getApplication()
            val config = Configuration(base.resources.configuration).apply { fontScale=scale;uiMode=(uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or if(dark) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO }
            val context=android.view.ContextThemeWrapper(base.createConfigurationContext(config),R.style.Theme_ElonApp)
            val grid=GridShareModel.project(fields + ("totalPnl" to total),1790928000000)
            val ui=GridShareViews(context);val card=ui.summary(grid);layout(card,ui.dp(232))
            val texts=descendants(card).filterIsInstance<TextView>()
            assertTrue(texts.any { it.text=="龙虾USDT" });assertFalse(texts.any { it.text=="?" })
            if(total==null) assertTrue(texts.any { it.text=="暂未读取" })
            if(total=="-301.50") assertEquals(context.getColor(R.color.mobile_financial_negative),texts.single { it.text=="-301.5" }.currentTextColor)
            texts.filter { it.text.isNotBlank() }.forEach { assertTrue("clipped ${it.text}",it.right<=(it.parent as View).width);assertEquals(it.text.length,it.layout.getLineEnd(it.layout.lineCount-1)) }
            val details=ui.details(grid);layout(details,ui.dp(232))
            assertTrue(descendants(details).filterIsInstance<TextView>().any { it.text=="历史" });assertFalse(descendants(details).filterIsInstance<TextView>().any { it.text=="持仓" })
            val message=GridShareModel.PREFIX+GridShareModel.document(grid).put("snapshot_id","ai_snapshot_history").put("group_id","group_history")
            assertTrue(GridShareModel.quoteSummary(message)!!.contains("[历史网格] 龙虾USDT"))
        }
    }
    private fun layout(view:View,width:Int){view.measure(View.MeasureSpec.makeMeasureSpec(width,View.MeasureSpec.EXACTLY),View.MeasureSpec.makeMeasureSpec(0,View.MeasureSpec.UNSPECIFIED));view.layout(0,0,width,view.measuredHeight)}
    private fun descendants(view:View):List<View> = listOf(view)+if(view is ViewGroup)(0 until view.childCount).flatMap {descendants(view.getChildAt(it))}else emptyList()
}
