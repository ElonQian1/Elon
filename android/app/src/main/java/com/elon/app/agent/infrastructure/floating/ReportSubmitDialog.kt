// infrastructure/floating/ReportSubmitDialog.kt
// module: infrastructure/floating | layer: infrastructure | role: report-submit-dialog
// summary: 报告提交对话框 - 显示执行报告详情，让用户选择是否提交学习优化

package com.elon.app.agent.infrastructure.floating

import android.app.AlertDialog
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.os.Handler
import android.os.Looper
import android.text.method.ScrollingMovementMethod
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.*
import com.elon.app.agent.infrastructure.debug.DebugInterface
import com.elon.app.agent.infrastructure.network.FailureReportService
import kotlinx.coroutines.*

/**
 * 📋 报告提交对话框
 * 
 * 功能：
 * - 显示详细的执行报告
 * - 让用户选择是否提交优化建议
 * - 提交时包含完整的步骤详情、重试原因等
 */
class ReportSubmitDialog(private val context: Context) {
    
    companion object {
        private const val TAG = "ReportSubmitDialog"
    }
    
    private val handler = Handler(Looper.getMainLooper())
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    
    /**
     * 显示执行报告对话框
     */
    fun show(onDismiss: (() -> Unit)? = null) {
        handler.post {
            try {
                showInternal(onDismiss)
            } catch (e: Exception) {
                Log.e(TAG, "显示报告对话框失败", e)
            }
        }
    }
    
    private fun showInternal(onDismiss: (() -> Unit)?) {
        val debugInterface = DebugInterface.getInstance()
        val report = debugInterface.lastExecutionReport
        
        if (report == null) {
            Toast.makeText(context, "暂无执行报告", Toast.LENGTH_SHORT).show()
            onDismiss?.invoke()
            return
        }
        
        // 创建主布局
        val mainLayout = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(32, 24, 32, 24)
            setBackgroundColor(Color.parseColor("#181B20"))
        }
        
        // 标题
        mainLayout.addView(createTitleSection(report))
        
        // 分隔线
        mainLayout.addView(createDivider())
        
        // 摘要信息
        mainLayout.addView(createSummarySection(report))
        
        // 分隔线
        mainLayout.addView(createDivider())
        
        // 详细报告（可滚动）
        mainLayout.addView(createDetailSection(debugInterface.getHumanReadableReport()))
        
        // 建议提交提示（如果有问题）
        if (report.shouldReport) {
            mainLayout.addView(createRecommendationBanner())
        }
        
        // 创建对话框
        val dialog = AlertDialog.Builder(context, android.R.style.Theme_Material_Dialog_Alert)
            .setView(mainLayout)
            .setPositiveButton(null, null)  // 手动处理按钮
            .setNegativeButton(null, null)
            .setCancelable(true)
            .create()
        
        // 添加按钮区域
        mainLayout.addView(createButtonSection(dialog, report, onDismiss))
        
        dialog.setOnDismissListener {
            onDismiss?.invoke()
        }
        
        // 设置窗口类型（用于悬浮窗）
        dialog.window?.apply {
            setType(WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY)
            setBackgroundDrawableResource(android.R.color.transparent)
        }
        
