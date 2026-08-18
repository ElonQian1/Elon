package com.elon.app.chatgptweb

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import androidx.appcompat.widget.AppCompatImageButton
import com.elon.app.R
import com.elon.app.databinding.ActivityMainBinding

internal class ChatGptWebSkinPresentationController(
    private val binding: ActivityMainBinding,
    private val session: ChatGptBackgroundSession,
) {
    private var active = false
    private var chatListVisibility = View.VISIBLE
    private var inputVisibility = View.VISIBLE
    private val exitButton by lazy(LazyThreadSafetyMode.NONE) { createExitButton() }

    fun enter(): Boolean {
        if (active) return true
        if (!session.selectPresentationMode(ChatGptWebPresentationMode.SKIN)) return false
        chatListVisibility = binding.chatList.visibility
        inputVisibility = binding.inputLayout.visibility
        binding.chatList.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        attachExitButton()
        exitButton.visibility = View.VISIBLE
        active = true
        return true
    }

    fun exit(): Boolean {
        session.selectPresentationMode(ChatGptWebPresentationMode.NATIVE)
        if (!active) return true
        exitButton.visibility = View.GONE
        binding.chatList.visibility = chatListVisibility
        binding.inputLayout.visibility = inputVisibility
        active = false
        return true
    }

    fun isActive(): Boolean = active

    fun destroy() {
        exit()
        (exitButton.parent as? FrameLayout)?.removeView(exitButton)
    }

    private fun attachExitButton() {
        if (exitButton.parent === binding.chatListFrame) return
        (exitButton.parent as? FrameLayout)?.removeView(exitButton)
        binding.chatListFrame.addView(
            exitButton,
            FrameLayout.LayoutParams(dp(48), dp(48), Gravity.TOP or Gravity.END).apply {
                topMargin = dp(12)
                marginEnd = dp(12)
            },
        )
    }

    private fun createExitButton() = AppCompatImageButton(binding.root.context).apply {
        setImageResource(R.drawable.ic_popup_chat)
        imageTintList = ColorStateList.valueOf(Color.WHITE)
        scaleType = ImageView.ScaleType.CENTER_INSIDE
        setPadding(dp(12), dp(12), dp(12), dp(12))
        background = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(Color.parseColor("#D91B1C20"))
            setStroke(dp(1), Color.parseColor("#4DFFFFFF"))
        }
        elevation = dp(8).toFloat()
        contentDescription = "web-chat-skin-exit:chatgpt"
        visibility = View.GONE
        setOnClickListener { exit() }
    }

    private fun dp(value: Int): Int =
        (value * binding.root.resources.displayMetrics.density).toInt()
}
