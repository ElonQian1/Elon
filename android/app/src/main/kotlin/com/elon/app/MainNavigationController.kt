package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
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
    private val conversationHomeTitle: () -> String,
    private val renderProjectList: () -> Unit,
    private val renderProjectSpace: () -> Unit,
    private val refreshServerVersion: () -> Unit,
    private val openConversation: (Int) -> Unit,
    private val showConversationActions: (Int) -> Unit,
    private val showRenameConversationDialog: (Int) -> Unit,
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
    private val suspendSocialChatForProjectReturn: () -> Boolean,
    private val restoreSocialChatForProjectReturn: (Boolean) -> Boolean,
    private val showProjectMembers: () -> Unit,
    private val loadMarketplace: () -> Unit,
    private val onAgentTabSelected: () -> Unit,
    private val handleProjectSpaceInternalBack: () -> Boolean,
    private val openProjectSpacePostComposer: () -> Unit,
    private val showCreateProjectDialog: () -> Unit, private val createConversationAndOpen: () -> Unit, private val projectBrowserDependencies: ProjectBrowserSheetDependencies
) {
    private enum class ChatReturnTarget {
        FRIENDS,
        SOCIAL_CHAT,
        PROJECTS,
        PROJECT_SPACE
    }

    private var pageTransitionRunning = false
    private var chatReturnTarget = ChatReturnTarget.FRIENDS
    private var projectPageReturnTarget = ChatReturnTarget.PROJECTS
    private var nextProjectChatReturnTarget: ChatReturnTarget? = null
    private var projectSpaceTitle = "项目空间"
    private var exitConfirmDialog: AlertDialog? = null
    private val designMetrics = MainNavigationDesignMetrics(activity, binding, ::updateBottomTabVisual)
    private val projectBrowser = ProjectBrowserSheetController(activity, binding, ::dp, projectBrowserDependencies)
    private val bottomNavigation = MainBottomNavigationController(activity, binding, { selectBottomTab(it, false) }, { projectBrowser.close(false); createConversationAndOpen() })
    private val homeChrome = HomeChromeController(
        activity, binding, actionPopupProvider, ::dp, ::setNavigationBarColor, bottomNavigation::setVisible,
        { showConversationHome(animate = false) },
        { showProjectPlaza() }, projectBrowser::toggle
    )

    fun setupNavigation() {
        designMetrics.apply()
        projectBrowser.setup(); homeChrome.setup()
        bottomNavigation.setup()
        binding.projectHomeTopTabWrap.setOnClickListener { showProjectPlaza() }
        binding.projectPlazaTopTabWrap.setOnClickListener { showProjectPlaza() }
        binding.conversationItem.setOnClickListener { openConversation(0) }
        binding.conversationItem.setOnLongClickListener {
            showRenameConversationDialog(0)
            true
        }
        binding.searchButton.setOnClickListener { showFriendLocalSearch() }
        binding.moreButton.setOnClickListener { showChatActionPopup(binding.moreButton) }
        binding.voiceCallButton.setOnClickListener { openSocialAiVoiceCall() }
        binding.projectSpaceAiMenu.setOnClickListener { openProjectSpacePostComposer() }
        binding.backButton.setOnClickListener { navigateBackOneLevel() }
        selectBottomTab(binding.tabChat, animate = false)
    }

    fun captureProjectEntryReturnTarget() {
        val target = currentProjectEntryReturnTarget()
        val resolvedTarget = if (
            target == ChatReturnTarget.FRIENDS &&
            suspendSocialChatForProjectReturn()
        ) {
            ChatReturnTarget.SOCIAL_CHAT
        } else {
            target
        }
        projectPageReturnTarget = resolvedTarget
        nextProjectChatReturnTarget = when (resolvedTarget) {
            ChatReturnTarget.FRIENDS,
            ChatReturnTarget.SOCIAL_CHAT -> resolvedTarget
            ChatReturnTarget.PROJECTS,
            ChatReturnTarget.PROJECT_SPACE -> null
        }
    }

    private fun showMainTabs() {
        binding.scheduleNavigationBarChrome(activity, R.color.elon_bg_app, false)
        binding.projectSpaceAiMenu.visibility = View.GONE
        homeChrome.showMenuOnly()
    }

    private fun hideBottomMenus() {
        projectBrowser.close(false); binding.scheduleNavigationBarChrome(activity, R.color.elon_bg_app, true)
        bottomNavigation.setVisible(false)
        binding.projectSpaceAiMenu.visibility = View.GONE
        homeChrome.hide()
    }

    private fun showProjectTopTabs(plazaSelected: Boolean) {
        designMetrics.setProjectToolbarExpanded(true)
        binding.topTitleText.visibility = View.GONE
        binding.projectTopTabs.visibility = View.VISIBLE
        binding.projectTopTabs.setPadding(0, 0, 0, 0)
        binding.projectTopTabs.gravity = Gravity.CENTER
        binding.projectHomeTopTabWrap.visibility = if (plazaSelected) View.GONE else View.VISIBLE
        binding.projectPlazaTopTabWrap.visibility = View.VISIBLE
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
        if (plazaSelected) binding.projectPlazaTopTab.apply { setTypeface(typeface, android.graphics.Typeface.NORMAL); binding.projectPlazaTabIndicator.visibility = View.GONE }
    }
    private fun hideProjectTopTabs() {
        designMetrics.setProjectToolbarExpanded(false)
        binding.projectTopTabs.visibility = View.GONE
        binding.projectHomeTopTabWrap.visibility = View.GONE
        binding.projectPlazaTopTabWrap.visibility = View.VISIBLE
        setProjectHomeSegmentVisible(false)
        binding.topTitleText.visibility = View.VISIBLE
    }

    private fun setProjectHomeSegmentVisible(visible: Boolean) {
        binding.projectSegmentBar.visibility = if (visible) View.VISIBLE else View.GONE
        if (visible) binding.projectSegmentBar.bringToFront()
    }

    private fun updateProjectTopTabVisual(
        tab: TextView,
        indicator: View,
        selected: Boolean,
        showIndicator: Boolean = selected
    ) {
        tab.setTextColor(activity.getColor(R.color.elon_text_primary))
        tab.setTypeface(tab.typeface, if (selected) Typeface.BOLD else Typeface.BOLD)
        indicator.visibility = if (showIndicator) View.VISIBLE else View.INVISIBLE
    }

    private fun showProjectSpaceBottomMenu() {
        binding.scheduleNavigationBarChrome(activity, R.color.elon_store_detail_bg, false)
        bottomNavigation.setVisible(false)
        binding.projectSpaceAiMenu.visibility = View.GONE
        homeChrome.hide()
    }

    private fun setNavigationBarColor(colorRes: Int) {
        binding.scheduleNavigationBarChrome(activity, colorRes)
    }

    private fun selectBottomTab(tab: TextView, animate: Boolean) {
        if (pageTransitionRunning) return
        projectBrowser.close(false); val outgoing = currentPrimaryPage()
        val incoming = pageForBottomTab(tab) ?: return
        if (!animate ||
            outgoing == null ||
            outgoing === incoming
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
        if (tab == binding.tabProject) {
            loadMarketplace()
            applyMarketplaceChrome()
            return
        }
        binding.toolbar.setBackgroundColor(activity.getColor(R.color.elon_bg_app))
        listOf(binding.tabChat, binding.tabProject, binding.tabProfile).forEach {
            updateBottomTabVisual(it, it == tab)
        }
        binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
        binding.chatPage.visibility = View.GONE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
        binding.marketplacePage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
        binding.agentPage.root.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        if (tab == binding.tabChat) homeChrome.showHome() else if (tab == binding.tabProfile) homeChrome.showMenuOnly() else showMainTabs()
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = if (tab == binding.tabChat || tab == binding.tabProject) View.VISIBLE else View.GONE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, tab)
        }
        binding.topTitleText.setOnLongClickListener(null)
        hideProjectTopTabs()
        binding.topTitleText.text = when (tab) {
            binding.tabProfile -> "个人中心"
            else -> conversationHomeTitle()
        }
        if (tab == binding.tabChat) {
            refreshFriends()
            renderConversationList()
        } else if (tab == binding.tabProfile) {
            refreshServerVersion()
        }
    }

    private fun finishBottomTabSelection(tab: TextView) {
        binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
        binding.chatPage.visibility = View.GONE
        binding.marketplacePage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
        binding.agentPage.root.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        if (tab == binding.tabChat) homeChrome.showHome() else if (tab == binding.tabProfile) homeChrome.showMenuOnly() else showMainTabs()
    }

    private fun pageForBottomTab(tab: TextView): View? {
        return when (tab) {
            binding.tabChat -> binding.conversationPage
            binding.tabProject -> binding.marketplacePage
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
        resetProjectReturnTargets()
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
        homeChrome.showMenuOnly()
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
        if (projectBrowser.handleBack() || exitFriendLocalSearch()) return
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
                ChatReturnTarget.SOCIAL_CHAT -> {
                    onProjectChannelClosed()
                    resetProjectReturnTargets()
                    if (!restoreSocialChatForProjectReturn(true)) {
                        showConversationHome(animate = true)
                    }
                }
                ChatReturnTarget.PROJECT_SPACE -> {
                    onProjectChannelClosed()
                    showProjectSpace(projectSpaceTitle, animate = true)
                }
                ChatReturnTarget.FRIENDS -> showConversationHome(animate = true)
            }
            return
        }
        if (binding.isProjectSpaceSurfaceVisible()) {
            if (handleProjectSpaceInternalBack()) return
            when (projectPageReturnTarget) {
                ChatReturnTarget.FRIENDS -> showConversationHome(animate = true)
                ChatReturnTarget.SOCIAL_CHAT -> {
                    resetProjectReturnTargets()
                    if (!restoreSocialChatForProjectReturn(true)) {
                        showConversationHome(animate = true)
                    }
                }
                ChatReturnTarget.PROJECTS,
                ChatReturnTarget.PROJECT_SPACE -> showProjectHome(animate = true)
            }
            return
        }
        if (binding.marketplacePage.visibility == View.VISIBLE) {
            showExitConfirmation()
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
        resetProjectReturnTargets()
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
                incoming = listOf(binding.conversationPage),
                onEnd = {
                    binding.chatPage.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.marketplacePage.visibility = View.GONE
                    binding.conversationPage.visibility = View.VISIBLE
                    homeChrome.showHome()
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
        val returnTarget = consumeNextProjectChatReturnTarget(ChatReturnTarget.PROJECTS)
        if (returnTarget == ChatReturnTarget.SOCIAL_CHAT) {
            onProjectChannelClosed()
        } else {
            onFriendChatClosed()
        }
        chatReturnTarget = returnTarget
        val shouldAnimateFromProject = animate && binding.projectPage.visibility == View.VISIBLE
        val shouldAnimateFromConversation = animate && binding.conversationPage.visibility == View.VISIBLE
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        applyChatChrome()
        if (shouldAnimateFromConversation) {
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
        } else if (shouldAnimateFromProject) {
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
        val returnTarget = consumeNextProjectChatReturnTarget(projectPersonalChatReturnTarget())
        chatReturnTarget = if (
            returnTarget == ChatReturnTarget.FRIENDS &&
            suspendSocialChatForProjectReturn()
        ) {
            ChatReturnTarget.SOCIAL_CHAT
        } else {
            returnTarget
        }
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

    private fun projectPersonalChatReturnTarget(): ChatReturnTarget {
        if (binding.chatPage.visibility == View.VISIBLE) return chatReturnTarget
        if (binding.isProjectSpaceSurfaceVisible()) {
            return when (projectPageReturnTarget) {
                ChatReturnTarget.FRIENDS,
                ChatReturnTarget.SOCIAL_CHAT -> projectPageReturnTarget
                ChatReturnTarget.PROJECTS,
                ChatReturnTarget.PROJECT_SPACE -> ChatReturnTarget.PROJECT_SPACE
            }
        }
        if (binding.conversationPage.visibility == View.VISIBLE) return ChatReturnTarget.FRIENDS
        return ChatReturnTarget.PROJECTS
    }

    private fun currentProjectEntryReturnTarget(): ChatReturnTarget {
        if (binding.chatPage.visibility == View.VISIBLE) return chatReturnTarget
        if (binding.isProjectSpaceSurfaceVisible()) {
            return projectPageReturnTarget
        }
        if (binding.conversationPage.visibility == View.VISIBLE) return ChatReturnTarget.FRIENDS
        return ChatReturnTarget.PROJECTS
    }

    private fun consumeNextProjectChatReturnTarget(defaultTarget: ChatReturnTarget): ChatReturnTarget {
        val target = nextProjectChatReturnTarget ?: defaultTarget
        nextProjectChatReturnTarget = null
        return target
    }

    private fun resetProjectReturnTargets() {
        projectPageReturnTarget = ChatReturnTarget.PROJECTS
        nextProjectChatReturnTarget = null
    }

    fun showProjectManagement(animate: Boolean = false) {
        if (animate) closeChatSideMenu(false)
        showProjectPlaza()
    }

    fun showProjectSpace(title: String, animate: Boolean = false) {
        clearMessageSelection()
        projectSpaceTitle = title.ifBlank { "项目空间" }
        actionPopupProvider()?.dismiss()
        closeChatSideMenu(false)
        val enteringFromProjectHome = animate &&
            binding.isProjectHomeSurfaceVisible()
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

    fun showProjectHome(animate: Boolean = false) {
        if (animate) closeChatSideMenu(false)
        showProjectPlaza()
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
        binding.restoreChatToolbar(activity.getColor(R.color.elon_bg_app))
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
        binding.moreButton.setImageResource(R.drawable.ic_more_horizontal)
        applyDefaultMoreButtonIconInsets()
        binding.moreButton.setOnClickListener { showChatActionPopup(binding.moreButton) }
        binding.moreButton.contentDescription = "聊天功能"
        binding.stageHintBar.visibility = View.VISIBLE
        renderConversationList()
        binding.topTitleText.text = activeConversationProvider().title
        binding.topTitleText.setOnLongClickListener {
            showConversationActions(activeConversationIndexProvider())
            true
        }
    }

    private fun applyConversationHomeChrome() {
        binding.toolbar.setBackgroundColor(activity.getColor(R.color.elon_bg_app))
        updateBottomTabSelection(binding.tabChat)
        binding.conversationPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        homeChrome.showHome()
        hideProjectTopTabs()
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.VISIBLE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, binding.tabChat)
        }
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = conversationHomeTitle()
        refreshFriends()
    }

    private fun applyFriendChatChrome(title: String) {
        binding.restoreChatToolbar(activity.getColor(R.color.elon_bg_app))
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
        binding.moreButton.setImageResource(R.drawable.ic_more_horizontal)
        applyDefaultMoreButtonIconInsets()
        binding.moreButton.setOnClickListener { showContactChatSettings() }
        binding.moreButton.contentDescription = "聊天设置"
        binding.quickActionStrip.visibility = View.GONE
        binding.stageHintBar.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
    }

    private fun applyProjectHomeChrome() {
        binding.toolbar.setBackgroundColor(activity.getColor(R.color.elon_bg_app))
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.projectPage.visibility = View.VISIBLE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        showMainTabs()
        showProjectTopTabs(plazaSelected = false)
        setProjectHomeSegmentVisible(true)
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
        resetProjectHomeScroll()
    }

    private fun applyMarketplaceChrome() {
        binding.toolbar.setBackgroundColor(activity.getColor(R.color.elon_bg_app))
        updateBottomTabSelection(binding.tabProject)
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.GONE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.marketplacePage.visibility = View.VISIBLE
        binding.agentPage.root.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        homeChrome.showProjectPlazaEntry()
        hideProjectTopTabs()
        setProjectHomeSegmentVisible(false)
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.GONE
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = "项目广场"
    }

    private fun applyProjectSpaceChrome(title: String) {
        binding.toolbar.setBackgroundColor(activity.getColor(R.color.elon_store_detail_bg))
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
        binding.projectMembersButton.visibility = View.GONE
        hideVoiceCallButton()
        binding.moreButton.visibility = View.VISIBLE
        binding.moreButton.setImageResource(R.drawable.ic_project_members_toolbar)
        applyProjectMemberMoreButtonIconInsets()
        binding.moreButton.setOnClickListener { showProjectMembers() }
        binding.moreButton.contentDescription = "项目成员"
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = title
        binding.topTitleText.visibility = View.GONE
        bringProjectSpaceFloatingControlsToFront()
    }

    private fun bringProjectSpaceFloatingControlsToFront() {
        if (binding.projectSpaceAiMenu.visibility == View.VISIBLE) {
            binding.projectSpaceAiMenu.bringToFront()
        }
    }

    private fun applyProjectChannelChrome(title: String) {
        binding.restoreChatToolbar(activity.getColor(R.color.elon_bg_app))
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
        binding.stageHintBar.visibility = View.GONE
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
        val color = tab.context.getColor(R.color.elon_text_nav)
        tab.isSelected = selected
        tab.setTextColor(color)
        tab.textSize = 14f
        tab.compoundDrawableTintList = ColorStateList.valueOf(color)
        designMetrics.applyBottomTabAssetState(tab, selected)
    }

    private fun resetProjectHomeScroll() {
        binding.projectScrollView.post { binding.projectScrollView.scrollTo(0, 0) }
    }

    private fun applyDefaultMoreButtonIconInsets() {
        binding.moreButton.setPadding(dp(8), dp(8), dp(8), dp(8))
    }

    private fun applyProjectMemberMoreButtonIconInsets() {
        binding.moreButton.setPadding(dp(8), dp(11), dp(8), dp(5))
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
        homeChrome.clearTranslations()
    }
}
