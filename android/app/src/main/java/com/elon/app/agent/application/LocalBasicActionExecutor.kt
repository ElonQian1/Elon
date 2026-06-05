// application/LocalBasicActionExecutor.kt
// module: application | layer: application | role: 基础动作本地直控
// summary: 把"打开应用/返回/回桌面/最近任务"等基础控机动作直接用无障碍执行，无需 AI、无需服务器

package com.elon.app.agent.application

import android.accessibilityservice.AccessibilityService
import android.content.Intent
import android.provider.Settings
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

        // 4. 打开应用 / 系统功能（仅"纯打开"，带复合后续意图的交给 AI）
        if (openVerbs.any { goal.contains(it) }) {
            // 4a. 先按"应用"解析：内置常用表 + PackageManager 动态解析任意已安装应用
            resolveApp(service, goal)?.let { app ->
                if (!isPureOpen(goal, app.name)) return Result.NotHandled
                return if (launchApp(service, app.pkg)) Result.Handled("已打开${app.name}")
                       else Result.NotHandled
            }
            // 4b. 应用没匹配到 → 系统功能兜底（设置/WiFi/蓝牙等，跨厂商用 Intent action）
            resolveSystemAction(g)?.let { sys ->
                if (!isSimpleOpen(goal)) return Result.NotHandled
                return if (startIntent(service, sys.intent)) Result.Handled("已打开${sys.name}")
                       else Result.NotHandled
            }
            // 4c. 都没命中（可能是未安装应用或更复杂意图）→ 交给 AI
            return Result.NotHandled
        }

        return Result.NotHandled
    }

    /** 仅当指令足够短（纯导航动作）时才拦截，避免误伤复杂任务。 */
    private fun isShortNav(goal: String): Boolean = goal.length <= 8

    private fun matchesAny(lowerGoal: String, keys: List<String>): Boolean =
        keys.any { lowerGoal.contains(it) }

    /** 解析到的应用：显示名 + 包名。 */
    private data class AppMatch(val name: String, val pkg: String)

    /**
     * 解析"打开 XXX"里的应用：
     *   1. 内置常用表优先（快、准，且确认已安装）；
     *   2. 未命中时用 PackageManager 动态遍历已安装应用，按显示名（label）匹配。
     * 这样无需为每个应用预置脚本，任意已安装应用都能本地打开。
     */
    private fun resolveApp(service: AccessibilityService, goal: String): AppMatch? {
        // 内置常用表：最长匹配优先（"哔哩哔哩"优先于"b站"）
        val builtin = appPackages.keys
            .filter { goal.contains(it, ignoreCase = true) }
            .maxByOrNull { it.length }
        if (builtin != null) {
            val pkg = appPackages.getValue(builtin)
            if (service.packageManager.getLaunchIntentForPackage(pkg) != null) {
                return AppMatch(builtin, pkg)
            }
        }
        // 动态：按应用显示名匹配已安装应用
        return resolveByLabel(service, goal)
    }

    /** 去掉动词和修饰词后，用剩余文本去匹配已安装应用的显示名。 */
    private fun resolveByLabel(service: AccessibilityService, goal: String): AppMatch? {
        val query = stripQuery(goal)
        if (query.length < 2) return null

        val pm = service.packageManager
        val mainIntent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        val infos = try {
            pm.queryIntentActivities(mainIntent, 0)
        } catch (e: Exception) {
            Log.w(TAG, "查询已安装应用失败: ${e.message}")
            return null
        }

        var exact: AppMatch? = null
        var partial: AppMatch? = null
        var partialLen = Int.MAX_VALUE
        for (info in infos) {
            val label = info.loadLabel(pm).toString().trim()
            if (label.isEmpty()) continue
            val pkg = info.activityInfo.packageName
            if (label == query) {
                exact = AppMatch(label, pkg)
                break
            }
            if (label.contains(query) || query.contains(label)) {
                // 选最短 label（最具体），减少歧义
                if (label.length < partialLen) {
                    partial = AppMatch(label, pkg)
                    partialLen = label.length
                }
            }
        }
        return exact ?: partial
    }

    /** 去掉动词和常见修饰词，得到用户想打开的应用名。 */
    private fun stripQuery(goal: String): String {
        var q = goal
        for (verb in openVerbs) q = q.replace(verb, "")
        for (filler in listOf("请", "帮我", "麻烦", "一下", "的", "app", "应用", "软件", "给我", "打開")) {
            q = q.replace(filler, "", ignoreCase = true)
        }
        return q.trim().trim('，', ',', '。', '.', '!', '！', '~', ' ')
    }

    /** 系统功能动作：无独立应用入口的系统设置项，用 Intent action 打开（跨厂商通用）。 */
    private data class SystemAction(val name: String, val intent: Intent)

    private fun resolveSystemAction(lowerGoal: String): SystemAction? {
        fun act(a: String) = Intent(a).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return when {
            lowerGoal.contains("wifi") || lowerGoal.contains("无线网") ->
                SystemAction("WiFi 设置", act(Settings.ACTION_WIFI_SETTINGS))
            lowerGoal.contains("蓝牙") ->
                SystemAction("蓝牙设置", act(Settings.ACTION_BLUETOOTH_SETTINGS))
            lowerGoal.contains("飞行模式") || lowerGoal.contains("无线和网络") ->
                SystemAction("无线和网络", act(Settings.ACTION_WIRELESS_SETTINGS))
            lowerGoal.contains("定位") || lowerGoal.contains("位置信息") ->
                SystemAction("定位设置", act(Settings.ACTION_LOCATION_SOURCE_SETTINGS))
            lowerGoal.contains("流量") || lowerGoal.contains("数据使用") ->
                SystemAction("流量设置", act(Settings.ACTION_DATA_USAGE_SETTINGS))
            // "设置/系统设置" 放最后兜底，避免吃掉上面更具体的设置项
            lowerGoal.contains("设置") ->
                SystemAction("系统设置", act(Settings.ACTION_SETTINGS))
            else -> null
        }
    }

    /** 系统设置类指令通常很短：足够短且不含复合连接词才算"纯打开"。 */
    private fun isSimpleOpen(goal: String): Boolean {
        if (goal.length > 12) return false
        val compound = listOf("然后", "接着", "并", "给", "发消息", "搜索", "播放", "之后")
        return compound.none { goal.contains(it) }
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

    private fun startIntent(service: AccessibilityService, intent: Intent): Boolean {
        return try {
            service.startActivity(intent)
            Log.i(TAG, "⚡ 本地打开系统功能: ${intent.action}")
            true
        } catch (e: Exception) {
            Log.w(TAG, "打开系统功能失败 ${intent.action}: ${e.message}")
            false
        }
    }
}
