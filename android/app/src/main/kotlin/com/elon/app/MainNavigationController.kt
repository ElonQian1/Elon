package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.widget.PopupWindow
import android.widget.TextView
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainNavigationController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val actionPopupProvider: () -> PopupWindow?,
    private val activeConversationProvider: () -> AppConversation,
    private val activeConversationIndexProvider: () -> Int,
    private val compactProjectTitle: () -> String,
    private val renderConversationList: () -> Unit,
    private val renderProjectList: () -> Unit,
    private val renderProjectSpace: () -> Unit,
    private val refreshServerVersion: () -> Unit,
    private val openConversation: (Int) -> Unit,
    private val showConversationActions: (Int) -> Unit,
    private val showHomeActionPopup: (View, TextView) -> Unit,
    private val showChatActionPopup: (View) -> Unit,
    private val showContactChatSettings: () -> Unit,
    private val showAddFriendDialog: () -> Unit,
    private val refreshFriends: () -> Unit,
    private val updateFirstConversationStatus: (String) -> Unit,
    private val collapseInputComposer: (Boolean) -> Unit,
    private val collapseInputComposerForBack: () -> Boolean,
    private val isChatSideMenuOpen: () -> Boolean,
    private val closeChatSideMenu: (Boolean) -> Unit,
    private val isActiveConversationWorking: () -> Boolean,
    private val isMessageSelectionActive: () -> Boolean,
    private val clearMessageSelection: () -> Unit,
    private val setSendEnabled: (Boolean) -> Unit,
    private val maybePrewarmCodexSession: (String) -> Unit,
    private val onFriendChatClosed: () -> Unit,
    private val onProjectChannelClosed: () -> Unit,
    private val showProjectMembers: () -> Unit,
    private val loadMarketplace: () -> Unit,
    private val onAgentTabSelected: () -> Unit
) {
    private enum class ChatReturnTarget {
        FRIENDS,
        PROJECTS,
        PROJECT_SPACE
    }

    private var pageTransitionRunning = false
    private var chatReturnTarget = ChatReturnTarget.FRIENDS
    private var projectSpaceTitle = "项目空间"
    private var exitConfirmDialog: AlertDialog? = null

    fun setupNavigation() {
        val tabs = listOf(binding.tabChat, binding.tabProject, binding.tabProfile)

        fun select(tab: TextView) {
            WechatPageTransition.cancelActive()
            pageTransitionRunning = false
            clearPageTranslations()
            tabs.forEach {
                updateBottomTabVisual(it, it == tab)
            }
            binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.chatPage.visibility = View.GONE
            binding.projectPage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
            binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
            binding.marketplacePage.visibility = View.GONE
            binding.agentPage.root.visibility = View.GONE
            binding.inputLayout.visibility = View.GONE
            binding.pageTabs.visibility = View.VISIBLE
            binding.backButton.visibility = View.GONE
            binding.searchButton.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.addButton.visibility = if (tab == binding.tabChat || tab == binding.tabProject) View.VISIBLE else View.GONE
            binding.projectMembersButton.visibility = View.GONE
            binding.moreButton.visibility = View.GONE
            binding.addButton.setOnClickListener {
                showHomeActionPopup(binding.addButton, tab)
            }
            binding.topTitleText.setOnLongClickListener(null)
            binding.topTitleText.text = when (tab) {
                binding.tabProject -> "项目管理"
                binding.tabProfile -> "我的"
                else -> "好友"
            }
            if (tab != binding.tabChat) {
                renderConversationList()
            }
            if (tab == binding.tabProject) {
                renderProjectList()
            } else if (tab == binding.tabChat) {
                refreshFriends()
                renderConversationList()
            } else if (tab == binding.tabProfile) {
                refreshServerVersion()
            }
        }

        binding.tabChat.setOnClickListener { select(binding.tabChat) }
        binding.tabProject.setOnClickListener { select(binding.tabProject) }
        binding.tabProfile.setOnClickListener { select(binding.tabProfile) }
        binding.conversationItem.setOnClickListener { openConversation(0) }
        binding.conversationItem.setOnLongClickListener {
            showConversationActions(0)
            true
        }
        binding.searchButton.setOnClickListener { showAddFriendDialog() }
        binding.moreButton.setOnClickListener { showChatActionPopup(binding.moreButton) }
        binding.backButton.setOnClickListener { navigateBackOneLevel() }
        select(binding.tabChat)
    }

    fun showProjectPlaza() {
        if (pageTransitionRunning) return
        clearMessageSelection()
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        val shouldAnimate = binding.projectPage.visibility == View.VISIBLE &&
            binding.marketplacePage.visibility != View.VISIBLE
        loadMarketplace()
        applyMarketplaceChrome()
        if (shouldAnimate) {
            pageTransitionRunning = true
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(binding.marketplacePage),
                outgoing = listOf(binding.projectPage),
                onEnd = {
                    binding.conversationPage.visibility = View.GONE
                    binding.chatPage.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.VISIBLE
                    binding.agentPage.root.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.pageTabs.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            clearPageTranslations()
        }
    }

    fun showAgentCenter() {
        if (pageTransitionRunning) return
        clearMessageSelection()
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        updateBottomTabSelection(binding.tabProfile)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.GONE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.agentPage.root.visibility = View.VISIBLE
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = "Agent 自动化"
        onAgentTabSelected()
    }

    fun setupBackHandling() {
        activity.onBackPressedDispatcher.addCallback(activity, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                navigateBackOneLevel()
            }
        })
    }

    fun navigateBackOneLevel() {
        if (pageTransitionRunning) return
        if (isMessageSelectionActive()) {
            clearMessageSelection()
            return
        }
        if (isChatSideMenuOpen()) {
            closeChatSideMenu(true)
            return
        }
        if (binding.chatPage.visibility == View.VISIBLE) {
            if (collapseInputComposerForBack()) return
            collapseInputComposer(false)
            when (chatReturnTarget) {
                ChatReturnTarget.PROJECTS -> showProjectHome(animate = true)
                ChatReturnTarget.PROJECT_SPACE -> {
                    onProjectChannelClosed()
                    showProjectSpace(projectSpaceTitle, animate = true)
                }
                ChatReturnTarget.FRIENDS -> showConversationHome(animate = true)
            }
            return
        }
        if (binding.projectPage.visibility == View.VISIBLE && binding.pageTabs.visibility != View.VISIBLE) {
            showProjectHome(animate = true)
            return
        }
        if (binding.marketplacePage.visibility == View.VISIBLE) {
            showProjectHome(animate = true)
            return
        }
        if (binding.agentPage.root.visibility == View.VISIBLE) {
            binding.tabProfile.performClick()
            return
        }
        showExitConfirmation()
    }

    fun showConversationHome(animate: Boolean = false) {
        clearMessageSelection()
        onFriendChatClosed()
        if (animate && binding.chatPage.visibility == View.VISIBLE) {
            actionPopupProvider()?.dismiss()
            closeChatSideMenu(false)
            renderConversationList()
            applyConversationHomeChrome()
            pageTransitionRunning = true
            WechatPageTransition.exitToRight(
                container = binding.contentContainer,
                outgoing = listOf(binding.chatPage, binding.inputLayout),
                incoming = listOf(binding.conversationPage, binding.pageTabs),
                onEnd = {
                    binding.chatPage.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.conversationPage.visibility = View.VISIBLE
                    binding.pageTabs.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                    renderConversationList()
                }
            )
        } else {
            binding.tabChat.performClick()
        }
    }

    fun showChat(animate: Boolean = false) {
        if (pageTransitionRunning) return
        clearMessageSelection()
        if (binding.chatPage.visibility != View.VISIBLE) {
            chatReturnTarget = ChatReturnTarget.FRIENDS
        }
        val shouldAnimate = animate && binding.conversationPage.visibility == View.VISIBLE
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        applyChatChrome()
        if (shouldAnimate) {
            collapseInputComposer(false)
            pageTransitionRunning = true
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(binding.chatPage, binding.inputLayout),
                outgoing = listOf(binding.conversationPage, binding.pageTabs),
                onEnd = {
                    binding.conversationPage.visibility = View.GONE
                    binding.pageTabs.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.chatPage.visibility = View.VISIBLE
                    binding.inputLayout.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.conversationPage.visibility = View.GONE
            binding.pageTabs.visibility = View.GONE
            binding.projectPage.visibility = View.GONE
            binding.profilePage.visibility = View.GONE
            binding.marketplacePage.visibility = View.GONE
            binding.chatPage.visibility = View.VISIBLE
            binding.inputLayout.visibility = View.VISIBLE
            clearPageTranslations()
        }
        setSendEnabled(!isActiveConversationWorking())
        maybePrewarmCodexSession("show_chat")
    }

    fun showFriendChat(title: String, animate: Boolean = false) {
        if (pageTransitionRunning) return
        clearMessageSelection()
        chatReturnTarget = ChatReturnTarget.FRIENDS
        val shouldAnimate = animate && binding.conversationPage.visibility == View.VISIBLE
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        applyFriendChatChrome(title)
        if (shouldAnimate) {
            collapseInputComposer(false)
            pageTransitionRunning = true
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(binding.chatPage),
                outgoing = listOf(binding.conversationPage),
                incomingFull = listOf(binding.inputLayout),
                outgoingFull = listOf(binding.pageTabs),
                onEnd = {
                    binding.conversationPage.visibility = View.GONE
                    binding.pageTabs.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.chatPage.visibility = View.VISIBLE
                    binding.inputLayout.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.conversationPage.visibility = View.GONE
            binding.pageTabs.visibility = View.GONE
            binding.projectPage.visibility = View.GONE
            binding.profilePage.visibility = View.GONE
            binding.marketplacePage.visibility = View.GONE
            binding.chatPage.visibility = View.VISIBLE
            binding.inputLayout.visibility = View.VISIBLE
            clearPageTranslations()
        }
        setSendEnabled(true)
    }

    fun showProjectChat(animate: Boolean = false) {
        if (pageTransitionRunning) return
        clearMessageSelection()
        onFriendChatClosed()
        chatReturnTarget = ChatReturnTarget.PROJECTS
        val shouldAnimate = animate && binding.projectPage.visibility == View.VISIBLE
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        applyChatChrome()
        if (shouldAnimate) {
            collapseInputComposer(false)
            pageTransitionRunning = true
            WechatPageTransition.enterFromLeft(
                container = binding.contentContainer,
                incoming = listOf(binding.chatPage, binding.inputLayout),
                outgoing = listOf(binding.projectPage, binding.pageTabs),
                onEnd = {
                    binding.conversationPage.visibility = View.GONE
                    binding.pageTabs.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.chatPage.visibility = View.VISIBLE
                    binding.inputLayout.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.conversationPage.visibility = View.GONE
            binding.pageTabs.visibility = View.GONE
            binding.projectPage.visibility = View.GONE
            binding.profilePage.visibility = View.GONE
            binding.marketplacePage.visibility = View.GONE
            binding.chatPage.visibility = View.VISIBLE
            binding.inputLayout.visibility = View.VISIBLE
            clearPageTranslations()
        }
        setSendEnabled(!isActiveConversationWorking())
        maybePrewarmCodexSession("show_project_chat")
    }

    fun showProjectChannelChat(title: String, animate: Boolean = false) {
        if (pageTransitionRunning) return
        clearMessageSelection()
        chatReturnTarget = ChatReturnTarget.PROJECT_SPACE
        val shouldAnimate = animate && binding.projectPage.visibility == View.VISIBLE
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        applyProjectChannelChrome(title)
        if (shouldAnimate) {
            collapseInputComposer(false)
            pageTransitionRunning = true
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(binding.chatPage, binding.inputLayout),
                outgoing = listOf(binding.projectPage),
                incomingFull = emptyList(),
                outgoingFull = listOf(binding.pageTabs),
                onEnd = {
                    binding.projectPage.visibility = View.GONE
                    binding.pageTabs.visibility = View.GONE
                    binding.conversationPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.chatPage.visibility = View.VISIBLE
                    binding.inputLayout.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.conversationPage.visibility = View.GONE
            binding.pageTabs.visibility = View.GONE
            binding.projectPage.visibility = View.GONE
            binding.profilePage.visibility = View.GONE
            binding.marketplacePage.visibility = View.GONE
            binding.chatPage.visibility = View.VISIBLE
            binding.inputLayout.visibility = View.VISIBLE
            clearPageTranslations()
        }
        setSendEnabled(true)
    }

    fun showProjectPersonalChat(title: String, animate: Boolean = false) {
        if (pageTransitionRunning) return
        clearMessageSelection()
        onFriendChatClosed()
        chatReturnTarget = ChatReturnTarget.PROJECT_SPACE
        val shouldAnimate = animate && binding.projectPage.visibility == View.VISIBLE
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        applyChatChrome()
        binding.topTitleText.text = title.ifBlank { activeConversationProvider().title }
        if (shouldAnimate) {
            collapseInputComposer(false)
            pageTransitionRunning = true
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(binding.chatPage, binding.inputLayout),
                outgoing = listOf(binding.projectPage),
                incomingFull = emptyList(),
                outgoingFull = listOf(binding.pageTabs),
                onEnd = {
                    binding.projectPage.visibility = View.GONE
                    binding.pageTabs.visibility = View.GONE
                    binding.conversationPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.chatPage.visibility = View.VISIBLE
                    binding.inputLayout.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.conversationPage.visibility = View.GONE
            binding.pageTabs.visibility = View.GONE
            binding.projectPage.visibility = View.GONE
            binding.profilePage.visibility = View.GONE
            binding.marketplacePage.visibility = View.GONE
            binding.chatPage.visibility = View.VISIBLE
            binding.inputLayout.visibility = View.VISIBLE
            clearPageTranslations()
        }
        setSendEnabled(!isActiveConversationWorking())
        maybePrewarmCodexSession("show_project_personal_chat")
    }

    fun showProjectManagement(animate: Boolean = false) {
        showProjectHome(animate = animate)
    }

    fun showProjectSpace(title: String, animate: Boolean = false) {
        clearMessageSelection()
        projectSpaceTitle = title.ifBlank { "项目空间" }
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        renderProjectSpace()
        applyProjectSpaceChrome(projectSpaceTitle)
        if (animate && binding.chatPage.visibility == View.VISIBLE) {
            pageTransitionRunning = true
            WechatPageTransition.exitToLeft(
                container = binding.contentContainer,
                outgoing = listOf(binding.chatPage, binding.inputLayout),
                incoming = listOf(binding.projectPage),
                incomingFull = emptyList(),
                outgoingFull = listOf(binding.pageTabs),
                onEnd = {
                    binding.chatPage.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.conversationPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.projectPage.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                    renderProjectSpace()
                }
            )
        }
    }

    private fun showProjectHome(animate: Boolean = false) {
        if (animate && binding.marketplacePage.visibility == View.VISIBLE) {
            actionPopupProvider()?.dismiss()
            closeChatSideMenu(false)
            renderProjectList()
            applyProjectHomeChrome()
            pageTransitionRunning = true
            WechatPageTransition.exitToRight(
                container = binding.contentContainer,
                outgoing = listOf(binding.marketplacePage),
                incoming = listOf(binding.projectPage),
                onEnd = {
                    binding.chatPage.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.conversationPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.projectPage.visibility = View.VISIBLE
                    binding.pageTabs.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                    renderProjectList()
                }
            )
        } else if (animate && binding.chatPage.visibility == View.VISIBLE) {
            actionPopupProvider()?.dismiss()
            closeChatSideMenu(false)
            renderProjectList()
            applyProjectHomeChrome()
            pageTransitionRunning = true
            WechatPageTransition.exitToLeft(
                container = binding.contentContainer,
                outgoing = listOf(binding.chatPage, binding.inputLayout),
                incoming = listOf(binding.projectPage, binding.pageTabs),
                onEnd = {
                    binding.chatPage.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.conversationPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.projectPage.visibility = View.VISIBLE
                    binding.pageTabs.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                    renderProjectList()
                }
            )
        } else {
            binding.tabProject.performClick()
        }
    }

    private fun showExitConfirmation() {
        if (exitConfirmDialog?.isShowing == true) return
        exitConfirmDialog = AlertDialog.Builder(activity)
            .setTitle("退出应用")
            .setMessage("确定要退出一龙吗？")
            .setNegativeButton("取消", null)
            .setPositiveButton("退出") { _, _ -> activity.finish() }
            .create()
        exitConfirmDialog?.show()
    }

    private fun applyChatChrome() {
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.VISIBLE
        binding.pageTabs.visibility = View.GONE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.VISIBLE
        binding.moreButton.setOnClickListener { showChatActionPopup(binding.moreButton) }
        binding.moreButton.contentDescription = "聊天功能"
        binding.stageHintText.visibility = View.VISIBLE
        renderConversationList()
        binding.topTitleText.text = activeConversationProvider().title
        binding.topTitleText.setOnLongClickListener {
            showConversationActions(activeConversationIndexProvider())
            true
        }
    }

    private fun applyConversationHomeChrome() {
        updateBottomTabSelection(binding.tabChat)
        binding.conversationPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.VISIBLE
        binding.addButton.visibility = View.VISIBLE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.GONE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, binding.tabChat)
        }
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = "好友"
        refreshFriends()
    }

    private fun applyFriendChatChrome(title: String) {
        updateBottomTabSelection(binding.tabChat)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.inputLayout.visibility = View.VISIBLE
        binding.pageTabs.visibility = View.GONE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.VISIBLE
        binding.moreButton.setOnClickListener { showContactChatSettings() }
        binding.moreButton.contentDescription = "聊天设置"
        binding.quickActionStrip.visibility = View.GONE
        binding.stageHintText.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
    }

    private fun applyProjectHomeChrome() {
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.projectPage.visibility = View.VISIBLE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.VISIBLE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.GONE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, binding.tabProject)
        }
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = "项目管理"
        renderProjectList()
    }

    private fun applyMarketplaceChrome() {
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.GONE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.VISIBLE
        binding.agentPage.root.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = "项目广场"
    }

    private fun applyProjectSpaceChrome(title: String) {
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.GONE
        binding.projectPage.visibility = View.VISIBLE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.GONE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.VISIBLE
        binding.projectMembersButton.setOnClickListener { showProjectMembers() }
        binding.moreButton.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
    }

    private fun applyProjectChannelChrome(title: String) {
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.VISIBLE
        binding.pageTabs.visibility = View.GONE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        binding.moreButton.visibility = View.GONE
        binding.quickActionStrip.visibility = View.GONE
        binding.stageHintText.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
    }

    private fun updateBottomTabSelection(selectedTab: TextView) {
        listOf(binding.tabChat, binding.tabProject, binding.tabProfile).forEach { tab ->
            updateBottomTabVisual(tab, tab == selectedTab)
        }
    }

    private fun updateBottomTabVisual(tab: TextView, selected: Boolean) {
        val color = Color.parseColor(if (selected) "#E1E1E1" else "#A8A8A8")
        tab.isSelected = selected
        tab.setTextColor(color)
        tab.textSize = 12f
        tab.compoundDrawableTintList = ColorStateList.valueOf(color)
    }

    /** 更新"好友"tab 未读消息角标。count=0 时隐藏角标。 */
    fun updateChatTabBadge(count: Int) {
        val badge = binding.tabChatBadge
        if (count <= 0) {
            badge.visibility = View.GONE
            return
        }
        badge.text = if (count > 99) "99+" else count.toString()
        badge.textSize = if (count > 99) 7f else 9f
        if (badge.background == null || badge.background !is GradientDrawable) {
            badge.background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor("#E53935"))
            }
        }
        badge.visibility = View.VISIBLE
    }

    private fun clearPageTranslations() {
        binding.conversationPage.translationX = 0f
        binding.chatPage.translationX = 0f
        binding.projectPage.translationX = 0f
        binding.profilePage.translationX = 0f
        binding.marketplacePage.translationX = 0f
        binding.agentPage.root.translationX = 0f
        binding.inputLayout.translationX = 0f
        binding.pageTabs.translationX = 0f
    }
}
