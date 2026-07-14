package com.elon.app

import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import kotlin.math.roundToInt

internal class MainNavigationDesignMetrics(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val updateBottomTabVisual: (TextView, Boolean) -> Unit
) {
    fun apply() {
        setProjectToolbarExpanded(false)
        val topControlSize = designPx(PROJECT_ADD_BUTTON_SIZE_PX)
        fun alignFrameTopControl(view: View, resizeWidth: Boolean) {
            (view.layoutParams as? FrameLayout.LayoutParams)?.let {
                if (resizeWidth) it.width = topControlSize
                it.height = topControlSize
                view.layoutParams = it
            }
        }
        listOf(
            binding.backButton,
            binding.homeMenuButton,
            binding.searchButton,
            binding.addButton,
            binding.projectMembersButton,
            binding.voiceCallButton,
            binding.moreButton
        ).forEach { alignFrameTopControl(it, resizeWidth = true) }
        alignFrameTopControl(binding.topTitleText, resizeWidth = false)
        alignFrameTopControl(binding.projectTopTabs, resizeWidth = false)
        binding.projectTopTabs.setPadding(
            designPx(PROJECT_TOP_PADDING_START_PX),
            0,
            designPx(PROJECT_TOP_PADDING_END_PX),
            0
        )
        (binding.projectHomeTopTabWrap.layoutParams as? LinearLayout.LayoutParams)?.let {
            it.marginEnd = designPx(PROJECT_TOP_TAB_GAP_PX)
            binding.projectHomeTopTabWrap.layoutParams = it
        }
        listOf(binding.projectHomeTopTab, binding.projectPlazaTopTab).forEach {
            it.setTextSize(TypedValue.COMPLEX_UNIT_SP, PROJECT_TOP_TAB_TEXT_SP)
        }
        listOf(binding.projectHomeTabIndicator, binding.projectPlazaTabIndicator).forEach {
            val params = it.layoutParams as? FrameLayout.LayoutParams ?: FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            )
            it.layoutParams = params.apply {
                width = designPx(PROJECT_TOP_INDICATOR_WIDTH_PX)
                height = designPx(PROJECT_TOP_INDICATOR_HEIGHT_PX)
                gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
                bottomMargin = designPx(PROJECT_TOP_INDICATOR_BOTTOM_PX)
            }
        }
        binding.addButton.setPadding(
            designPx(PROJECT_ADD_BUTTON_PADDING_PX),
            designPx(PROJECT_ADD_BUTTON_PADDING_PX),
            designPx(PROJECT_ADD_BUTTON_PADDING_PX),
            designPx(PROJECT_ADD_BUTTON_PADDING_PX)
        )
        listOf(binding.tabChat, binding.tabProject, binding.tabProfile).forEach {
            updateBottomTabVisual(it, it.isSelected)
        }
    }

    fun setProjectToolbarExpanded(expanded: Boolean) {
        binding.toolbar.layoutParams = binding.toolbar.layoutParams.apply {
            height = designPx(if (expanded) PROJECT_TOP_TOOLBAR_HEIGHT_PX else PROJECT_TOOLBAR_HEIGHT_PX)
        }
    }

    fun applyBottomTabAssetState(tab: TextView, selected: Boolean) {
        val (selection, icon) = when (tab) {
            binding.tabChat -> binding.tabChatSelection to binding.tabChatIcon
            binding.tabProject -> binding.tabProjectSelection to binding.tabProjectIcon
            binding.tabProfile -> binding.tabProfileSelection to binding.tabProfileIcon
            else -> return
        }
        selection.isSelected = selected
        icon.isSelected = selected
    }

    private fun designPx(value: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: DESIGN_WIDTH_PX
        return (value * (width / DESIGN_WIDTH_PX.toFloat())).roundToInt()
    }

    private companion object {
        const val DESIGN_WIDTH_PX = 1272
        const val PROJECT_TOOLBAR_HEIGHT_PX = 176
        const val PROJECT_TOP_TOOLBAR_HEIGHT_PX = PROJECT_TOOLBAR_HEIGHT_PX
        const val PROJECT_TOP_PADDING_START_PX = 78
        const val PROJECT_TOP_PADDING_END_PX = 250
        const val PROJECT_TOP_TAB_GAP_PX = 188
        const val PROJECT_TOP_TAB_TEXT_SP = 16f
        const val PROJECT_TOP_INDICATOR_WIDTH_PX = 98
        const val PROJECT_TOP_INDICATOR_HEIGHT_PX = 6
        const val PROJECT_TOP_INDICATOR_BOTTOM_PX = 18
        const val PROJECT_ADD_BUTTON_SIZE_PX = 156
        const val PROJECT_ADD_BUTTON_PADDING_PX = 16
    }
}
