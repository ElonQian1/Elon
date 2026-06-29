package com.elon.app

import android.content.Intent
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainQuickCommandActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val createConversationAndOpen: () -> Unit,
    private val showChat: () -> Unit,
    private val sendMessage: () -> Unit,
    private val enablePlanModeWithStarterPrompt: () -> Unit
) {
    fun fillPlanPrompt() {
        showChat()
        enablePlanModeWithStarterPrompt()
    }

    fun sendQuickCommand(text: String) {
        if (activeConversation().ended) {
            createConversationAndOpen()
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
