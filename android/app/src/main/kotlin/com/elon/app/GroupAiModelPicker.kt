package com.elon.app

import android.os.Handler
import android.os.Looper
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.GroupWebAiModelControls
import com.elon.app.chatgptweb.GroupWebAiSession
import com.elon.app.chatgptweb.GroupAiModelPort
import java.util.UUID

/** Reuses the personal-chat presentation, not its document, draft or preference owner. */
internal class GroupAiModelPicker(
    private val activity: AppCompatActivity,
    private val anchor: View,
    private val cache: WebChatProductionInteractionCache,
    private val configuration: GroupAiConfiguration,
    private val select: (List<GroupAiModelChoice>) -> Unit,
    private val switchProvider: () -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private val controls = GroupWebAiModelControls()
    private val provider = WebChatProviderId.CHATGPT_WEB
    private var popup: WebChatModelControlPopupHandle? = null
    private var session: GroupWebAiSession? = null
    private var path = emptyList<GroupAiModelChoice>()
    private var displayed = emptyList<WebChatConsumerOption>()
    private var pendingSubmenu: GroupAiModelChoice? = null
    private var command: String? = null
    private var ready = false
    private var closed = false
    private var failed = false
    private val pendingControls = mutableListOf<ChatGptWebEvent>()
    private val deadline = Runnable { unavailable() }

    fun show() {
        displayed = cached()
        popup = WebChatModelControlPopup.show(activity, anchor, marked(displayed), configuration.label,
            onOptionSelected = ::choose, onProviderSwitch = switchProvider, onDismissed = ::close,
            providerSwitchLabel = "切换 AI")
        if (popup == null) return
        handler.postDelayed(deadline, 40_000)
        runCatching { session = GroupWebAiSession(activity, ::event, ::unavailable).also { it.start() } }
            .onFailure { unavailable() }
    }

    private fun section() = if (path.isEmpty()) "model" else "model_${GroupAiConfigurationStore.key(path.joinToString { it.label }).take(16)}"
    private fun cached() = cache.composerOptions(provider, section(), emptyList())
    private fun marked(options: List<WebChatConsumerOption>) = options.map {
        it.copy(selected = !it.opensSubmenu && it.label == configuration.modelPath.lastOrNull()?.label)
    }

    private fun render() {
        val live = controls.displayed
        if (live.isNotEmpty()) {
            cache.composerOptions(provider, section(), live)
            if (pendingSubmenu == null && command == null) handler.removeCallbacks(deadline)
        }
        displayed = live.ifEmpty { cached() }
        popup?.update(marked(displayed), configuration.label)
    }

    private fun choose(option: WebChatConsumerOption) {
        val choice = GroupWebAiModelControls.choice(option, displayed)
        if (!choice.submenu) {
            select(path + choice)
            close()
            return
        }
        if (failed) { unavailable(); return }
        if (pendingSubmenu != null) return
        pendingSubmenu = choice
        handler.removeCallbacks(deadline)
        handler.postDelayed(deadline, 40_000)
        openPending()
    }

    private fun openPending() {
        if (!ready || command != null) return
        val expected = pendingSubmenu ?: return
        val live = controls.resolve(expected) ?: return
        command = UUID.randomUUID().toString()
        controls.select(GroupAiModelPort.from(requireNotNull(session).adapter), live, requireNotNull(command))
    }

    private fun event(event: ChatGptWebEvent) {
        if (closed || failed) return
        if (event is ChatGptWebEvent.Snapshot) {
            if (event.value.loginRequired) { unavailable(); return }
            if (!ready && session?.isReady(event.value) == true) {
                ready = true
                session?.adapter?.listModelOptions()
                session?.adapter?.requestUiManifest()
            }
        }
        if (event is ChatGptWebEvent.CommandResult && command != null && command == event.requestId) {
            command = null
            if (!event.ok) { unavailable(); return }
            pendingSubmenu?.let { choice ->
                val existing = path.indexOfFirst { it.label == choice.label }
                path = if (existing >= 0) path.take(existing + 1) else (path + choice).take(5)
            }
            pendingSubmenu = null
            controls.clear()
            pendingControls.forEach(controls::accept)
            pendingControls.clear()
            render()
            if (controls.displayed.isEmpty()) session?.adapter?.collectModelOptions()
            session?.adapter?.requestUiManifest()
            return
        }
        if (command != null) {
            if (event is ChatGptWebEvent.ComposerControls || event is ChatGptWebEvent.UiManifest) {
                pendingControls.removeAll { it::class == event::class }
                pendingControls.add(event)
            }
            return
        }
        controls.accept(event)
        if (event is ChatGptWebEvent.ComposerControls || event is ChatGptWebEvent.UiManifest) {
            render()
            openPending()
        }
    }

    private fun unavailable() {
        if (closed) return
        failed = true
        handler.removeCallbacks(deadline)
        session?.close()
        session = null
        Toast.makeText(activity, "模型配置暂未同步，可使用缓存档位；发送前会确认设置", Toast.LENGTH_LONG).show()
    }

    fun close() {
        if (closed) return
        closed = true
        handler.removeCallbacks(deadline)
        popup?.dismiss()
        popup = null
        session?.close()
        session = null
    }
}
