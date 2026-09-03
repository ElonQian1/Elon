package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.DebugTraceStore
import com.elon.app.WebChatBackgroundInteractionKind

internal class ChatGptWebTouchRequestHandler(
    private val webView: () -> WebView?,
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    private val touchDispatcher: () -> ChatGptWebTouchDispatcher?,
    private val isInteractiveSurface: () -> Boolean,
    private val runBackgroundInteraction: (WebChatBackgroundInteractionKind, () -> Unit) -> Boolean,
    private val interactionRequested: () -> Unit,
    private val dismissComposerOptions: () -> Unit,
    private val scheduleModelOptions: () -> Unit,
    private val scheduleToolOptions: () -> Unit,
    private val onDispatchFailed: () -> Unit,
) {
    fun handle(event: ChatGptWebEvent.WebTouchRequest) {
        val view = webView() ?: return
        val adapter = pageAdapter() ?: return
        interactionRequested()
        val dispatch = {
            dispatchTouch(event, view, adapter)
        }
        val interactive = isInteractiveSurface()
        val interactionKind = interactionKind(event.purpose)
        val route = when {
            interactive -> {
                dispatch()
                "interactive"
            }
            runBackgroundInteraction(interactionKind, dispatch) -> when (interactionKind) {
                WebChatBackgroundInteractionKind.TRANSIENT -> "background_lease"
                WebChatBackgroundInteractionKind.DICTATION_START -> "background_dictation_start"
                WebChatBackgroundInteractionKind.DICTATION_FINISH -> "background_dictation_finish"
            }
            else -> {
                dispatch()
                "background_direct"
            }
        }
        DebugTraceStore.record(
            "web_chat_touch_request",
            mapOf("purpose" to event.purpose, "route" to route),
        )
    }

    private fun interactionKind(purpose: String): WebChatBackgroundInteractionKind = when (purpose) {
        "start_dictation" -> WebChatBackgroundInteractionKind.DICTATION_START
        "cancel_dictation", "submit_dictation" ->
            WebChatBackgroundInteractionKind.DICTATION_FINISH
        else -> WebChatBackgroundInteractionKind.TRANSIENT
    }

    private fun dispatchTouch(
        event: ChatGptWebEvent.WebTouchRequest,
        view: WebView,
        adapter: ChatGptWebPageAdapter,
    ) {
        touchDispatcher()?.dispatch(event) { dispatched ->
            DebugTraceStore.record(
                "web_chat_touch_dispatch",
                mapOf("purpose" to event.purpose, "dispatched" to dispatched),
            )
            if (!dispatched) {
                chatGptComposerSectionForAction(event.purpose)?.let { dismissComposerOptions() }
                onDispatchFailed()
                return@dispatch
            }
            scheduleFollowUp(view, adapter, event.purpose)
        }
    }

    private fun scheduleFollowUp(
        view: WebView,
        adapter: ChatGptWebPageAdapter,
        purpose: String,
    ) {
        when (purpose) {
            "list_model_options" -> scheduleModelOptions()
            "list_composer_tools" -> scheduleToolOptions()
            "open_model_submenu" -> view.postDelayed(
                adapter::collectModelOptions,
                ChatGptWebInteractionTimings.COMPOSER_MENU_SETTLE_MS,
            )
            "open_composer_tools_submenu" -> view.postDelayed(
                adapter::collectComposerTools,
                ChatGptWebInteractionTimings.COMPOSER_MENU_SETTLE_MS,
            )
            "list_navigation" -> view.postDelayed(
                adapter::collectFeatures,
                ChatGptWebInteractionTimings.NAVIGATION_SETTLE_MS,
            )
            "select_model_option", "select_composer_tool", "remove_attachment" -> view.postDelayed(
                adapter::requestSnapshot,
                ChatGptWebInteractionTimings.COMPOSER_MENU_SETTLE_MS,
            )
            "start_dictation" -> DICTATION_START_SNAPSHOT_DELAYS_MS.forEach { delay ->
                view.postDelayed(adapter::requestSnapshot, delay)
            }
            "cancel_dictation", "submit_dictation" ->
                DICTATION_FINISH_SNAPSHOT_DELAYS_MS.forEach { delay ->
                    view.postDelayed(adapter::requestSnapshot, delay)
                }
            "select_navigation", "invoke_ui_control", "regenerate_open_menu", "regenerate_retry" ->
                view.postDelayed(
                    adapter::requestSnapshot,
                    ChatGptWebInteractionTimings.NAVIGATION_SETTLE_MS,
                )
        }
    }

    private companion object {
        val DICTATION_START_SNAPSHOT_DELAYS_MS = longArrayOf(240L, 700L, 1_500L, 3_000L)
        val DICTATION_FINISH_SNAPSHOT_DELAYS_MS = longArrayOf(240L, 700L, 1_500L)
    }
}
