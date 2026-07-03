package com.elon.app

internal enum class AiRuntimeRoute(
    val wireValue: String?,
    val buttonLabel: String,
    val title: String,
    val subtitle: String,
    val configTitle: String? = null
) {
    Auto(
        wireValue = null,
        buttonLabel = "自动",
        title = "自动选择",
        subtitle = "一龙按当前项目选择合适的 AI"
    ),
    MyPcAi(
        wireValue = "route_a",
        buttonLabel = "本机AI",
        title = "本机AI",
        subtitle = "连接自己 PC 上已经登录的 Codex / Copilot / Claude",
        configTitle = "连接我的电脑"
    ),
    MyKey(
        wireValue = "route_b",
        buttonLabel = "我的Key",
        title = "我的Key",
        subtitle = "用自己的 API Key，手机里的 AI 服务也共用",
        configTitle = "配置我的Key"
    ),
    PlatformAi(
        wireValue = "route_c",
        buttonLabel = "平台AI",
        title = "平台AI",
        subtitle = "使用一龙平台提供的 AI"
    ),
    RemoteAi(
        wireValue = "route_c2",
        buttonLabel = "远程AI",
        title = "远程一龙AI",
        subtitle = "其他用户 PC 节点 + 一龙 CLI",
        configTitle = "选择远程节点"
    ),
    RemoteCodex(
        wireValue = "route_c3",
        buttonLabel = "远Codex",
        title = "远程 Codex / Claude",
        subtitle = "其他用户 PC 节点 + Codex / Claude",
        configTitle = "选择远程节点"
    );

    companion object {
        val default: AiRuntimeRoute = PlatformAi
        val projectDefault: AiRuntimeRoute = MyPcAi

        val quickOptions: List<AiRuntimeRoute> = listOf(
            Auto,
            MyPcAi,
            MyKey,
            PlatformAi,
            RemoteAi,
            RemoteCodex
        )

        fun fromStored(value: String?, fallback: AiRuntimeRoute = default): AiRuntimeRoute {
            val clean = value?.trim().orEmpty()
            if (clean.isBlank()) return fallback
            if (clean.equals("auto", ignoreCase = true)) return Auto
            return quickOptions.firstOrNull { it.wireValue.equals(clean, ignoreCase = true) } ?: fallback
        }
    }
}

internal fun ModelOption.matchesRuntimeRoute(route: AiRuntimeRoute): Boolean {
    return when (route) {
        AiRuntimeRoute.Auto -> true
        AiRuntimeRoute.MyPcAi,
        AiRuntimeRoute.RemoteCodex -> isCliModelOption()
        AiRuntimeRoute.PlatformAi -> agentName == null || backend.equals("api", ignoreCase = true)
        AiRuntimeRoute.MyKey,
        AiRuntimeRoute.RemoteAi -> false
    }
}

internal fun ModelOption.isCliModelOption(): Boolean {
    if (backend.equals("cli", ignoreCase = true)) return true
    val providerName = provider.trim().lowercase()
    return providerName in setOf("codex", "copilot", "github", "claude", "gemini")
}
