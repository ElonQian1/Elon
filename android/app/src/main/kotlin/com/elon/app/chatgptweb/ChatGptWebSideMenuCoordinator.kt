package com.elon.app.chatgptweb

import android.graphics.drawable.Drawable
import android.view.View
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.WebChatLocalProjectActions
import java.time.LocalDate

internal enum class ChatGptWebSideMenuTab(val wireValue: String) {
    DATE("date"),
    PROJECTS("projects");

    companion object {
        fun parse(value: String): ChatGptWebSideMenuTab? = entries.firstOrNull {
            it.wireValue == value.trim().lowercase()
        }
    }
}

internal data class ChatGptWebSideMenuState(
    val tab: ChatGptWebSideMenuTab,
    val date: LocalDate,
    val selectedProjectId: String? = null,
)

internal class ChatGptWebSideMenuCoordinator(
    private val activity: AppCompatActivity,
    private val index: () -> ChatGptWebConversationIndexState,
    private val refreshIndex: (String?) -> Boolean,
    private val newConversation: () -> Unit,
    private val openConversation: (String) -> Unit,
    private val openProject: (String) -> Unit,
    private val openFeatureNavigation: () -> Unit,
    private val providerId: () -> String,
    private val providerName: () -> String,
    private val localProjectActions: () -> WebChatLocalProjectActions?,
    private val remoteConversationActionsAvailable: () -> Boolean,
    private val openRemoteConversationActions: (ChatGptWebConversation) -> Unit,
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
            openFeatureNavigation = openFeatureNavigation,
            providerId = providerId,
            providerName = providerName,
            localProjectActions = localProjectActions,
            remoteConversationActionsAvailable = remoteConversationActionsAvailable,
            openRemoteConversationActions = openRemoteConversationActions,
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

    fun state(): ChatGptWebSideMenuState? =
        if (::view.isInitialized) view.state() else null

    fun selectTab(tab: ChatGptWebSideMenuTab): Boolean {
        if (!active() || !::view.isInitialized) return false
        view.selectTab(tab)
        return true
    }

    fun selectDate(date: LocalDate): Boolean {
        if (!active() || !::view.isInitialized) return false
        view.selectDate(date)
        return true
    }

    fun selectProject(projectId: String): Boolean {
        if (!active() || !::view.isInitialized) return false
        return view.selectProject(projectId)
    }

    fun hide() {
        view.visibility = View.GONE
    }
}
