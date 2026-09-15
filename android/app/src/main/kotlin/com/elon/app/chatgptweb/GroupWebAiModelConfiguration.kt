package com.elon.app.chatgptweb

import com.elon.app.ChatGptConsumerModelOptionMapper
import com.elon.app.GroupAiModelChoice
import com.elon.app.WebChatConsumerOption
import com.elon.app.WebChatModelControlPolicy
import com.elon.app.WebChatModelRangeBinding
import com.elon.app.WebChatModelRangePolicy
import java.util.UUID

internal interface GroupAiModelPort {
    fun list()
    fun collect()
    fun manifest()
    fun select(id: String, request: String)
    fun slider(id: String, value: Double, request: String)
    fun dismiss(request: String)

    companion object {
        fun from(adapter: ChatGptWebPageAdapter) = object : GroupAiModelPort {
            override fun list() { adapter.listModelOptions() }
            override fun collect() { adapter.collectModelOptions() }
            override fun manifest() { adapter.requestUiManifest() }
            override fun select(id: String, request: String) { adapter.selectModelOption(id, request) }
            override fun slider(id: String, value: Double, request: String) { adapter.setUiControlSlider(id, value, request) }
            override fun dismiss(request: String) { adapter.dismissComposerMenu(request) }
        }
    }
}

internal class GroupWebAiModelControls {
    var options: List<WebChatConsumerOption> = emptyList(); private set
    var range: WebChatModelRangeBinding? = null; private set
    val displayed: List<WebChatConsumerOption> get() = range?.options?.let { levels ->
        levels + options.filter { it.opensSubmenu }
    } ?: options

    fun accept(event: ChatGptWebEvent) {
        when (event) {
            is ChatGptWebEvent.ComposerControls -> if (event.section == "model" && event.options.isNotEmpty()) {
                options = event.options.mapNotNull(ChatGptConsumerModelOptionMapper::map)
            }
            is ChatGptWebEvent.UiManifest -> range = WebChatModelRangePolicy.resolve(event.value.controls)
            else -> Unit
        }
    }

    fun clear() { options = emptyList(); range = null }

    fun resolve(choice: GroupAiModelChoice): WebChatConsumerOption? {
        if (choice.rangeIndex != null) {
            val levels = range?.options ?: return null
            return levels.takeIf { it.size == choice.rangeCount }?.getOrNull(choice.rangeIndex)
        }
        val expected = WebChatConsumerOption("intent", choice.label, false, "model", choice.submenu, "")
        return WebChatModelControlPolicy.resolveSelection(expected, options, options.map { it.id }.toSet())
    }

    fun select(port: GroupAiModelPort, option: WebChatConsumerOption, requestId: String) {
        val step = range?.selections?.get(option.id)
        if (step != null) port.slider(step.controlId, step.value, requestId)
        else port.select(option.id, requestId)
    }

    companion object {
        fun choice(option: WebChatConsumerOption, displayed: List<WebChatConsumerOption>): GroupAiModelChoice {
            val levels = displayed.filter { it.id.startsWith("model-range:") }
            val index = levels.indexOfFirst { it.id == option.id }.takeIf { it >= 0 }
            return GroupAiModelChoice(option.label, option.opensSubmenu, index, levels.size.takeIf { index != null })
        }
    }
}

/** Applies the frozen intent to live controls before the one-use send authorization. */
internal class GroupWebAiModelConfiguration(
    private val port: GroupAiModelPort,
    private val path: List<GroupAiModelChoice>,
    private val onReady: () -> Unit,
    private val onFailure: () -> Unit,
) {
    private val controls = GroupWebAiModelControls()
    private var index = 0
    private var pending: String? = null
    private var closing = false
    private var finished = false
    private val pendingControls = mutableListOf<ChatGptWebEvent>()

    fun start() {
        if (path.isEmpty()) { finished = true; onReady(); return }
        port.list()
        port.manifest()
    }

    fun event(event: ChatGptWebEvent) {
        if (finished) return
        if (event is ChatGptWebEvent.CommandResult && pending != null && pending == event.requestId) {
            pending = null
            if (!event.ok) { finished = true; onFailure(); return }
            if (closing) { finished = true; onReady(); return }
            index++
            controls.clear()
            // The private adapter emits the new catalog before its command receipt.
            pendingControls.forEach(controls::accept)
            pendingControls.clear()
            if (index == path.size) {
                closing = true
                pending = UUID.randomUUID().toString()
                port.dismiss(requireNotNull(pending))
            } else {
                selectNext()
                if (pending == null) { port.collect(); port.manifest() }
            }
            return
        }
        if (pending != null && !closing &&
            (event is ChatGptWebEvent.ComposerControls || event is ChatGptWebEvent.UiManifest)) {
            pendingControls.removeAll { it::class == event::class }
            pendingControls.add(event)
            return
        }
        controls.accept(event)
        selectNext()
    }

    private fun selectNext() {
        if (pending != null || closing) return
        val option = path.getOrNull(index)?.let(controls::resolve) ?: return
        pending = UUID.randomUUID().toString()
        controls.select(port, option, requireNotNull(pending))
    }
}
