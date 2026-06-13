package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.inputmethod.InputMethodManager
import android.view.animation.DecelerateInterpolator
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class MainHomeListActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: () -> String,
    private val projects: () -> List<AppProject>,
    private val conversations: () -> List<AppConversation>,
    private val friends: () -> List<AppFriend>,
    private val groups: () -> List<AppGroup>,
    private val activeProject: () -> AppProject,
    private val compactProjectTitle: () -> String,
    private val formatTime: (Long) -> String,
    private val isTaskRunning: (String, String) -> Boolean,
    private val homeRows: () -> MainHomeRows,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val showCreateProjectDialog: () -> Unit,
    private val showProjectPlaza: () -> Unit,
    private val openProject: (Int) -> Unit,
    private val openProjectConversations: (Int) -> Unit,
    private val showProjectActions: (Int, View?) -> Unit,
    private val showAddFriendDialog: () -> Unit,
    private val openFriend: (AppFriend) -> Unit,
    private val openGroup: (AppGroup) -> Unit
) {
    private var friendSearchActive = false
    private var friendSearchQuery = ""
    private var shouldFocusFriendSearch = false
    private var animateFriendSearchEnter = false
    private var personalProjectsExpanded = false
    private var jointProjectsExpanded = false
    private var plazaBannerProjects: List<StoreProject> = emptyList()
    private var plazaBannerLoading = false
    private var plazaBannerLoaded = false

    fun showFriendLocalSearch() {
        if (binding.conversationPage.visibility != View.VISIBLE || binding.chatPage.visibility == View.VISIBLE) return
        friendSearchActive = true
        friendSearchQuery = ""
        shouldFocusFriendSearch = true
        animateFriendSearchEnter = true
        renderConversationList()
    }

    fun exitFriendLocalSearch(): Boolean {
        if (!friendSearchActive) return false
        clearFriendSearchState()
        renderConversationList()
        return true
    }

    fun renderConversationList() {
        val listVisible = binding.conversationPage.visibility == View.VISIBLE &&
            binding.chatPage.visibility != View.VISIBLE
        if (!listVisible && friendSearchActive) {
            clearFriendSearchState()
        }
        if (listVisible) {
            binding.topTitleText.text = if (friendSearchActive) "搜索" else "好友"
            binding.searchButton.visibility = if (friendSearchActive) View.GONE else View.VISIBLE
            binding.addButton.visibility = if (friendSearchActive) View.GONE else View.VISIBLE
        }

        homeRows().cancelHomeRowShimmer()
        binding.conversationPage.removeAllViews()
        if (friendSearchActive) {
            binding.conversationPage.addView(createFriendSearchHeader())
            renderFriendSearchResults()
            return
        }
        val chatItems = buildHomeChatItems()
        if (chatItems.isEmpty()) {
            binding.conversationPage.addView(
                homeRows().createFriendPlaceholder(AuthManager.isLoggedIn(activity)) {
                    showAddFriendDialog()
                }
            )
            return
        }
        renderHomeChatItems(chatItems)
    }

    private fun renderFriendSearchResults() {
        val resultStartIndex = if (friendSearchActive) 1 else 0
        while (binding.conversationPage.childCount > resultStartIndex) {
            binding.conversationPage.removeViewAt(resultStartIndex)
        }
        val allChatItems = buildHomeChatItems()
        val chatItems = filterHomeChatItems(allChatItems)
        if (chatItems.isEmpty()) {
            if (friendSearchActive) {
                binding.conversationPage.addView(createFriendSearchEmptyRow())
                return
            }
        }
        renderHomeChatItems(chatItems)
    }

    private fun renderHomeChatItems(chatItems: List<HomeChatItem>) {
        chatItems.forEachIndexed { index, item ->
            if (index > 0) {
                binding.conversationPage.addView(homeRows().createConversationDivider())
            }
            when (item) {
                is HomeChatItem.FriendItem -> binding.conversationPage.addView(
                    homeRows().createFriendRow(item.friend) {
                        clearFriendSearchState()
                        openFriend(item.friend)
                    }
                )
                is HomeChatItem.GroupItem -> binding.conversationPage.addView(
                    homeRows().createGroupRow(item.group) {
                        clearFriendSearchState()
                        openGroup(item.group)
                    }
                )
            }
        }
    }

    private fun clearFriendSearchState() {
        if (friendSearchActive) {
            hideFriendSearchKeyboard()
        }
        friendSearchActive = false
        friendSearchQuery = ""
        shouldFocusFriendSearch = false
        animateFriendSearchEnter = false
    }

    private fun hideFriendSearchKeyboard() {
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.conversationPage.windowToken, 0)
    }

    private fun filterHomeChatItems(items: List<HomeChatItem>): List<HomeChatItem> {
        val query = normalizeSearch(friendSearchQuery)
        if (!friendSearchActive || query.isBlank()) return items
        return items.filter { item ->
            when (item) {
                is HomeChatItem.FriendItem -> matchesQuery(
                    query,
                    item.friend.name,
                    item.friend.account,
                    item.friend.phone,
                    item.friend.lastMessage
                )
                is HomeChatItem.GroupItem -> matchesQuery(
                    query,
                    item.group.name,
                    item.group.lastMessage,
                    "${item.group.memberCount} 位成员",
                    item.group.members.joinToString(" ") { member -> member.displayName }
                )
            }
        }
    }

    private fun matchesQuery(query: String, vararg values: String?): Boolean {
        return values.any { value -> normalizeSearch(value).contains(query) }
    }

    private fun normalizeSearch(value: String?): String {
        return value.orEmpty().trim().lowercase()
    }

    private fun createFriendSearchHeader(): LinearLayout {
        val targetHeight = dp(58)
        val animateEnter = animateFriendSearchEnter
        animateFriendSearchEnter = false
        val root = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                if (animateEnter) 0 else targetHeight
            )
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(14), dp(8), dp(14), dp(8))
            setBackgroundColor(Color.parseColor("#101010"))
            alpha = if (animateEnter) 0f else 1f
            translationY = if (animateEnter) -dp(8).toFloat() else 0f
            clipToPadding = false
        }
        val input = EditText(activity).apply {
            setText(friendSearchQuery)
            setSingleLine(true)
            textSize = 15f
            hint = "搜索好友、群聊、最近消息"
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#777777"))
            background = null
            setPadding(dp(12), 0, dp(12), 0)
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
                override fun afterTextChanged(s: Editable?) {
                    val nextQuery = s?.toString()?.trim().orEmpty()
                    if (nextQuery == friendSearchQuery) return
                    friendSearchQuery = nextQuery
                    renderFriendSearchResults()
                }
            })
        }
        val searchBox = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            background = roundedRect("#222222", 8, "#2E2E2E")
            addView(TextView(activity).apply {
                text = "⌕"
                textSize = 18f
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#A8A8A8"))
            }, LinearLayout.LayoutParams(dp(30), LinearLayout.LayoutParams.MATCH_PARENT))
            addView(input, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        }
        val cancel = TextView(activity).apply {
            text = "取消"
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#58BE6A"))
            setOnClickListener { exitFriendLocalSearch() }
        }
        root.addView(searchBox)
        root.addView(cancel, LinearLayout.LayoutParams(dp(54), LinearLayout.LayoutParams.MATCH_PARENT).apply {
            marginStart = dp(10)
        })
        if (animateEnter) {
            animateSearchHeaderIn(root, targetHeight)
        }
        if (shouldFocusFriendSearch) {
            shouldFocusFriendSearch = false
            input.postDelayed({
                input.requestFocus()
                input.setSelection(input.text?.length ?: 0)
                val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
                imm?.showSoftInput(input, InputMethodManager.SHOW_IMPLICIT)
            }, if (animateEnter) 90L else 0L)
        }
        return root
    }

    private fun animateSearchHeaderIn(header: View, targetHeight: Int) {
        header.post {
            val params = header.layoutParams as LinearLayout.LayoutParams
            ValueAnimator.ofInt(0, targetHeight).apply {
                duration = 180L
                interpolator = DecelerateInterpolator()
                addUpdateListener { animator ->
                    val progress = animator.animatedFraction
                    params.height = animator.animatedValue as Int
                    header.layoutParams = params
                    header.alpha = progress
                    header.translationY = -dp(8) * (1f - progress)
                }
                start()
            }
        }
    }

    private fun createFriendSearchEmptyRow(): View {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(92)
            )
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#777777"))
            textSize = 14f
            text = if (friendSearchQuery.isBlank()) {
                "输入关键词搜索本地好友内容"
            } else {
                "没有匹配的好友或群聊"
            }
        }
    }

    private fun roundedRect(fillColor: String, radiusDp: Int, strokeColor: String? = null): GradientDrawable =
        GradientDrawable().apply {
            setColor(Color.parseColor(fillColor))
            cornerRadius = dp(radiusDp).toFloat()
            strokeColor?.let { setStroke(dp(1), Color.parseColor(it)) }
        }

    private fun buildHomeChatItems(): List<HomeChatItem> {
        val friendItems = friends().map { friend ->
            HomeChatItem.FriendItem(
                friend = friend,
                sortTime = friend.lastMessageAt ?: 0L
            )
        }
        val groupItems = groups().map { group ->
            HomeChatItem.GroupItem(
                group = group,
                sortTime = group.lastMessageAt ?: group.createdAt ?: 0L
            )
        }
        return (groupItems + friendItems)
            .sortedWith(compareByDescending<HomeChatItem> { it.sortTime }.thenBy { item ->
                when (item) {
                    is HomeChatItem.FriendItem -> item.friend.name
                    is HomeChatItem.GroupItem -> item.group.name
                }
            })
    }

    private sealed class HomeChatItem(open val sortTime: Long) {
        data class FriendItem(val friend: AppFriend, override val sortTime: Long) : HomeChatItem(sortTime)
        data class GroupItem(val group: AppGroup, override val sortTime: Long) : HomeChatItem(sortTime)
    }

    fun renderProjectList() {
        ensurePlazaBannerProjects()
        ProjectManagementHomeView(
            activity = activity,
            container = binding.projectContentLayout,
            projects = projects,
            plazaProjects = { plazaBannerProjects },
            personalProjectsExpanded = { personalProjectsExpanded },
            jointProjectsExpanded = { jointProjectsExpanded },
            setPersonalProjectsExpanded = { personalProjectsExpanded = it },
            setJointProjectsExpanded = { jointProjectsExpanded = it },
            formatTime = formatTime,
            openProject = openProject,
            openProjectConversations = openProjectConversations,
            isProjectWorking = ::isProjectWorking,
            showProjectActions = showProjectActions,
            showCreateProjectDialog = showCreateProjectDialog,
            showProjectPlaza = showProjectPlaza,
            dp = dp,
            selectableForeground = selectableForeground
        ).render()
    }

    private fun ensurePlazaBannerProjects() {
        if (plazaBannerLoading || plazaBannerLoaded) return
        plazaBannerLoading = true
        thread(name = "project-plaza-banner") {
            val result = runCatching {
                fetchStoreProjects(
                    http = http,
                    serverUrl = serverUrl(),
                    limit = 18,
                    sort = "members"
                ).filter { it.isPublic }
            }
            activity.runOnUiThread {
                plazaBannerLoading = false
                plazaBannerLoaded = true
                result.onSuccess { projects ->
                    plazaBannerProjects = projects
                        .sortedWith(
                            compareByDescending<StoreProject> { it.memberCount }
                                .thenBy { it.name }
                        )
                    if (binding.projectPage.visibility == View.VISIBLE) renderProjectList()
                }
            }
        }
    }

    fun updatePlazaProjectIcon(projectIds: Set<String>, iconDataUrl: String?) {
        val ids = projectIds.mapNotNull { it.trim().takeIf(String::isNotBlank) }.toSet()
        if (ids.isEmpty() || plazaBannerProjects.isEmpty()) return
        val cleanIcon = iconDataUrl?.trim()?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
        var changed = false
        plazaBannerProjects = plazaBannerProjects.map { project ->
            if (project.id !in ids) return@map project
            changed = true
            project.copy(iconDataUrl = cleanIcon)
        }
        if (changed && binding.projectPage.visibility == View.VISIBLE) {
            renderProjectList()
        }
    }

    fun refreshPlazaBannerProjects() {
        plazaBannerLoaded = false
        ensurePlazaBannerProjects()
    }

    fun isConversationWorking(index: Int): Boolean {
        val conversations = conversations()
        if (index !in conversations.indices || conversations[index].ended) return false
        return isTaskRunning(activeProject().id, conversations[index].id)
    }

    private fun isProjectWorking(project: AppProject): Boolean {
        if (projectHasRunningStatus(project)) return true
        return project.conversations.any { conversation ->
            !conversation.ended && isTaskRunning(project.projectSpaceId(), conversation.id)
        }
    }

    private fun projectHasRunningStatus(project: AppProject): Boolean {
        val stage = project.stage.trim()
        return stage.equals("running", ignoreCase = true) || stage == "运行中"
    }

}
