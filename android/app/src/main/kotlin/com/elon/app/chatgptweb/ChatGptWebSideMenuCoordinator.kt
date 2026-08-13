package com.elon.app.chatgptweb

import android.graphics.drawable.Drawable
import android.view.View
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity

internal class ChatGptWebSideMenuCoordinator(
    private val activity: AppCompatActivity,
    private val index: () -> ChatGptWebConversationIndexState,
    private val refreshIndex: () -> Boolean,
    private val newConversation: () -> Unit,
    private val openConversation: (String) -> Unit,
    private val openProject: (String) -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val active: () -> Boolean,
) {
    private lateinit var view: ChatGptWebSideMenuView

    fun attach(
        panel: FrameLayout,
        requestClose: (Boolean) -> Unit,
        openSettings: () -> Unit,
        dp: (Int) -> Int,
        selectableForeground: () -> Drawable?,
    ) {
        view = ChatGptWebSideMenuView(
            activity = activity,
            index = index,
            refreshIndex = refreshIndex,
            newConversation = newConversation,
            openConversation = openConversation,
            openProject = openProject,
            openOfficialFallback = openOfficialFallback,
            openSettings = openSettings,
            requestClose = requestClose,
            dp = dp,
            selectableForeground = selectableForeground,
        )
        panel.addView(view, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT,
        ))
        view.visibility = View.GONE
    }

    fun show() {
        val opening = view.visibility != View.VISIBLE
        view.visibility = View.VISIBLE
        if (opening) view.refresh() else view.render()
    }

    fun isActive(): Boolean = active()

    fun onIndexChanged() {
        if (::view.isInitialized && view.visibility == View.VISIBLE) view.render()
    }

    fun hide() {
        view.visibility = View.GONE
    }
}
