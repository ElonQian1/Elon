package com.elon.app

import android.content.Intent
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainQuickCommandActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val showCreateConversationDialog: () -> Unit,
    private val showChat: () -> Unit,
    private val sendMessage: () -> Unit
) {
    fun fillPlanPrompt() {
        replaceInput("我想开发一个 App，请先帮我拆解功能、页面和开发计划：")
    }

    fun sendQuickCommand(text: String) {
        if (activeConversation().ended) {
            showCreateConversationDialog()
            return
        }
        showChat()
        replaceInput(text)
        sendMessage()
    }

    fun openSettings() {
        activity.startActivity(Intent(activity, SettingsActivity::class.java))
    }

    private fun replaceInput(text: String) {
        binding.inputEdit.setText(text)
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
    }
}