        dialog.show()
    }
    
    private fun createTitleSection(report: DebugInterface.ExecutionReport): LinearLayout {
        return LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            
            // 状态图标
            addView(TextView(context).apply {
                text = if (report.success) "✅" else "❌"
                textSize = 24f
                setPadding(0, 0, 16, 0)
            })
            
            // 标题
            addView(TextView(context).apply {
                text = "执行报告"
                textSize = 20f
                setTextColor(Color.parseColor("#F2F5FA"))
                typeface = Typeface.DEFAULT_BOLD
            })
            
            // 性能评分徽章
            addView(createPerformanceBadge(report.summary.performanceScore))
        }
    }
    
    private fun createPerformanceBadge(score: String): TextView {
        val (bgColor, text) = when (score) {
            "GOOD" -> Pair(Color.parseColor("#58BE6A"), "优秀")
            "FAIR" -> Pair(Color.parseColor("#81B3D9"), "一般")
            else -> Pair(Color.parseColor("#D97A7A"), "较差")
        }
        
        return TextView(context).apply {
            this.text = text
            textSize = 12f
            setTextColor(Color.parseColor("#F2F5FA"))
            setBackgroundColor(bgColor)
            setPadding(16, 8, 16, 8)
            gravity = Gravity.CENTER
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = 16
            }
        }
    }
    
    private fun createSummarySection(report: DebugInterface.ExecutionReport): LinearLayout {
        return LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 16, 0, 16)
            
            addView(createInfoRow("🎯 任务", report.goal))
            addView(createInfoRow("⏱️ 耗时", formatDuration(report.totalDurationMs)))
            addView(createInfoRow("📊 步骤", "${report.completedSteps}/${report.totalSteps}"))
            addView(createInfoRow("🔄 重试", "${report.summary.totalRetries} 次"))
            
            if (report.summary.aiInterventions > 0) {
                addView(createInfoRow("🤖 AI介入", "${report.summary.aiInterventions} 次"))
            }
            
            if (report.summary.slowSteps.isNotEmpty()) {
                addView(createInfoRow("⚠️ 慢步骤", report.summary.slowSteps.joinToString(", ") { "步骤$it" }))
            }
        }
    }
    
    private fun createInfoRow(label: String, value: String): LinearLayout {
        return LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(0, 4, 0, 4)
            
            addView(TextView(context).apply {
                text = label
                textSize = 14f
                setTextColor(Color.parseColor("#A6AFBD"))
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })
            
            addView(TextView(context).apply {
                text = value
                textSize = 14f
                setTextColor(Color.parseColor("#F2F5FA"))
            })
        }
    }
    
    private fun createDetailSection(reportText: String): ScrollView {
        return ScrollView(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                400  // 固定高度，可滚动
            ).apply {
                topMargin = 16
                bottomMargin = 16
            }
            
            addView(TextView(context).apply {
                text = reportText
                textSize = 12f
                setTextColor(Color.parseColor("#DDE8FC"))
                setBackgroundColor(Color.parseColor("#0F1217"))
                setPadding(16, 16, 16, 16)
                movementMethod = ScrollingMovementMethod.getInstance()
                typeface = Typeface.MONOSPACE
            })
        }
    }
    
    private fun createRecommendationBanner(): LinearLayout {
        return LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor("#152C3E"))
            setPadding(16, 12, 16, 12)
            gravity = Gravity.CENTER_VERTICAL
            
            addView(TextView(context).apply {
                text = "💡"
                textSize = 18f
                setPadding(0, 0, 12, 0)
            })
            
            addView(TextView(context).apply {
                text = "检测到性能问题，建议提交报告帮助我们改进"
                textSize = 13f
                setTextColor(Color.parseColor("#81B3D9"))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
            })
            
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = 16
            }
        }
    }
    
    private fun createButtonSection(
        dialog: AlertDialog, 
        report: DebugInterface.ExecutionReport,
        onDismiss: (() -> Unit)?
    ): LinearLayout {
        return LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.END
            setPadding(0, 24, 0, 0)
            
            // 关闭按钮
            addView(Button(context).apply {
                text = "关闭"
                setTextColor(Color.parseColor("#A6AFBD"))
                setBackgroundColor(Color.TRANSPARENT)
                setOnClickListener {
                    dialog.dismiss()
                }
            })
            
            // 提交按钮
            addView(Button(context).apply {
                text = "📤 提交优化建议"
                setTextColor(Color.parseColor("#F2F5FA"))
                setBackgroundColor(Color.parseColor("#283140"))
                setPadding(32, 16, 32, 16)
                
                setOnClickListener {
                    isEnabled = false
                    text = "提交中..."
                    submitReport(report, dialog, this, onDismiss)
                }
            })
        }
    }
    
    private fun createDivider(): View {
        return View(context).apply {
            setBackgroundColor(Color.parseColor("#283140"))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                2
            ).apply {
                topMargin = 8
                bottomMargin = 8
            }
        }
    }
    
    private fun submitReport(
        report: DebugInterface.ExecutionReport,
        dialog: AlertDialog,
        submitButton: Button,
        onDismiss: (() -> Unit)?
    ) {
        scope.launch {
            try {
                // 构建详细的失败/优化报告
                val reportJson = DebugInterface.getInstance().getReportForSubmission()
                
                // 调用云端 API 上报
                val result = FailureReportService.reportPerformanceIssue(
                    taskGoal = report.goal,
                    success = report.success,
                    totalDurationMs = report.totalDurationMs,
                    totalRetries = report.summary.totalRetries,
                    slowSteps = report.summary.slowSteps,
                    detailJson = reportJson,
                    recommendation = report.recommendation
                )
                
                withContext(Dispatchers.Main) {
                    result.onSuccess { id ->
                        Toast.makeText(context, "✅ 报告已提交 (#$id)，感谢您的反馈！", Toast.LENGTH_LONG).show()
                        dialog.dismiss()
                    }.onFailure { e ->
                        submitButton.isEnabled = true
                        submitButton.text = "📤 提交优化建议"
                        Toast.makeText(context, "❌ 提交失败: ${e.message}", Toast.LENGTH_LONG).show()
                    }
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    submitButton.isEnabled = true
                    submitButton.text = "📤 提交优化建议"
                    Toast.makeText(context, "❌ 提交失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }
    }
    
    private fun formatDuration(ms: Long): String {
        return when {
            ms < 1000 -> "${ms}毫秒"
            ms < 60000 -> "%.1f秒".format(ms / 1000.0)
            else -> "%.1f分钟".format(ms / 60000.0)
        }
    }
    
    fun destroy() {
        scope.cancel()
    }
}
