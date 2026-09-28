package com.elon.app

import android.app.Activity
import android.content.Context
import android.content.res.Configuration

/** Appearance support for existing platform Activities, without changing their lifecycle framework. */
abstract class MobileActivity : Activity() {
    override fun attachBaseContext(newBase: Context) {
        val config = Configuration(newBase.resources.configuration)
        config.uiMode = (config.uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or MobileThemePreference.night(newBase)
        super.attachBaseContext(newBase.createConfigurationContext(config))
    }

    override fun onResume() {
        super.onResume()
        if ((resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) != MobileThemePreference.night(this)) recreate()
    }
}
