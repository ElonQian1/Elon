package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.widget.FrameLayout
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
    private val isDirectSocialAiChatActive: () -> Boolean,
    private val openSocialAiVoiceCall: () -> Unit,
    private val showFriendLocalSearch: () -> Unit,
    private val exitFriendLocalSearch: () -> Boolean,
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
    private val onAgentTabSelected: () -> Unit,
    private val openProjectSpaceAiConversation: () -> Unit
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
        binding.tabChat.setOnClickListener { selectBottomTab(binding.tabChat, animate = false) }
        binding.tabProject.setOnClickListener { selectBottomTab(binding.tabProject, animate = false) }
        binding.tabProfile.setOnClickListener { selectBottomTab(binding.tabProfile, animate = false) }
        binding.projectHomeTopTabWrap.setOnClickListener { showProjectHome(animate = false) }
        binding.projectPlazaTopTabWrap.setOnClickListener { showProjectPlaza() }
        binding.conversationItem.setOnClickListener { openConversation(0) }
        binding.conversationItem.setOnLongClickListener {
            showConversationActions(0)
            true
        }
        binding.searchButton.setOnClickListener { showFriendLocalSearch() }
        binding.moreButton.setOnClickListener { showChatActionPopup(binding.moreButton) }
        binding.voiceCallButton.setOnClickListener { openSocialAiVoiceCall() }
        binding.projectSpaceAiMenu.setOnClickListener { openProjectSpaceAiConversation() }
        binding.backButton.setOnClickListener { navigateBackOneLevel() }
        selectBottomTab(binding.tabChat, animate = false)
    }

    private fun showMainTabs() {
        setNavigationBarColor(R.color.elon_nav_bg)
        binding.pageTabs.visibility = View.VISIBLE
        binding.projectSpaceAiMenu.visibility = View.GONE
        binding.projectSpaceFeedActionsOverlay.visibility = View.GONE
    }

    private fun hideBottomMenus() {
        setNavigationBarColor(R.color.elon_nav_bg)
        binding.pageTabs.visibility = View.GONE
        binding.projectSpaceAiMenu.visibility = View.GONE
        binding.projectSpaceFeedActionsOverlay.visibility = View.GONE
    }

    private fun showProjectTopTabs(plazaSelected: Boolean) {
        binding.topTitleText.visibility = View.GONE
        binding.projectTopTabs.visibility = View.VISIBLE
        updateProjectTopTabVisual(
            tab = binding.projectHomeTopTab,
            indicator = binding.projectHomeTabIndicator,
            selected = !plazaSelected
        )
        updateProjectTopTabVisual(
            tab = binding.projectPlazaTopTab,
            indicator = binding.projectPlazaTabIndicator,
            selected = plazaSelected
        )
    }

    private fun hideProjectTopTabs() {
        binding.projectTopTabs.visibility = View.GONE
        binding.topTitleText.visibility = View.VISIBLE
    }

    private fun updateProjectTopTabVisual(tab: TextView, indicator: View, selected: Boolean) {
        tab.setTextColor(activity.getColor(R.color.elon_text_primary))
        tab.setTypeface(tab.typeface, if (selected) Typeface.BOLD else Typeface.BOLD)
        indicator.visibility = if (selected) View.VISIBLE else View.INVISIBLE
    }

    private fun showProjectSpaceBottomMenu() {
        setNavigationBarColor(R.color.elon_bg_app)
        binding.pageTabs.visibility = View.GONE
        binding.projectSpaceAiMenu.visibility = View.VISIBLE
        binding.projectSpaceAiMenu.bringToFront()
    }

    private fun setNavigationBarColor(colorRes: Int) {
        activity.window.navigationBarColor = activity.getColor(colorRes)
    }

    private fun selectBottomTab(tab: TextView, animate: Boolean) {
        if (pageTransitionRunning) return
        val outgoing = currentPrimaryPage()
        val incoming = pageForBottomTab(tab) ?: return
        if (!animate ||
            outgoing == null ||
            outgoing === incoming ||
            binding.pageTabs.visibility != View.VISIBLE
        ) {
            WechatPageTransition.cancelActive()
            pageTransitionRunning = false
            clearPageTranslations()
            applyBottomTabChrome(tab)
            return
        }

        val outgoingPage = outgoing
        val enterFromRight = bottomTabIndex(tab) > pageTabIndex(outgoingPage)
        applyBottomTabChrome(tab)
        outgoingPage.visibility = View.VISIBLE
        incoming.visibility = View.VISIBLE
        pageTransitionRunning = true
        val onEnd = {
            finishBottomTabSelection(tab)
            clearPageTranslations()
            pageTransitionRunning = false
        }
        if (enterFromRight) {
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(incoming),
                outgoing = listOf(outgoingPage),
                onEnd = onEnd
            )
        } else {
            WechatPageTransition.enterFromLeft(
                container = binding.contentContainer,
                incoming = listOf(incoming),
                outgoing = listOf(outgoingPage),
                onEnd = onEnd
            )
        }
    }

    private fun applyBottomTabChrome(tab: TextView) {
        listOf(binding.tabChat, binding.tabProject, binding.tabProfile).forEach {
            updateBottomTabVisual(it, it == tab)
        }
        binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
        binding.chatPage.visibility = View.GONE
        binding.projectPage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
        binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.agentPage.root.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        showMainTabs()
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
        binding.addButton.visibility = if (tab == binding.tabChat || tab == binding.tabProject) View.VISIBLE else View.GONE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, tab)
        }
        binding.topTitleText.setOnLongClickListener(null)
        if (tab == binding.tabProject) {
            showProjectTopTabs(plazaSelected = false)
        } else {
            hideProjectTopTabs()
        }
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

    private fun finishBottomTabSelection(tab: TextView) {
        binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
        binding.projectPage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
        binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
        binding.chatPage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.agentPage.root.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        showMainTabs()
    }

    private fun pageForBottomTab(tab: TextView): View? {
        return when (tab) {
            binding.tabChat -> binding.conversationPage
            binding.tabProject -> binding.projectPage
            binding.tabProfile -> binding.profilePage
            else -> null
        }
    }

    private fun currentPrimaryPage(): View? {
        return when {
            binding.marketplacePage.visibility == View.VISIBLE -> binding.marketplacePage
            binding.agentPage.root.visibility == View.VISIBLE -> binding.agentPage.root
            binding.conversationPage.visibility == View.VISIBLE -> binding.conversationPage
            binding.projectPage.visibility == View.VISIBLE -> binding.projectPage
            binding.profilePage.visibility == View.VISIBLE -> binding.profilePage
            else -> null
        }
    }

    private fun bottomTabIndex(tab: TextView): Int {
        return when (tab) {
            binding.tabChat -> 0
            binding.tabProject -> 1
            binding.tabProfile -> 2
            else -> 0
        }
    }

    private fun pageTabIndex(page: View): Int {
        return when (page) {
            binding.conversationPage -> 0
            binding.projectPage, binding.marketplacePage -> 1
            binding.profilePage, binding.agentPage.root -> 2
            else -> 0
        }
    }

    fun showProjectPlaza() {
        if (pageTransitionRunning) return
        clearMessageSelection()
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        loadMarketplace()
        applyMarketplaceChrome()
        clearPageTranslations()
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
        showMainTabs()
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = "Agent 自动化"
        hideProjectTopTabs()
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
        if (exitFriendLocalSearch()) return
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
                    showMainTabs()
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
                    hideBottomMenus()
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
            hideBottomMenus()
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
                    hideBottomMenus()
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
            hideBottomMenus()
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
                    hideBottomMenus()
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
            hideBottomMenus()
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
                    hideBottomMenus()
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
            hideBottomMenus()
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
                    hideBottomMenus()
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
            hideBottomMenus()
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
        val enteringFromProjectHome = animate &&
            binding.projectPage.visibility == View.VISIBLE &&
            binding.pageTabs.visibility == View.VISIBLE
        if (enteringFromProjectHome) {
            pageTransitionRunning = true
            WechatPageTransition.replaceContentFromRight(
                container = binding.contentContainer,
                page = binding.projectPage,
                updateContent = {
                    renderProjectSpace()
                    applyProjectSpaceChrome(projectSpaceTitle)
                },
                onEnd = {
                    clearPageTranslations()
                    bringProjectSpaceFloatingControlsToFront()
                    pageTransitionRunning = false
                }
            )
            return
        }
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
                    bringProjectSpaceFloatingControlsToFront()
                }
            )
        }
    }

    private fun showProjectHome(animate: Boolean = false) {
        if (animate && binding.projectPage.visibility == View.VISIBLE && binding.pageTabs.visibility != View.VISIBLE) {
            actionPopupProvider()?.dismiss()
            closeChatSideMenu(false)
            pageTransitionRunning = true
            WechatPageTransition.replaceContentToRight(
                container = binding.contentContainer,
                page = binding.projectPage,
                updateContent = {
                    renderProjectList()
                    applyProjectHomeChrome()
                },
                onEnd = {
                    clearPageTranslations()
                    pageTransitionRunning = false
                    renderProjectList()
                }
            )
        } else if (animate && binding.marketplacePage.visibility == View.VISIBLE) {
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
                    showMainTabs()
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
                    showMainTabs()
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
        hideBottomMenus()
        hideProjectTopTabs()
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
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
        showMainTabs()
        hideProjectTopTabs()
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.VISIBLE
        binding.addButton.visibility = View.VISIBLE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
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
        hideBottomMenus()
        hideProjectTopTabs()
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        updateFriendVoiceCallButton()
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
        showMainTabs()
        showProjectTopTabs(plazaSelected = false)
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.VISIBLE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, binding.tabProject)
        }
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
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
        showMainTabs()
        showProjectTopTabs(plazaSelected = true)
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.VISIBLE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
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
        showProjectSpaceBottomMenu()
        hideProjectTopTabs()
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.VISIBLE
        binding.projectMembersButton.setOnClickListener { showProjectMembers() }
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
        bringProjectSpaceFloatingControlsToFront()
    }

    private fun bringProjectSpaceFloatingControlsToFront() {
        if (binding.projectSpaceFeedActionsOverlay.visibility == View.VISIBLE) {
            binding.projectSpaceFeedActionsOverlay.bringToFront()
        }
        if (binding.projectSpaceAiMenu.visibility == View.VISIBLE) {
            binding.projectSpaceAiMenu.bringToFront()
        }
    }

    private fun applyProjectChannelChrome(title: String) {
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.VISIBLE
        hideBottomMenus()
        hideProjectTopTabs()
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.quickActionStrip.visibility = View.GONE
        binding.stageHintText.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
    }

    private fun hideVoiceCallButton() {
        binding.voiceCallButton.visibility = View.GONE
    }

    private fun updateFriendVoiceCallButton() {
        binding.voiceCallButton.visibility = if (isDirectSocialAiChatActive()) {
            View.VISIBLE
        } else {
            View.GONE
        }
    }

    private fun updateBottomTabSelection(selectedTab: TextView) {
        listOf(binding.tabChat, binding.tabProject, binding.tabProfile).forEach { tab ->
            updateBottomTabVisual(tab, tab == selectedTab)
        }
    }

    private fun updateBottomTabVisual(tab: TextView, selected: Boolean) {
        val color = tab.context.getColor(
            if (selected) R.color.elon_text_primary else R.color.elon_text_secondary
        )
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
        val badgeText = if (count > 99) "99+" else count.toString()
        val height = dp(22)
        val width = when {
            badgeText.length >= 3 -> dp(34)
            badgeText.length == 2 -> dp(28)
            else -> height
        }
        badge.text = badgeText
        badge.textSize = 12f
        badge.layoutParams = (badge.layoutParams as FrameLayout.LayoutParams).apply {
            this.width = width
            this.height = height
        }
        badge.background = GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = height / 2f
            setColor(Color.parseColor("#F04B4F"))
        }
        badge.visibility = View.VISIBLE
    }

    private fun dp(value: Int): Int {
        return (value * activity.resources.displayMetrics.density + 0.5f).toInt()
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
        binding.projectSpaceAiMenu.translationX = 0f
        binding.projectSpaceFeedActionsOverlay.translationX = 0f
    }
}
