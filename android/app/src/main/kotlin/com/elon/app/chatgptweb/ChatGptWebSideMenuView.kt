package com.elon.app.chatgptweb

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.elon.app.WebChatLocalProjectActions
import com.elon.app.WebChatLocalProjectDialogs
import com.elon.app.createSocialSidebarDateStrip
import java.time.LocalDate

internal class ChatGptWebSideMenuView(
    private val activity: AppCompatActivity,
    private val index: () -> ChatGptWebConversationIndexState,
    private val refreshIndex: () -> Boolean,
    private val newConversation: () -> Unit,
    private val openConversation: (String) -> Unit,
    private val openProject: (String) -> Unit,
    private val openFeatureNavigation: () -> Unit,
    private val providerId: () -> String,
    private val providerName: () -> String,
    private val localProjectActions: () -> WebChatLocalProjectActions?,
    private val remoteConversationActionsAvailable: () -> Boolean,
    private val openRemoteConversationActions: (ChatGptWebConversation) -> Unit,
    private val openSettings: () -> Unit,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
) : FrameLayout(activity) {
    private val root = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(22), dp(36), dp(18), dp(18))
        setBackgroundColor(Color.parseColor("#0D0D0D"))
    }
    private var selectedTab = ChatGptWebSideMenuTab.DATE
    private var selectedDate = LocalDate.now()
    private var selectedProjectId: String? = null
    private var searchVisible = false
    private var searchQuery = ""
    private var lastRefreshRequestedAtMs = 0L
    private val conversationActions by lazy {
        ChatGptWebSideMenuConversationActions(
            activity = activity,
            index = index,
            localProjectActions = localProjectActions,
            remoteActionsAvailable = remoteConversationActionsAvailable,
            openRemoteActions = openRemoteConversationActions,
            closeThen = ::closeThen,
            render = ::render,
            dp = dp,
            selectableForeground = selectableForeground,
        )
    }

    init {
        addView(root, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
    }

    fun render() {
        root.removeAllViews()
        root.addView(topTabs())
        if (searchVisible) root.addView(searchField())
        if (selectedTab == ChatGptWebSideMenuTab.DATE && searchQuery.isBlank()) {
            root.addView(createSocialSidebarDateStrip(
                context = activity,
                selectedDate = selectedDate,
                onDateSelected = { date ->
                    selectedDate = date
                    render()
                },
                dp = dp,
                selectableForeground = selectableForeground,
                dateContentDescription = ChatGptNativeNavigationSelector::date,
            ))
        }
        root.addView(WebChatSideMenuStateViews.status(activity, index(), selectedDate, dp))
        root.addView(contentScroll(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1f,
        ))
        root.addView(footer())
    }

    fun refresh() {
        render()
        val nowMs = System.currentTimeMillis()
        if (WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen(
                collection = index().collection,
                nowMs = nowMs,
                lastRequestedAtMs = lastRefreshRequestedAtMs,
            )
        ) {
            lastRefreshRequestedAtMs = nowMs
            refreshIndex()
        }
    }

    fun state() = ChatGptWebSideMenuState(selectedTab, selectedDate, selectedProjectId)

    fun selectTab(tab: ChatGptWebSideMenuTab) {
        if (selectedTab == tab && (tab != ChatGptWebSideMenuTab.PROJECTS || selectedProjectId == null)) return
        selectedTab = tab
        selectedProjectId = null
        searchQuery = ""
        render()
    }

    fun selectDate(date: LocalDate) {
        if (selectedTab == ChatGptWebSideMenuTab.DATE && selectedDate == date) return
        selectedTab = ChatGptWebSideMenuTab.DATE
        selectedDate = date
        selectedProjectId = null
        searchQuery = ""
        render()
    }

    fun selectProject(projectId: String): Boolean {
        val project = index().projects.firstOrNull { it.id == projectId } ?: return false
        enterProject(project)
        return true
    }

    private fun topTabs(): LinearLayout = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(48))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(tabText(
            "${selectedDate.monthValue}月${selectedDate.dayOfMonth}号",
            selectedTab == ChatGptWebSideMenuTab.DATE,
            ChatGptNativeNavigationSelector.DATE_TAB,
        ) {
            selectedTab = ChatGptWebSideMenuTab.DATE
            selectedProjectId = null
            searchQuery = ""
            render()
        }, LinearLayout.LayoutParams(dp(92), LinearLayout.LayoutParams.MATCH_PARENT))
        addView(tabText(
            if (localProjectActions() == null) {
                activity.getString(R.string.chatgpt_side_menu_projects)
            } else {
                "本机项目"
            },
            selectedTab == ChatGptWebSideMenuTab.PROJECTS,
            ChatGptNativeNavigationSelector.PROJECTS_TAB,
        ) {
            selectedTab = ChatGptWebSideMenuTab.PROJECTS
            selectedProjectId = null
            searchQuery = ""
            render()
        }, LinearLayout.LayoutParams(dp(58), LinearLayout.LayoutParams.MATCH_PARENT))
        addView(View(activity), LinearLayout.LayoutParams(0, 1, 1f))
        addView(iconButton(R.drawable.social_sidebar_search, "搜索${providerName()}会话") {
            searchVisible = !searchVisible
            if (!searchVisible) searchQuery = ""
            render()
        })
        addView(iconButton(
            android.R.drawable.ic_popup_sync,
            ChatGptNativeNavigationSelector.REFRESH_CONVERSATIONS,
        ) {
            requestIndexRefresh()
        })
        addView(iconButton(R.drawable.ic_side_menu_new_chat, ChatGptNativeNavigationSelector.NEW_CONVERSATION) {
            requestClose(true)
            postDelayed(newConversation, CLOSE_DELAY_MS)
        })
    }

    private fun tabText(
        title: String,
        selected: Boolean,
        description: String,
        onClick: () -> Unit,
    ) = TextView(activity).apply {
        gravity = Gravity.CENTER_VERTICAL or Gravity.START
        includeFontPadding = false
        text = title
        textSize = 18f
        setTextColor(Color.parseColor(if (selected) "#8EA7D5" else "#F8F7F4"))
        contentDescription = "$description:${if (selected) "selected" else "idle"}"
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { onClick() }
    }

    private fun iconButton(resource: Int, description: String, onClick: () -> Unit) =
        ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            setImageResource(resource)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            setPadding(dp(11), dp(11), dp(11), dp(11))
            contentDescription = description
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }

    private fun searchField(): EditText = EditText(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(48)).apply {
            topMargin = dp(8)
            bottomMargin = dp(8)
        }
        background = roundedBackground("#272727", 24)
        setPadding(dp(18), 0, dp(18), 0)
        setTextColor(Color.parseColor("#F8F7F4"))
        setHintTextColor(Color.parseColor("#80BEBEBA"))
        textSize = 15f
        hint = activity.getString(R.string.chatgpt_side_menu_search_hint)
        setSingleLine(true)
        setText(searchQuery)
        setSelection(text.length)
        contentDescription = ChatGptNativeNavigationSelector.SEARCH
        addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                searchQuery = s?.toString().orEmpty()
            }
            override fun afterTextChanged(s: Editable?) {
                post {
                    render()
                    (root.getChildAt(1) as? EditText)?.requestFocus()
                }
            }
        })
    }

    private fun contentScroll() = ScrollView(activity).apply {
        isVerticalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            if (selectedTab == ChatGptWebSideMenuTab.PROJECTS) {
                selectedProject()?.let { renderProjectConversations(this, it) } ?: renderProjects(this)
            } else {
                renderConversations(this)
            }
        })
    }

    private fun renderConversations(container: LinearLayout) {
        val state = index()
        val query = searchQuery.trim()
        if (query.isBlank()) {
            val active = ChatGptWebConversationIndex.activeOn(state.conversations, selectedDate)
            val unassigned = ChatGptWebConversationIndex.unassigned(state.conversations)
            val visibleCount = (active + unassigned).distinctBy { it.id }.size
            val contentStatus = WebChatSideMenuContentState.resolve(
                collection = state.collection,
                availableCount = state.conversations.size,
                visibleCount = visibleCount,
            )
            if (contentStatus != WebChatSideMenuContentStatus.CONTENT) {
                container.addView(contentStateView(
                    contentStatus,
                    emptyMessage = "${providerName()}在这一天暂无会话活动",
                    loadingMessage = "正在读取${providerName()}会话…",
                    failedMessage = "暂时无法读取${providerName()}会话",
                ))
                return
            }
            renderConversationSection(
                container,
                activity.getString(R.string.chatgpt_side_menu_daily_active),
                active,
            )
            renderConversationSection(
                container,
                activity.getString(R.string.chatgpt_side_menu_unassigned),
                unassigned,
            )
            return
        }
        val values = state.conversations.filter { conversation ->
            conversation.title.contains(query, ignoreCase = true) ||
                conversation.projectTitle.orEmpty().contains(query, ignoreCase = true)
        }
        val contentStatus = WebChatSideMenuContentState.resolve(
            collection = state.collection,
            availableCount = state.conversations.size,
            visibleCount = values.size,
        )
        if (contentStatus != WebChatSideMenuContentStatus.CONTENT) {
            container.addView(contentStateView(
                contentStatus,
                emptyMessage = "没有匹配的${providerName()}会话",
                loadingMessage = "正在读取${providerName()}会话…",
                failedMessage = "暂时无法读取${providerName()}会话",
            ))
            return
        }
        ChatGptWebConversationIndex.sections(values).forEach { section ->
            renderConversationSection(container, section.label, section.conversations)
        }
    }

    private fun renderConversationSection(
        container: LinearLayout,
        label: String,
        conversations: List<ChatGptWebConversation>,
    ) {
        if (conversations.isEmpty()) return
        container.addView(sectionLabel(label))
        conversations.forEach { container.addView(conversationRow(it)) }
    }

    private fun renderProjects(container: LinearLayout) {
        val state = index()
        val query = searchQuery.trim()
        val projects = state.projects.filter { project ->
            query.isBlank() || project.title.contains(query, ignoreCase = true)
        }
        val contentStatus = WebChatSideMenuContentState.resolve(
            collection = state.collection,
            availableCount = state.projects.size,
            visibleCount = projects.size,
        )
        if (contentStatus != WebChatSideMenuContentStatus.CONTENT) {
            container.addView(contentStateView(
                contentStatus,
                emptyMessage = if (query.isNotBlank()) {
                    "没有匹配的${providerName()}项目"
                } else if (localProjectActions() == null) {
                    "${providerName()}当前暂无项目"
                } else {
                    "还没有本机项目"
                },
                loadingMessage = "正在读取${providerName()}项目…",
                failedMessage = "暂时无法读取${providerName()}项目",
            ))
            return
        }
        projects.forEach { project ->
            container.addView(projectRow(project))
        }
    }

    private fun renderProjectConversations(container: LinearLayout, project: ChatGptWebProject) {
        val state = index()
        container.addView(projectBackRow(project))
        val all = state.conversations.filter { it.projectId == project.id }
        val query = searchQuery.trim()
        val visible = all.filter { conversation ->
            query.isBlank() || conversation.title.contains(query, ignoreCase = true)
        }
        val contentStatus = WebChatSideMenuContentState.resolve(
            collection = state.collection,
            availableCount = all.size,
            visibleCount = visible.size,
        )
        if (contentStatus != WebChatSideMenuContentStatus.CONTENT) {
            container.addView(contentStateView(
                contentStatus,
                emptyMessage = if (query.isBlank()) {
                    "${project.title}暂无已同步会话"
                } else {
                    "${project.title}没有匹配的会话"
                },
                loadingMessage = "正在读取${project.title}会话…",
                failedMessage = "暂时无法读取${project.title}会话",
            ))
            return
        }
        renderConversationSection(container, activity.getString(R.string.chatgpt_side_menu_project_conversations), visible)
    }

    private fun selectedProject(): ChatGptWebProject? {
        val id = selectedProjectId ?: return null
        return index().projects.firstOrNull { it.id == id }.also { project ->
            if (project == null) selectedProjectId = null
        }
    }

    private fun sectionLabel(value: String) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(38))
        gravity = Gravity.BOTTOM or Gravity.START
        includeFontPadding = false
        text = value
        textSize = 13f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor("#B3DDDBD5"))
    }

    private fun projectRow(project: ChatGptWebProject) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(56))
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        setPadding(dp(4), 0, dp(8), 0)
        text = project.title
        textSize = 16f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor("#F8F7F4"))
        setCompoundDrawablesRelativeWithIntrinsicBounds(R.drawable.ic_side_menu_folder_closed, 0, 0, 0)
        compoundDrawablePadding = dp(12)
        contentDescription = ChatGptNativeNavigationSelector.project(project)
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { enterProject(project) }
    }

    private fun projectBackRow(project: ChatGptWebProject) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(56))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        contentDescription = ChatGptNativeNavigationSelector.projectBack(project)
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener {
            selectedProjectId = null
            searchQuery = ""
            render()
        }
        addView(ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            setImageResource(R.drawable.ic_toolbar_back_custom)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            setPadding(dp(12), dp(12), dp(12), dp(12))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            gravity = Gravity.CENTER_VERTICAL
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = project.title
            textSize = 17f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#F8F7F4"))
        })
    }

    private fun enterProject(project: ChatGptWebProject) {
        selectedTab = ChatGptWebSideMenuTab.PROJECTS
        selectedProjectId = project.id
        searchQuery = ""
        render()
        if (localProjectActions() == null) post { openProject(project.path) }
    }

    private fun conversationRow(
        conversation: ChatGptWebConversation,
        nested: Boolean = false,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(62))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(if (nested) dp(40) else dp(4), dp(8), dp(8), dp(8))
        contentDescription = ChatGptNativeNavigationSelector.conversation(conversation)
        tag = conversation.id
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { closeThen { openConversation(conversation.path) } }
        setOnLongClickListener { conversationActions.show(conversation) }
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                text = conversation.title
                textSize = 15f
                setTextColor(Color.parseColor(if (conversation.active) "#B4C5E3" else "#F8F7F4"))
            })
            val metadata = conversation.projectTitle.orEmpty().takeIf { it.isNotBlank() }
            if (metadata != null) addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                text = metadata
                textSize = 12f
                setPadding(0, dp(5), 0, 0)
                setTextColor(Color.parseColor("#80BEBEBA"))
            })
        })
        if (conversationActions.available()) addView(conversationActions.button(conversation))
    }

    private fun contentStateView(
        status: WebChatSideMenuContentStatus,
        emptyMessage: String,
        loadingMessage: String,
        failedMessage: String,
    ) = WebChatSideMenuStateViews.create(
        activity = activity,
        status = status,
        emptyMessage = emptyMessage,
        loadingMessage = loadingMessage,
        failedMessage = failedMessage,
        onRetry = ::requestIndexRefresh,
        dp = dp,
        selectableForeground = selectableForeground,
    )

    private fun requestIndexRefresh() {
        lastRefreshRequestedAtMs = System.currentTimeMillis()
        refreshIndex()
        render()
    }

    private fun footer() = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(54))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        val projectActions = localProjectActions()
        addView(footerAction(
            if (selectedTab == ChatGptWebSideMenuTab.PROJECTS && projectActions != null) {
                "新建项目"
            } else {
                activity.getString(R.string.web_chat_open_official)
            },
            if (selectedTab == ChatGptWebSideMenuTab.PROJECTS && projectActions != null) {
                "web-chat-local-project-create:${providerId()}"
            } else {
                "web-chat-feature-navigation:${providerId()}"
            },
        ) {
            if (selectedTab == ChatGptWebSideMenuTab.PROJECTS && projectActions != null) {
                WebChatLocalProjectDialogs.showCreate(activity, projectActions, ::render)
            } else {
                requestClose(true)
                postDelayed(openFeatureNavigation, CLOSE_DELAY_MS)
            }
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        addView(footerAction(
            activity.getString(R.string.chatgpt_side_menu_settings),
            "web-chat-settings:${providerId()}",
            openSettings,
        ),
            LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
    }

    private fun footerAction(
        title: String,
        description: String,
        onClick: () -> Unit,
    ) = TextView(activity).apply {
        gravity = Gravity.CENTER_VERTICAL or Gravity.START
        includeFontPadding = false
        text = title
        contentDescription = description
        textSize = 14f
        setTextColor(Color.parseColor("#B3DDDBD5"))
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { onClick() }
    }

    private fun closeThen(action: () -> Unit) {
        requestClose(true)
        postDelayed(action, CLOSE_DELAY_MS)
    }

    private fun roundedBackground(color: String, radiusDp: Int) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(radiusDp).toFloat()
        setColor(Color.parseColor(color))
    }

    private companion object {
        const val CLOSE_DELAY_MS = 180L
    }
}
