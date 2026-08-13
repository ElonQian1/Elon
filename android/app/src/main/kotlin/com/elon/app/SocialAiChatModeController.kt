package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebTestActivity
import com.elon.app.databinding.ActivityMainBinding

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
    private var activeFriendIsSocialAi = false
    private var interactionMode = modeStore.interactionMode()
    private var providerId = modeStore.providerId()

    fun onFriendChanged(friend: AppFriend?) {
        activeFriendIsSocialAi = friend.isSocialAi()
        if (!activeFriendIsSocialAi) {
            deactivateChatProvider()
            configureTitle(friend?.name.orEmpty(), false)
            return
        }
        configureTitle(activeTitle(), true)
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
        activity.startActivity(ChatGptWebTestActivity.createProductIntent(activity))
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
        if (!provider.available) return false
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
                if (provider.available) {
                    activateChatProvider(provider)
                } else {
                    interactionMode = SocialAiInteractionMode.WORK
                    persist()
                    activateWorkMode()
                }
            }
        }
        configureTitle(activeTitle(), true)
    }

    private fun showSelector() {
        val providers = WebChatProviderRegistry.available()
        val workIndex = providers.size
        val checked = if (interactionMode == SocialAiInteractionMode.WORK) {
            workIndex
        } else {
            providers.indexOfFirst { it.id == providerId }.coerceAtLeast(0)
        }
        val labels = buildList {
            providers.forEach { provider ->
                add(activity.getString(R.string.social_ai_mode_chat_provider, provider.displayName))
            }
            add(activity.getString(R.string.social_ai_mode_work))
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle(R.string.social_ai_mode_picker_title)
            .setSingleChoiceItems(labels.toTypedArray(), checked) { selector, which ->
                selector.dismiss()
                if (which == workIndex) {
                    selectInteractionMode(SocialAiInteractionMode.WORK)
                } else {
                    providers.getOrNull(which)?.let { selectChatProvider(it.id) }
                }
            }
            .setNegativeButton(android.R.string.cancel, null)
            .apply {
                if (interactionMode == SocialAiInteractionMode.CHAT) {
                    setNeutralButton(R.string.web_chat_open_official, null)
                }
            }
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL)?.setOnClickListener { openOfficialFallback() }
        }
        dialog.show()
    }

    private fun activeTitle(): String = when (interactionMode) {
        SocialAiInteractionMode.WORK -> activity.getString(R.string.social_ai_mode_work_title)
        SocialAiInteractionMode.CHAT -> WebChatProviderRegistry.get(providerId).displayName
    }

    private fun configureTitle(title: String, selectable: Boolean) {
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
            setCompoundDrawablesRelativeWithIntrinsicBounds(
                0,
                0,
                if (selectable) R.drawable.ic_input_chevron_new else 0,
                0,
            )
            compoundDrawablePadding = if (selectable) dp(6) else 0
            setOnClickListener(if (selectable) android.view.View.OnClickListener { showSelector() } else null)
        }
    }

    private fun persist() = modeStore.save(interactionMode, providerId)

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()
}
