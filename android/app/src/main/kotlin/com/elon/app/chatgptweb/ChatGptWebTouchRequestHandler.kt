package com.elon.app.chatgptweb

import android.webkit.WebView

internal class ChatGptWebTouchRequestHandler(
    private val webView: () -> WebView?,
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    private val touchDispatcher: () -> ChatGptWebTouchDispatcher?,
    private val isInteractiveSurface: () -> Boolean,
    private val runBackgroundInteraction: ((() -> Unit) -> Boolean),
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
        if (isInteractiveSurface() || !runBackgroundInteraction(dispatch)) dispatch()
    }

    private fun dispatchTouch(
        event: ChatGptWebEvent.WebTouchRequest,
        view: WebView,
        adapter: ChatGptWebPageAdapter,
    ) {
        touchDispatcher()?.dispatch(event) { dispatched ->
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
            "select_model_option", "select_composer_tool", "remove_attachment",
            "start_dictation", "cancel_dictation", "submit_dictation" -> view.postDelayed(
                adapter::requestSnapshot,
                ChatGptWebInteractionTimings.COMPOSER_MENU_SETTLE_MS,
            )
            "select_navigation", "invoke_ui_control", "regenerate_open_menu", "regenerate_retry" ->
                view.postDelayed(
                    adapter::requestSnapshot,
                    ChatGptWebInteractionTimings.NAVIGATION_SETTLE_MS,
                )
        }
    }
}
