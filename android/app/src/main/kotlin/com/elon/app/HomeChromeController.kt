package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class HomeChromeController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val actionPopupProvider: () -> PopupWindow?,
    private val dp: (Int) -> Int,
    private val setNavigationBarColor: (Int) -> Unit,
    showCreateProjectDialog: () -> Unit,
    private val showFriendsHome: () -> Unit,
    private val showProjectHome: () -> Unit,
    private val showProfileHome: () -> Unit
) {
    private val projectCreateFab = HomeProjectCreateFabController(
        binding = binding,
        dp = dp,
        showCreateProjectDialog = showCreateProjectDialog
    )

    fun setup() {
        projectCreateFab.setup()
        binding.homeMenuButton.setOnClickListener { showNavigationMenu() }
    }

    fun showHome() {
        setNavigationBarColor(R.color.elon_bg_app)
        binding.pageTabs.visibility = android.view.View.GONE
        binding.projectSpaceAiMenu.visibility = android.view.View.GONE
        binding.homeMenuButton.visibility = android.view.View.VISIBLE
        projectCreateFab.show()
    }

    fun hide() {
        binding.homeMenuButton.visibility = android.view.View.GONE
        projectCreateFab.hide()
    }

    fun clearTranslations() {
        binding.homeProjectCreateMenu.translationX = 0f
    }

    private fun showNavigationMenu() {
        actionPopupProvider()?.dismiss()
        val panel = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(6), 0, dp(6))
            background = GradientDrawable().apply {
                cornerRadius = dp(14).toFloat()
                setColor(Color.parseColor(WECHAT_POPUP_PANEL_COLOR))
            }
            alpha = 0f
            scaleX = 0.98f
            scaleY = 0.98f
        }

        lateinit var popup: PopupWindow
        fun addRow(label: String, iconRes: Int, action: () -> Unit) {
            panel.addView(TextView(activity).apply {
                text = label
                setTextColor(activity.getColor(R.color.elon_text_nav))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(18), 0, dp(16), 0)
                compoundDrawablePadding = dp(12)
                setCompoundDrawablesWithIntrinsicBounds(iconRes, 0, 0, 0)
                compoundDrawableTintList = ColorStateList.valueOf(activity.getColor(R.color.elon_text_nav))
                isClickable = true
                setOnClickListener {
                    popup.dismiss()
                    action()
                }
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(44)))
        }

        addRow("好友", R.drawable.ic_tab_chat_custom, showFriendsHome)
        addRow("项目", R.drawable.ic_tab_project_custom, showProjectHome)
        addRow("我的", R.drawable.ic_tab_profile_custom, showProfileHome)

        popup = PopupWindow(panel, dp(156), ViewGroup.LayoutParams.WRAP_CONTENT, true).apply {
            isOutsideTouchable = true
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAsDropDown(binding.homeMenuButton, dp(8), -dp(2))
        }
        panel.pivotX = 0f
        panel.pivotY = 0f
        panel.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .setDuration(120L)
            .start()
    }

}
