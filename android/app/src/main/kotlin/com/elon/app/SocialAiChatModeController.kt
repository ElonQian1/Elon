package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebTestActivity
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.googleweb.GoogleWebOfficialActivity

internal class SocialAiChatModeController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val findSocialAiFriend: () -> AppFriend?,
    private val closeGroupChat: () -> Unit,
    private val closeProjectChat: () -> Unit,
    private val openFriend: (AppFriend) -> Unit,
    private val onFriendOpened: () -> Unit,
    private val activateWorkMode: () -> Unit,
    private val activateChatProvider: (WebChatProviderIdentity) -> Unit,
    private val deactivateChatProvider: () -> Unit,
) {
    private val modeStore = SocialAiModeStore(activity)
    private val modeControl = SocialAiModeSegmentedControl(
        activity = activity,
        host = binding.socialAiModeControlHost,
        onSelected = ::selectInteractionMode,
    )
    private var activeFriendIsSocialAi = false
    private var interactionMode = modeStore.interactionMode()
    private var providerId = modeStore.providerId()

    fun onFriendChanged(friend: AppFriend?) {
        activeFriendIsSocialAi = friend.isSocialAi()
        if (!activeFriendIsSocialAi) {
            deactivateChatProvider()
            modeControl.hide()
            configureTitle(friend?.name.orEmpty(), false)
            return
        }
        configureSocialAiToolbar()
        binding.root.post {
            if (activeFriendIsSocialAi) applyCurrentMode()
        }
    }

    fun openChatGptWeb() {
        providerId = WebChatProviderId.CHATGPT_WEB
        interactionMode = SocialAiInteractionMode.CHAT
        persist()
        if (activeFriendIsSocialAi) {
            applyCurrentMode()
        } else {
            openSocialAiChat()
        }
    }

    fun openOfficialFallback() {
        val intent = when (providerId) {
            WebChatProviderId.CHATGPT_WEB -> ChatGptWebTestActivity.createProductIntent(activity)
            WebChatProviderId.GOOGLE_WEB -> GoogleWebOfficialActivity.createIntent(activity)
        }
        activity.startActivity(intent)
    }

    fun openSocialAiChat(): Boolean {
        val friend = findSocialAiFriend() ?: return false
        closeGroupChat()
        closeProjectChat()
        openFriend(friend)
        onFriendOpened()
        return true
    }

    fun selectInteractionMode(mode: SocialAiInteractionMode): Boolean {
        interactionMode = mode
        persist()
        if (activeFriendIsSocialAi) applyCurrentMode()
        return true
    }

    fun selectChatProvider(id: WebChatProviderId): Boolean {
        val provider = WebChatProviderRegistry.get(id)
        if (!provider.selectable) return false
        providerId = id
        interactionMode = SocialAiInteractionMode.CHAT
        persist()
        if (activeFriendIsSocialAi) applyCurrentMode()
        return true
    }

    fun interactionMode(): SocialAiInteractionMode = interactionMode

    fun providerId(): WebChatProviderId = providerId

    fun isChatModeActive(): Boolean =
        activeFriendIsSocialAi && interactionMode == SocialAiInteractionMode.CHAT

    private fun applyCurrentMode() {
        when (interactionMode) {
            SocialAiInteractionMode.WORK -> activateWorkMode()
            SocialAiInteractionMode.CHAT -> {
                val provider = WebChatProviderRegistry.get(providerId)
                if (provider.selectable) {
                    activateChatProvider(provider)
                } else {
                    interactionMode = SocialAiInteractionMode.WORK
                    persist()
                    activateWorkMode()
                }
            }
        }
        configureSocialAiToolbar()
    }

    private fun configureSocialAiToolbar() {
        binding.topTitleText.visibility = android.view.View.GONE
        modeControl.show(interactionMode)
    }

    private fun configureTitle(title: String, selectable: Boolean) {
        binding.topTitleText.visibility = android.view.View.VISIBLE
        binding.topTitleText.apply {
            text = title
            isClickable = selectable
            isFocusable = selectable
            contentDescription = if (selectable) {
                activity.getString(
                    R.string.social_ai_mode_current_description,
                    if (interactionMode == SocialAiInteractionMode.WORK) {
                        activity.getString(R.string.social_ai_mode_work_short)
                    } else {
                        activity.getString(R.string.social_ai_mode_chat_short)
                    },
                    title,
                )
            } else {
                title
            }
            setCompoundDrawablesRelativeWithIntrinsicBounds(0, 0, 0, 0)
            compoundDrawablePadding = 0
            setOnClickListener(null)
        }
    }

    private fun persist() = modeStore.save(interactionMode, providerId)

}
