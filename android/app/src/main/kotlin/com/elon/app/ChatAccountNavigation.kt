package com.elon.app

import android.content.Context
import android.content.Intent

internal object ChatAccountNavigation {
    fun openPersonal(context: Context) {
        val destination = if (AuthManager.isLoggedIn(context)) {
            PersonalProfileActivity::class.java
        } else {
            LoginActivity::class.java
        }
        context.startActivity(Intent(context, destination))
    }

    fun openChatGpt(context: Context) {
        val destination = if (AuthManager.isLoggedIn(context)) {
            AiProviderAccountsActivity::class.java
        } else {
            LoginActivity::class.java
        }
        context.startActivity(Intent(context, destination))
    }
}
