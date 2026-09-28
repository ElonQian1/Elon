package com.elon.app

import android.content.Context
import android.content.res.ColorStateList
import android.view.View
import android.widget.TextView

/** Login/register presentation only; authentication and form data stay with the Activity. */
internal fun applyLoginMode(root: View, registration: Boolean) {
    val login = root.findViewById<TextView>(R.id.loginTabLogin)
    val register = root.findViewById<TextView>(R.id.loginTabRegister)
    val colors = MobileColors(login.context)
    listOf(login to !registration, register to registration).forEach { (tab, selected) ->
        tab.isSelected = selected
        tab.backgroundTintList = ColorStateList.valueOf(if (selected) colors.primaryContainer else colors.container)
        tab.setTextColor(if (selected) colors.onPrimaryContainer else colors.muted)
    }
    root.findViewById<View>(R.id.loginNicknameRow).visibility = if (registration) View.VISIBLE else View.GONE
    root.findViewById<TextView>(R.id.loginSubmitButton).text = if (registration) "注册并登录" else "登录"
    root.findViewById<View>(R.id.loginGoogleButton).visibility = if (registration) View.GONE else View.VISIBLE
    root.findViewById<View>(R.id.loginRecoveryButton).visibility = if (registration) View.GONE else View.VISIBLE
    root.findViewById<View>(R.id.loginErrorText).visibility = View.GONE
}

internal fun mobileAuthDp(context: Context, size: Int) = (size * context.resources.displayMetrics.density + .5f).toInt()
