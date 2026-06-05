// application/LocalChatResponder.kt
// module: application | layer: application | role: 本地常识问答
// summary: 把"今天几号/现在几点/星期几/电量"等本地就能答的问题直接本地回答，不发服务器 CLI

package com.elon.app.agent.application

import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import java.util.Calendar

/**
 * 💬 本地常识问答器
 *
 * 设计目标：CLI 模式下，服务器 CLI（Codex/Claude）是用来**改代码 / 做项目任务**的，
 * 不该也不擅长回答"今天几号""现在几点"这类日常问答——发过去要么困惑、要么超时报错。
 *
 * 因此把"手机本地就能直接答"的常识问题在发服务器之前拦下来，本地秒答。
 * 只覆盖**确定能本地答准**的几类；其它问答继续交给上层（AI / 服务器）。
 */
object LocalChatResponder {

    private val weekNames = arrayOf("日", "一", "二", "三", "四", "五", "六")

    /**
     * 尝试本地回答。
     *
     * @return 命中则返回答案文本；未命中返回 null（调用方继续走 AI / 服务器）。
     */
    fun tryAnswer(context: Context, rawInput: String): String? {
        val q = rawInput.trim().lowercase()
        if (q.isEmpty()) return null

        // 1. 日期：今天几号 / 几月几号 / 今天日期 / 今天星期几（日期+星期一起答）
        val asksDate = containsAny(q, "几号", "几月几", "今天日期", "现在日期", "今天是几", "今天几")
        val asksWeek = containsAny(q, "星期几", "周几", "礼拜几", "今天星期", "今天周")
        if (asksDate || asksWeek) {
            val cal = Calendar.getInstance()
            val month = cal.get(Calendar.MONTH) + 1
            val day = cal.get(Calendar.DAY_OF_MONTH)
            val week = weekNames[cal.get(Calendar.DAY_OF_WEEK) - 1]
            val year = cal.get(Calendar.YEAR)
            return when {
                asksDate && asksWeek -> "今天是 $year 年 $month 月 $day 日，星期$week。"
                asksWeek -> "今天是星期$week。"
                else -> "今天是 $year 年 $month 月 $day 日，星期$week。"
            }
        }

        // 2. 时间：现在几点 / 现在时间 / 几点了
        if (containsAny(q, "几点", "现在时间", "现在的时间", "报时", "什么时间", "时间是")) {
            val cal = Calendar.getInstance()
            val h = cal.get(Calendar.HOUR_OF_DAY)
            val m = cal.get(Calendar.MINUTE)
            val period = when (h) {
                in 0..4 -> "凌晨"
                in 5..8 -> "早上"
                in 9..11 -> "上午"
                12 -> "中午"
                in 13..17 -> "下午"
                else -> "晚上"
            }
            return "现在是 $period $h 点 ${if (m < 10) "0$m" else "$m"} 分。"
        }

        // 3. 电量：电量 / 电池 / 还有多少电
        if (containsAny(q, "电量", "电池", "多少电", "还有多少电", "剩多少电")) {
            batteryPercent(context)?.let { return "当前电量 $it%。" }
        }

        return null
    }

    private fun containsAny(text: String, vararg keys: String): Boolean =
        keys.any { text.contains(it) }

    private fun batteryPercent(context: Context): Int? {
        return try {
            val bm = context.getSystemService(Context.BATTERY_SERVICE) as? BatteryManager
            val viaManager = bm?.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
            if (viaManager != null && viaManager in 0..100) return viaManager

            val intent = context.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
            val level = intent?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
            val scale = intent?.getIntExtra(BatteryManager.EXTRA_SCALE, -1) ?: -1
            if (level >= 0 && scale > 0) (level * 100 / scale) else null
        } catch (e: Exception) {
            null
        }
    }
}
