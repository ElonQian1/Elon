// application/LocalBasicActionExecutor.kt
// module: application | layer: application | role: 基础动作本地直控
// summary: 把"打开应用/返回/回桌面/最近任务"等基础控机动作直接用无障碍执行，无需 AI、无需服务器

package com.elon.app.agent.application

import android.accessibilityservice.AccessibilityService
import android.content.Intent
import android.util.Log

/**
 * ⚡ 基础动作本地直控执行器
 *
 * 设计目标：像"打开微信""返回""回桌面"这类基础控机动作，
 * 完全不需要 AI 生成脚本、也不需要服务器参与——手机本地无障碍 API 就能直接做。
 *
 * 之前的问题：所有操作都走 [AgentService.executeGoalIndependently] →
 * [ScriptEngine.generateScript]（必须调 AI），导致：
 *   1. "打开微信"要等 AI 出脚本，慢；
 *   2. 一旦 AI 链路走服务器且服务器异常（如脚本执行环境报错），基础动作直接失败。
 *
 * 因此把"纯基础动作"在执行链最前面拦下来，本地直接执行。
 * **只拦截高置信度的纯基础动作**；任何带后续复合意图的指令
 * （如"打开微信给妈妈发消息"）一律返回 [Result.NotHandled]，继续交给 AI 脚本流程。
 */
object LocalBasicActionExecutor {
    private const val TAG = "LocalBasicAction"

    /**
     * 常用应用名 → 包名映射。
     * 必须与 [ScriptEngine] 生成脚本时 prompt 里的包名表保持一致。
     */
    private val appPackages: Map<String, String> = linkedMapOf(
        "微信" to "com.tencent.mm",
        "腾讯qq" to "com.tencent.mobileqq",
        "qq" to "com.tencent.mobileqq",
        "小红书" to "com.xingin.xhs",
        "抖音" to "com.ss.android.ugc.aweme",
        "快手" to "com.smile.gifmaker",
        "淘宝" to "com.taobao.taobao",
        "京东" to "com.jingdong.app.mall",
        "拼多多" to "com.xunmeng.pinduoduo",
        "闲鱼" to "com.taobao.idlefish",
        "支付宝" to "com.eg.android.AlipayGphone",
        "微博" to "com.sina.weibo",
        "哔哩哔哩" to "tv.danmaku.bili",
        "b站" to "tv.danmaku.bili",
        "美团" to "com.sankuai.meituan",
        "饿了么" to "me.ele",
        "高德地图" to "com.autonavi.minimap",
        "高德" to "com.autonavi.minimap",
        "百度地图" to "com.baidu.BaiduMap",
        "网易云音乐" to "com.netease.cloudmusic",
        "网易云" to "com.netease.cloudmusic",
        "qq音乐" to "com.tencent.qqmusic",
        "酷狗" to "com.kugou.android",
        "喜马拉雅" to "com.ximalaya.ting.android",
        "今日头条" to "com.ss.android.article.news",
        "知乎" to "com.zhihu.android",
        "豆瓣" to "com.douban.frodo",
        "钉钉" to "com.alibaba.android.rimet",
        "百度" to "com.baidu.searchbox",
        "微信读书" to "com.tencent.weread"
    )

    /** 启动应用时需要识别的动词。 */
    private val openVerbs = listOf("打开", "启动", "进入", "运行", "开启", "跳转到", "切换到", "唤起", "调起")

    /** 执行结果。 */
    sealed class Result {
        /** 已本地执行完成，附带可读提示。 */
        data class Handled(val message: String) : Result()

        /** 不是基础动作（或本地执行失败），调用方应继续走 AI 脚本流程。 */
        object NotHandled : Result()
    }

    /**
     * 尝试本地执行基础动作。
     *
     * @param service 当前无障碍服务（用于 performGlobalAction / startActivity）
     * @param rawGoal 用户原始意图文本
     * @return [Result.Handled] 表示已本地处理；[Result.NotHandled] 表示需走 AI。
     */
    fun tryExecute(service: AccessibilityService, rawGoal: String): Result {
        val goal = rawGoal.trim()
        if (goal.isEmpty()) return Result.NotHandled
        val g = goal.lowercase()

        // 1. 回桌面（先于"返回"判断，避免"返回桌面"被当成普通返回）
        if (isShortNav(goal) && matchesAny(g, listOf("回桌面", "回到桌面", "返回桌面", "回主屏", "主屏幕", "回首页桌面", "桌面"))) {
            service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_HOME)
            return Result.Handled("已回到桌面")
        }

        // 2. 最近任务 / 多任务
        if (isShortNav(goal) && matchesAny(g, listOf("最近任务", "多任务", "最近应用", "任务列表", "后台应用"))) {
            service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_RECENTS)
            return Result.Handled("已打开最近任务")
        }

        // 3. 返回 / 后退
        if (isShortNav(goal) && matchesAny(g, listOf("返回", "后退", "退回", "回上一步", "上一页", "返回上一页"))) {
            service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
            return Result.Handled("已返回")
        }

        // 4. 打开应用（仅"纯打开"，带复合后续意图的交给 AI）
        if (openVerbs.any { goal.contains(it) }) {
            val resolved = resolveApp(goal) ?: return Result.NotHandled
            val (name, pkg) = resolved
            if (!isPureOpen(goal, name)) return Result.NotHandled
            return if (launchApp(service, pkg)) {
                Result.Handled("已打开$name")
            } else {
                // 本地启动失败（未安装等），回退 AI 脚本流程再试
                Result.NotHandled
            }
        }

        return Result.NotHandled
    }

    /** 仅当指令足够短（纯导航动作）时才拦截，避免误伤复杂任务。 */
    private fun isShortNav(goal: String): Boolean = goal.length <= 8

    private fun matchesAny(lowerGoal: String, keys: List<String>): Boolean =
        keys.any { lowerGoal.contains(it) }

    /** 在 goal 中查找应用名，最长匹配优先（"哔哩哔哩"优先于"b站"等）。 */
    private fun resolveApp(goal: String): Pair<String, String>? {
        val match = appPackages.keys
            .filter { goal.contains(it, ignoreCase = true) }
            .maxByOrNull { it.length } ?: return null
        return match to appPackages.getValue(match)
    }

    /** 判断是否为"纯打开应用"：去掉动词、应用名、常见修饰词后无实质剩余。 */
    private fun isPureOpen(goal: String, appName: String): Boolean {
        var rest = goal
        for (verb in openVerbs) rest = rest.replace(verb, "")
        for (filler in listOf("请", "帮我", "麻烦", "一下", "到", "的", "app", "应用", "软件", "给我", "打開")) {
            rest = rest.replace(filler, "", ignoreCase = true)
        }
        rest = rest.replace(appName, "", ignoreCase = true)
            .trim()
            .trim('，', ',', '。', '.', '!', '！', '~', ' ')
        return rest.length <= 1
    }

    private fun launchApp(service: AccessibilityService, pkg: String): Boolean {
        return try {
            val intent = service.packageManager.getLaunchIntentForPackage(pkg) ?: run {
                Log.w(TAG, "应用未安装或无启动入口: $pkg")
                return false
            }
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            service.startActivity(intent)
            Log.i(TAG, "⚡ 本地直接启动: $pkg")
            true
        } catch (e: Exception) {
            Log.w(TAG, "本地启动失败 $pkg: ${e.message}")
            false
        }
    }
}
