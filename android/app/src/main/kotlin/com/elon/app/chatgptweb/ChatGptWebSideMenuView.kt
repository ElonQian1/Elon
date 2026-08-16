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
    private var searchVisible = false
    private var searchQuery = ""
    private val collapsedProjectIds = mutableSetOf<String>()

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
        root.addView(statusRow())
        root.addView(contentScroll(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1f,
        ))
        root.addView(footer())
    }

    fun refresh() {
        render()
        refreshIndex()
    }

    fun state() = ChatGptWebSideMenuState(selectedTab, selectedDate)

    fun selectTab(tab: ChatGptWebSideMenuTab) {
        if (selectedTab == tab) return
        selectedTab = tab
        render()
    }

    fun selectDate(date: LocalDate) {
        if (selectedTab == ChatGptWebSideMenuTab.DATE && selectedDate == date) return
        selectedTab = ChatGptWebSideMenuTab.DATE
        selectedDate = date
        render()
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
            render()
        }, LinearLayout.LayoutParams(dp(58), LinearLayout.LayoutParams.MATCH_PARENT))
        addView(View(activity), LinearLayout.LayoutParams(0, 1, 1f))
        addView(iconButton(R.drawable.social_sidebar_search, "搜索${providerName()}会话") {
            searchVisible = !searchVisible
            if (!searchVisible) searchQuery = ""
            render()
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
            layoutParams = LinearLayout.LayoutParams(dp(40), dp(48))
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

    private fun statusRow() = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        textSize = 12f
        setTextColor(Color.parseColor("#80BEBEBA"))
        val state = index()
        val activeCount = ChatGptWebConversationIndex.activeOn(state.conversations, selectedDate).size
        val unassignedCount = state.conversations.count { it.projectId == null }
        val countLabel = "$activeCount 个活跃 · $unassignedCount 个未归项目 · 共 ${state.conversations.size} 个会话"
        text = when (state.collection.officialLoadState) {
            ChatGptWebConversationCollection.LOAD_LOADING -> "$countLabel · 正在刷新"
            ChatGptWebConversationCollection.LOAD_FAILED -> "$countLabel · 显示本机缓存"
            else -> countLabel
        }
        contentDescription = ChatGptNativeNavigationSelector.STATUS
    }

    private fun contentScroll() = ScrollView(activity).apply {
        isVerticalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            if (selectedTab == ChatGptWebSideMenuTab.PROJECTS) renderProjects(this) else renderConversations(this)
        })
    }

    private fun renderConversations(container: LinearLayout) {
        val state = index()
        val query = searchQuery.trim()
        if (query.isBlank()) {
            val active = ChatGptWebConversationIndex.activeOn(state.conversations, selectedDate)
            val unassigned = ChatGptWebConversationIndex.unassigned(state.conversations)
            if (active.isEmpty() && unassigned.isEmpty()) {
                container.addView(emptyState("${providerName()}在这一天暂无会话活动"))
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
        if (values.isEmpty()) {
            container.addView(emptyState("没有匹配的${providerName()}会话"))
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
        if (projects.isEmpty()) {
            container.addView(emptyState(
                if (localProjectActions() == null) {
                    "${providerName()}当前暂无项目"
                } else {
                    "还没有本机项目"
                },
            ))
            return
        }
        projects.forEach { project ->
            container.addView(projectRow(project))
            if (project.id !in collapsedProjectIds) {
                state.conversations
                    .filter { it.projectId == project.id }
                    .forEach { container.addView(conversationRow(it, nested = true)) }
            }
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
        setCompoundDrawablesRelativeWithIntrinsicBounds(R.drawable.ic_side_menu_project, 0, 0, 0)
        compoundDrawablePadding = dp(12)
        contentDescription = ChatGptNativeNavigationSelector.project(project)
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener {
            if (localProjectActions() == null) {
                closeThen { openProject(project.path) }
            } else {
                if (!collapsedProjectIds.add(project.id)) collapsedProjectIds.remove(project.id)
                render()
            }
        }
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
        setOnLongClickListener {
            val actions = localProjectActions() ?: return@setOnLongClickListener false
            WebChatLocalProjectDialogs.showAssignment(activity, index(), conversation, actions, ::render)
            true
        }
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
        if (localProjectActions() != null) addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(40), LinearLayout.LayoutParams.MATCH_PARENT)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "⋯"
            textSize = 22f
            setTextColor(Color.parseColor("#B3DDDBD5"))
            contentDescription = "整理会话：${conversation.title}"
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener {
                localProjectActions()?.let { actions ->
                    WebChatLocalProjectDialogs.showAssignment(activity, index(), conversation, actions, ::render)
                }
            }
        })
    }

    private fun emptyState(message: String) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(160))
        gravity = Gravity.CENTER
        includeFontPadding = false
        text = message
        textSize = 14f
        setTextColor(Color.parseColor("#80BEBEBA"))
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
