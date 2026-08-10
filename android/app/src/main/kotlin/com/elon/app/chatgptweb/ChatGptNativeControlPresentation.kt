package com.elon.app.chatgptweb

internal object ChatGptNativeControlPresentation {
    const val HEADER_ACTION_LIMIT = 2
    const val SUGGESTION_LIMIT = 4
    const val OVERLAY_ACTION_LIMIT = 40

    enum class Kind(val wireName: String) {
        DIRECT("direct"),
        DEDICATED("dedicated"),
        MENU("menu"),
        METADATA("metadata"),
        OFFICIAL_FALLBACK("official_fallback"),
    }

    data class Coverage(
        val controlId: String,
        val kind: Kind,
        val nativeSelector: String? = null,
        val nativeTriggerSelector: String? = null,
    )

    fun describe(controls: List<ChatGptWebUiControl>): Map<String, Coverage> {
        val headerIds = headerActions(controls).mapTo(mutableSetOf(), ChatGptWebUiControl::id)
        val suggestionIds = suggestions(controls).mapTo(mutableSetOf(), ChatGptWebUiControl::id)
        val messageActions = messageActions(controls)
        val messageActionIds = messageActions.values.flatten()
            .mapTo(mutableSetOf(), ChatGptWebUiControl::id)
        val primaryCopyIds = primaryMessageCopies(controls)
            .mapTo(mutableSetOf(), ChatGptWebUiControl::id)
        val messageActionCounts = messageActions.mapValues { it.value.size }
        val overlayIds = overlayActions(controls).mapTo(mutableSetOf(), ChatGptWebUiControl::id)
        val overlayTrigger = "chatgpt-overlay-actions:${overlayIds.size}"

        return controls.associate { control ->
            val coverage = when {
                control.id in headerIds || control.id in suggestionIds -> Coverage(
                    control.id,
                    Kind.DIRECT,
                    nativeSelector = control.accessibilityLabel,
                )
                control.region == ChatGptWebUiRegion.HEADER &&
                    control.semantic in HEADER_DEDICATED_SEMANTICS -> headerDedicatedCoverage(control)
                control.region == ChatGptWebUiRegion.COMPOSER &&
                    control.semantic in COMPOSER_DEDICATED_SEMANTICS -> composerDedicatedCoverage(control)
                control.id in primaryCopyIds -> Coverage(
                    control.id,
                    Kind.DEDICATED,
                    nativeSelector = messageCopySelector(control.contextId.orEmpty()),
                )
                control.id in messageActionIds -> Coverage(
                    control.id,
                    Kind.MENU,
                    nativeSelector = control.accessibilityLabel,
                    nativeTriggerSelector = messageActionsSelector(
                        control.contextId.orEmpty(),
                        messageActionCounts[control.contextId].orZero(),
                    ),
                )
                control.region == ChatGptWebUiRegion.OVERLAY && control.semantic == "timestamp" ->
                    Coverage(control.id, Kind.METADATA)
                control.region == ChatGptWebUiRegion.OVERLAY &&
                    control.semantic in OVERLAY_DEDICATED_SEMANTICS -> overlayDedicatedCoverage(control)
                control.id in overlayIds -> Coverage(
                    control.id,
                    Kind.MENU,
                    nativeSelector = control.accessibilityLabel,
                    nativeTriggerSelector = overlayTrigger,
                )
                else -> Coverage(control.id, Kind.OFFICIAL_FALLBACK)
            }
            control.id to coverage
        }
    }

    fun headerActions(controls: List<ChatGptWebUiControl>): List<ChatGptWebUiControl> = controls.asSequence()
        .filter { it.region == ChatGptWebUiRegion.HEADER }
        .filter { it.semantic !in HEADER_DEDICATED_SEMANTICS }
        .filterNot { it.semantic == "title" }
        .distinctBy(ChatGptWebUiControl::id)
        .take(HEADER_ACTION_LIMIT)
        .toList()

    fun suggestions(controls: List<ChatGptWebUiControl>): List<ChatGptWebUiControl> = controls.asSequence()
        .filter { it.region == ChatGptWebUiRegion.SUGGESTIONS && it.semantic == "suggestion" }
        .distinctBy(ChatGptWebUiControl::id)
        .take(SUGGESTION_LIMIT)
        .toList()

    fun usesHeaderIcon(control: ChatGptWebUiControl): Boolean = control.semantic == "sources"

