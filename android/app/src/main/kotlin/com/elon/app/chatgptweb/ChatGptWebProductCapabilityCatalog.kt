package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebProductCapabilityCatalog {
    enum class Risk(val wireName: String) {
        STANDARD("standard"),
        SENSITIVE("sensitive"),
    }

    data class PageFeature(
        val id: String,
        val protocolKind: String,
        val semantic: String,
        val restorablePrefixes: List<String>,
        val risk: Risk = Risk.STANDARD,
    )

    data class ComposerTool(
        val id: String,
        val semantic: String,
    )

    val PAGE_FEATURES = listOf(
        PageFeature("projects", "projects", "project", listOf("/projects")),
        PageFeature("tasks", "tasks", "tasks", listOf("/tasks", "/scheduled")),
        PageFeature("library", "library", "library", listOf("/library")),
        PageFeature("gpts", "gpts", "gpts", listOf("/gpts")),
        PageFeature("apps", "apps", "apps", listOf("/apps", "/plugins")),
        PageFeature("settings", "settings", "settings", listOf("/settings")),
        PageFeature(
            "health",
            "health",
            "health",
            listOf("/health"),
            Risk.SENSITIVE,
        ),
        PageFeature(
            "finances",
            "finances",
            "finances",
            listOf("/finance", "/finances"),
            Risk.SENSITIVE,
        ),
        PageFeature("work", "work", "work", listOf("/work")),
    )

    val COMPOSER_TOOLS = listOf(
        ComposerTool("web_search", ChatGptWebComposerOptionSemantics.WEB_SEARCH),
        ComposerTool("deep_research", ChatGptWebComposerOptionSemantics.DEEP_RESEARCH),
        ComposerTool("image_generation", ChatGptWebComposerOptionSemantics.IMAGE_GENERATION),
        ComposerTool("canvas", ChatGptWebComposerOptionSemantics.CANVAS),
        ComposerTool("study_mode", ChatGptWebComposerOptionSemantics.STUDY),
        ComposerTool("agent_mode", ChatGptWebComposerOptionSemantics.AGENT),
    )

    val FEATURE_KINDS: Set<String> = PAGE_FEATURES
        .mapTo(linkedSetOf(), PageFeature::protocolKind)
        .apply { addAll(listOf("memory", "more", "navigation")) }

    val RESTORABLE_PREFIXES: List<String> = PAGE_FEATURES
        .flatMap(PageFeature::restorablePrefixes)
        .plus("/studymode")
        .distinct()

    fun requiresUserConfirmation(featureKind: String): Boolean =
        PAGE_FEATURES.firstOrNull { it.protocolKind == featureKind }?.risk == Risk.SENSITIVE

    fun selectionError(feature: ChatGptWebFeature?, userConfirmed: Boolean): String? = when {
        feature == null -> "stale_feature_id"
        requiresUserConfirmation(feature.kind) && !userConfirmed ->
            "user_confirmation_required"
        else -> null
    }

    fun navigationJson(feature: ChatGptWebFeature): JSONObject = JSONObject()
        .put("id", feature.id)
        .put("label", feature.label)
        .put("kind", feature.kind)
        .put("selected", feature.selected)
        .put("requires_user_confirmation", requiresUserConfirmation(feature.kind))
        .put("native_action", "chatgpt_select_feature")
        .put(
            "native_adb_content_description",
            ChatGptNativeNavigationSelector.feature(feature),
        )

    fun describe(
        features: Collection<ChatGptWebFeature>,
        composerOptions: Collection<ChatGptWebComposerOption>,
        controls: Collection<ChatGptWebUiControl>,
    ): JSONObject {
        val observedFeatureKinds = features.mapTo(mutableSetOf(), ChatGptWebFeature::kind)
        val observedControlSemantics = controls.mapTo(mutableSetOf(), ChatGptWebUiControl::semantic)
        val observedComposerSemantics = composerOptions
            .mapTo(mutableSetOf(), ChatGptWebComposerOption::semantic)
        return JSONObject()
            .put("schema", "elon.chatgpt_web.product_capabilities.v1")
            .put("page_features", JSONArray().apply {
                PAGE_FEATURES.forEach { page ->
                    put(JSONObject()
                        .put("id", page.id)
                        .put("kind", page.protocolKind)
                        .put("semantic", page.semantic)
                        .put("risk", page.risk.wireName)
                        .put("requires_user_confirmation", page.risk == Risk.SENSITIVE)
                        .put(
                            "current_page_observed",
                            page.protocolKind in observedFeatureKinds ||
                                page.semantic in observedControlSemantics,
                        )
                        .put("official_fallback", true)
                    )
                }
            })
            .put("composer_tools", JSONArray().apply {
                COMPOSER_TOOLS.forEach { tool ->
                    put(JSONObject()
                        .put("id", tool.id)
                        .put("semantic", tool.semantic)
                        .put("current_page_observed", tool.semantic in observedComposerSemantics)
                        .put("official_fallback", true)
                    )
                }
            })
    }
}
