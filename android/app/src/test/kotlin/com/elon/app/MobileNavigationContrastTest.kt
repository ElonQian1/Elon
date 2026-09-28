package com.elon.app

import android.content.res.Configuration
import android.widget.ImageView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.ColorUtils
import com.elon.app.databinding.ActivityMainBinding
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class MobileNavigationContrastTest {
    @Test fun navigationIconsRemainReadableAcrossThemeAndSelectionChanges() {
        for (night in listOf(false, true)) {
            Robolectric.buildActivity(AppCompatActivity::class.java).use { controller ->
                val activity = controller.get()
                activity.setTheme(R.style.Theme_ElonApp)
                @Suppress("DEPRECATION")
                activity.resources.updateConfiguration(Configuration(activity.resources.configuration).apply {
                    uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                        if (night) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
                }, activity.resources.displayMetrics)
                controller.setup()
                val binding = ActivityMainBinding.inflate(activity.layoutInflater)
                val metrics = MainNavigationDesignMetrics(activity, binding) { _, _ -> }
                metrics.apply()
                fun readable(icon: ImageView, background: Int) {
                    val tint = icon.imageTintList
                    assertNotNull("Navigation icon must adapt to the current theme", tint)
                    val color = tint!!.getColorForState(icon.drawableState, tint.defaultColor)
                    assertTrue("Navigation icon contrast must be at least 3:1 (night=$night)",
                        ColorUtils.calculateContrast(color, activity.getColor(background)) >= 3.0)
                }
                for ((tab, icon) in listOf(binding.tabChat to binding.tabChatIcon,
                    binding.tabProject to binding.tabProjectIcon, binding.tabProfile to binding.tabProfileIcon)) {
                    for (selected in listOf(false, true, false)) {
                        metrics.applyBottomTabAssetState(tab, selected)
                        readable(icon, if (selected) R.color.mobile_primary_container else R.color.mobile_surface_container)
                    }
                }
                for (open in listOf(false, true, false)) {
                    binding.bottomMenuIcon.isActivated = open
                    readable(binding.bottomMenuIcon, R.color.mobile_surface_container)
                }
                readable(binding.bottomActionPlusIcon, R.color.mobile_surface_container_high)
            }
        }
    }
}
