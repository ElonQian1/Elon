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
) {
    fun onFriendChanged(friend: AppFriend?) {
        val active = friend.isSocialAi()
        binding.topTitleText.apply {
            isClickable = active
            isFocusable = active
            contentDescription = if (active) {
                activity.getString(R.string.social_ai_mode_current_description)
            } else {
                friend?.name.orEmpty()
            }
            setCompoundDrawablesRelativeWithIntrinsicBounds(
                0,
                0,
                if (active) R.drawable.ic_input_chevron_new else 0,
                0,
            )
            compoundDrawablePadding = if (active) dp(6) else 0
            setOnClickListener(if (active) android.view.View.OnClickListener { showSelector() } else null)
        }
    }

    fun openChatGptWeb() {
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

    private fun showSelector() {
        AlertDialog.Builder(activity)
            .setTitle(R.string.social_ai_mode_picker_title)
            .setSingleChoiceItems(
                arrayOf(
                    activity.getString(R.string.social_ai_mode_yilong),
                    activity.getString(R.string.social_ai_mode_chatgpt_web),
                ),
                MODE_YILONG,
            ) { dialog, which ->
                dialog.dismiss()
                if (which == MODE_CHATGPT_WEB) openChatGptWeb()
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    companion object {
        private const val MODE_YILONG = 0
        private const val MODE_CHATGPT_WEB = 1
    }
}