    fun messageActions(controls: List<ChatGptWebUiControl>): Map<String, List<ChatGptWebUiControl>> =
        controls.asSequence()
            .filter { it.region == ChatGptWebUiRegion.MESSAGE && it.contextId != null }
            .groupBy { it.contextId.orEmpty() }
            .mapValues { (_, grouped) ->
                val primaryCopyId = primaryCopy(grouped)?.id
                grouped.filter { it.id != primaryCopyId }.distinctBy(ChatGptWebUiControl::id)
            }

    fun overlayActions(controls: List<ChatGptWebUiControl>): List<ChatGptWebUiControl> = controls.asSequence()
        .filter { it.region == ChatGptWebUiRegion.OVERLAY && it.enabled }
        .filterNot { it.semantic == "timestamp" }
        .filterNot { it.semantic in OVERLAY_DEDICATED_SEMANTICS }
        .distinctBy(ChatGptWebUiControl::id)
        .take(OVERLAY_ACTION_LIMIT)
        .toList()

    fun messageActionsSelector(contextId: String, count: Int): String =
        "chatgpt-message-actions:${stableContextId(contextId)}:$count"

    fun messageCopySelector(contextId: String): String =
        "chatgpt-message-copy:${stableContextId(contextId)}"

    fun messageRegenerateSelector(contextId: String): String =
        "chatgpt-message-regenerate:${stableContextId(contextId)}"

    fun stableContextId(value: String): String = value
        .replace(Regex("[^A-Za-z0-9_.:-]"), "_")
        .take(MAX_CONTEXT_ID_LENGTH)

    private fun headerDedicatedCoverage(control: ChatGptWebUiControl): Coverage = when (control.semantic) {
        "navigation" -> Coverage(
            control.id,
            Kind.DEDICATED,
            nativeSelector = ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER,
        )
        "new_conversation" -> Coverage(
            control.id,
            Kind.DEDICATED,
            nativeSelector = ChatGptNativeNavigationSelector.NEW_CONVERSATION,
            nativeTriggerSelector = ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER,
        )
        else -> Coverage(
            control.id,
            Kind.DEDICATED,
            nativeSelector = ChatGptNativeNavigationSelector.STOP,
        )
    }

    private fun composerDedicatedCoverage(control: ChatGptWebUiControl): Coverage = Coverage(
        control.id,
        Kind.DEDICATED,
        nativeSelector = when (control.semantic) {
            "attachment" -> ChatGptNativeNavigationSelector.COMPOSER_TOOLS_TRIGGER
            "model" -> ChatGptNativeNavigationSelector.COMPOSER_MODEL_TRIGGER
            "dictation" -> ChatGptNativeNavigationSelector.DICTATION
            "send" -> ChatGptNativeNavigationSelector.SEND
            else -> ChatGptNativeNavigationSelector.STOP
        },
    )

    private fun overlayDedicatedCoverage(control: ChatGptWebUiControl): Coverage = Coverage(
        control.id,
        Kind.DEDICATED,
        nativeTriggerSelector = if (control.semantic == "conversation") {
            ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER
        } else {
            ChatGptNativeNavigationSelector.FEATURE_LIST_TRIGGER
        },
    )

    private fun primaryMessageCopies(controls: List<ChatGptWebUiControl>): List<ChatGptWebUiControl> =
        controls.asSequence()
            .filter { it.region == ChatGptWebUiRegion.MESSAGE && it.contextId != null }
            .groupBy { it.contextId.orEmpty() }
            .values
            .mapNotNull(::primaryCopy)

    private fun primaryCopy(controls: List<ChatGptWebUiControl>): ChatGptWebUiControl? {
        val copies = controls.filter { it.semantic == "copy" }
        return copies.firstOrNull { normalizeLabel(it.label) in PRIMARY_COPY_LABELS }
            ?: copies.singleOrNull()
    }

    private fun normalizeLabel(value: String): String = value.trim().lowercase()

    private fun Int?.orZero(): Int = this ?: 0

    private const val MAX_CONTEXT_ID_LENGTH = 160
    private val HEADER_DEDICATED_SEMANTICS = setOf("navigation", "new_conversation", "stop")
    private val COMPOSER_DEDICATED_SEMANTICS = setOf(
        "attachment",
        "model",
        "dictation",
        "send",
        "stop",
    )
    private val OVERLAY_DEDICATED_SEMANTICS = setOf("conversation", "project")
    private val PRIMARY_COPY_LABELS = setOf(
        "复制回复",
        "复制消息",
        "copy response",
        "copy message",
    )
}
