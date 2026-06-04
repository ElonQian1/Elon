package com.elon.app

import android.accessibilityservice.AccessibilityServiceInfo
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import android.view.accessibility.AccessibilityManager
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.agent.AgentConfigActivity
import com.elon.app.agent.AgentService
import com.elon.app.agent.infrastructure.ai.AIClientFactory
import com.elon.app.agent.infrastructure.floating.FloatingBallService
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.databinding.PageAgentBinding

/**
 * Agent 悬浮球功能入口控制器。
 *
 * 负责三件事：
 *  1. 检查 SYSTEM_ALERT_WINDOW（悬浮窗）权限并引导用户授权。
 *  2. 检查无障碍服务（AgentService）是否开启，并引导用户前往系统设置开启。
 *  3. 上面两个条件都满足后，启动 / 停止 FloatingBallService。
 *
 * 在 MainActivity 的 onResume() 里调用 refresh()，即可在用户从设置页返回时自动刷新状态。
 */
class AgentPageController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding
) {
    /** include 绑定对象（ViewBinding 自动生成 PageAgentBinding） */
    private val page: PageAgentBinding get() = binding.agentPage

    fun setup() {
        page.btnRequestOverlay.setOnClickListener { requestOverlayPermission() }
        page.btnOpenAccessibility.setOnClickListener { openAccessibilitySettings() }
        page.btnToggleFloatingBall.setOnClickListener { toggleFloatingBall() }
        page.btnOpenAgentConfig.setOnClickListener {
            activity.startActivity(Intent(activity, AgentConfigActivity::class.java))
        }
        refresh()
    }

    /** 在 Activity.onResume() 中调用，从系统设置页返回后刷新状态。 */
    fun refresh() {
        val overlayOk = canDrawOverlays()
        val accessibilityOk = isAgentServiceEnabled()
        val floatingOk = FloatingBallService.isRunning

        // 悬浮窗状态
        setStatus(
            icon = page.overlayStatusIcon, statusText = page.overlayStatusText, btn = page.btnRequestOverlay,
            ok = overlayOk,
            okLabel = "已授权", nokLabel = "未授权",
            btnOkText = "已获得悬浮窗权限", btnNokText = "前往授权悬浮窗"
        )

        // 无障碍状态
        setStatus(
            icon = page.accessibilityStatusIcon, statusText = page.accessibilityStatusText, btn = page.btnOpenAccessibility,
            ok = accessibilityOk,
            okLabel = "已开启", nokLabel = "未开启",
            btnOkText = "无障碍服务运行中", btnNokText = "前往无障碍设置"
        )

        // 悬浮球状态
        val canStart = overlayOk && accessibilityOk
        page.floatingStatusIcon.text = if (floatingOk) "●" else "○"
        page.floatingStatusIcon.setTextColor(android.graphics.Color.parseColor(if (floatingOk) "#58BE6A" else "#6F7785"))
        page.floatingStatusText.text = when {
            floatingOk -> "运行中"
            !canStart -> "请先完成步骤 1 和 2"
            else -> "未启动"
        }
        page.btnToggleFloatingBall.text = if (floatingOk) "停止悬浮球" else "启动悬浮球"
        page.btnToggleFloatingBall.alpha = if (canStart) 1.0f else 0.4f
        page.btnToggleFloatingBall.isEnabled = canStart

        // AI 链路状态：让用户清楚悬浮球到底走哪条
        page.agentAiStatusText.text = AIClientFactory.describe(activity)
    }

    // ------ 权限操作 ------

    private fun requestOverlayPermission() {
        val intent = Intent(
            Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
            Uri.parse("package:${activity.packageName}")
        )
        activity.startActivity(intent)
    }

    private fun openAccessibilitySettings() {
        activity.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        Toast.makeText(
            activity,
            "请找到「Elon Speed」，然后开启无障碍服务",
            Toast.LENGTH_LONG
        ).show()
    }

    private fun toggleFloatingBall() {
        if (FloatingBallService.isRunning) {
            activity.stopService(Intent(activity, FloatingBallService::class.java))
            Toast.makeText(activity, "悬浮球已关闭", Toast.LENGTH_SHORT).show()
        } else {
            FloatingBallService.start(activity)
        }
        // 延迟一点刷新，等服务状态稳定
        page.root.postDelayed({ refresh() }, 500)
    }

    // ------ 状态查询 ------

    private fun canDrawOverlays(): Boolean = Settings.canDrawOverlays(activity)

    private fun isAgentServiceEnabled(): Boolean {
        // 方式 1：直接检查 AgentService 静态实例（服务启动后会自赋值）
        if (AgentService.isRunning()) return true
        // 方式 2：通过系统 AccessibilityManager 查询（服务名精确匹配）
        val am = activity.getSystemService(Context.ACCESSIBILITY_SERVICE) as AccessibilityManager
        val enabled = am.getEnabledAccessibilityServiceList(AccessibilityServiceInfo.FEEDBACK_ALL_MASK)
        val target = "${activity.packageName}/${AgentService::class.java.name}"
        return enabled.any { it.resolveInfo.serviceInfo.let { si -> "${si.packageName}/${si.name}" } == target }
    }

    // ------ 工具 ------

    private fun setStatus(
        icon: TextView, statusText: TextView, btn: TextView,
        ok: Boolean,
        okLabel: String, nokLabel: String,
        btnOkText: String, btnNokText: String
    ) {
        icon.text = if (ok) "●" else "○"
        icon.setTextColor(android.graphics.Color.parseColor(if (ok) "#58BE6A" else "#6F7785"))
        statusText.text = if (ok) okLabel else nokLabel
        statusText.setTextColor(android.graphics.Color.parseColor(if (ok) "#58BE6A" else "#6F7785"))
        btn.text = if (ok) btnOkText else btnNokText
        btn.alpha = if (ok) 0.5f else 1.0f
        btn.isEnabled = !ok
    }
}
