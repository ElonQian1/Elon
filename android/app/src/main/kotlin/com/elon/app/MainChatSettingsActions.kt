package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.app.Dialog
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.animation.PathInterpolator
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

internal class MainChatSettingsActions(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val clearFriendMessages: () -> Unit,
    private val clearGroupMessages: () -> Unit,
    private val onAddGroupMember: ((AppGroup, () -> Unit) -> Unit)? = null
) {
    private val prefs by lazy { AuthManager.userDataPrefs(activity) }
    private var pageAnimator: AnimatorSet? = null

    fun showFriendSettings(friend: AppFriend) {
        showSettingsPage("聊天信息") { dialog ->
            addView(contactStrip(friend))
            addView(sectionSpacer())
            addView(actionRow("查找聊天内容", "搜索与 ${friend.name} 的聊天记录") {
                toast("聊天内容搜索准备中")
            })
            addView(divider())
            addView(actionRow("设置当前聊天背景", "为这个聊天单独选择背景") {
                toast("聊天背景设置准备中")
            })
            addView(sectionSpacer())
            addView(toggleRow("消息免打扰", "chat_friend_mute_${friend.id}", false))
            addView(divider())
            addView(toggleRow("置顶聊天", "chat_friend_pin_${friend.id}", false))
            addView(divider())
            addView(toggleRow("强提醒", "chat_friend_alert_${friend.id}", false))
            addView(sectionSpacer())
            addView(actionRow("投诉", null) {
                toast("投诉入口准备中")
            })
            addView(divider())
            addView(destructiveRow("清空聊天记录") {
                clearFriendMessages()
                toast("已清空当前聊天记录")
                dismissWithAnimation(dialog)
            })
        }
    }

    fun showGroupSettings(group: AppGroup) {
        showSettingsPage("聊天信息") { dialog ->
            addView(groupMemberStrip(group) {
                onAddGroupMember?.invoke(group) { dismissWithAnimation(dialog) }
                    ?: toast("添加群成员准备中")
            })
            addView(sectionSpacer())
            addView(actionRow("群聊名称", group.name) {
                toast("群聊名称编辑准备中")
            })
            addView(divider())
            addView(actionRow("群公告", "暂无公告") {
                toast("群公告功能准备中")
            })
            addView(divider())
            addView(actionRow("查找聊天内容", "搜索这个群里的聊天记录") {
                toast("聊天内容搜索准备中")
            })
            addView(sectionSpacer())
            addView(toggleRow("消息免打扰", "chat_group_mute_${group.id}", false))
            addView(divider())
            addView(toggleRow("置顶聊天", "chat_group_pin_${group.id}", false))
            addView(divider())
            addView(toggleRow("保存到通讯录", "chat_group_save_${group.id}", true))
            addView(divider())
            addView(toggleRow("显示群成员昵称", "chat_group_show_names_${group.id}", true))
            addView(sectionSpacer())
            addView(actionRow("我在本群的昵称", "未设置") {
                toast("群昵称设置准备中")
            })
            addView(divider())
            addView(actionRow("群二维码", null) {
                toast("群二维码准备中")
            })
            addView(divider())
            addView(actionRow("群管理", null) {
                toast("群管理准备中")
            })
            addView(sectionSpacer())
            addView(destructiveRow("清空聊天记录") {
                clearGroupMessages()
                toast("已清空当前群聊记录")
                dismissWithAnimation(dialog)
            })
            addView(divider())
            addView(destructiveRow("退出群聊") {
                toast("退出群聊功能准备中")
            })
        }
    }

    private fun showSettingsPage(title: String, contentBuilder: LinearLayout.(Dialog) -> Unit) {
        val dialog = Dialog(activity, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
        dialog.requestWindowFeature(Window.FEATURE_NO_TITLE)
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor("#101010"))
            translationX = activity.resources.displayMetrics.widthPixels.toFloat()
        }
        root.addView(topBar(title) { dismissWithAnimation(dialog) })
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(10), 0, dp(28))
            contentBuilder(dialog)
        }
        root.addView(ScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0,
                1f
            )
            addView(content)
        })
        dialog.setContentView(root)
        dialog.setOnShowListener {
            dialog.window?.let { window ->
                window.setLayout(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT
                )
                window.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
                window.attributes = window.attributes.apply {
                    dimAmount = 0f
                    windowAnimations = 0
                }
            }
            root.post {
                playPageSlide(root, pageWidth(root), 0f)
            }
        }
        dialog.window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        dialog.show()
    }

    private fun dismissWithAnimation(dialog: Dialog) {
        val root = dialog.window?.decorView?.findViewById<ViewGroup>(android.R.id.content)
            ?.getChildAt(0)
        if (root == null) {
            dialog.dismiss()
            return
        }
        playPageSlide(root, root.translationX, pageWidth(root)) {
            dialog.dismiss()
        }
    }

    private fun pageWidth(page: View): Float {
        return (page.width.takeIf { it > 0 } ?: activity.resources.displayMetrics.widthPixels).toFloat()
    }

    private fun playPageSlide(page: View, from: Float, to: Float, onEnd: () -> Unit = {}) {
        pageAnimator?.cancel()
        page.translationX = from
        page.setLayerType(View.LAYER_TYPE_HARDWARE, null)
        AnimatorSet().apply {
            pageAnimator = this
            duration = PAGE_ANIMATION_MS
            interpolator = PAGE_INTERPOLATOR
            playTogether(ObjectAnimator.ofFloat(page, View.TRANSLATION_X, from, to))
            addListener(object : AnimatorListenerAdapter() {
                private var cancelled = false

                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                    if (pageAnimator === animation) pageAnimator = null
                }

                override fun onAnimationEnd(animation: Animator) {
                    if (pageAnimator === animation) pageAnimator = null
                    page.setLayerType(View.LAYER_TYPE_NONE, null)
                    if (!cancelled) onEnd()
                }
            })
            start()
        }
    }

    private fun topBar(title: String, onBack: () -> Unit): View {
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(50)
            )
            setBackgroundColor(Color.parseColor("#101010"))
            addView(TextView(activity).apply {
                layoutParams = FrameLayout.LayoutParams(dp(50), FrameLayout.LayoutParams.MATCH_PARENT).apply {
                    gravity = Gravity.START or Gravity.CENTER_VERTICAL
                }
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "‹"
                setTextColor(Color.parseColor("#F2F5FA"))
                textSize = 31f
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { onBack() }
            })
            addView(TextView(activity).apply {
                layoutParams = FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ).apply {
                    gravity = Gravity.CENTER
                }
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#F2F5FA"))
                textSize = 17f
            })
        }
    }

    private fun contactStrip(friend: AppFriend): View {
        return HorizontalScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(108)
            )
            setBackgroundColor(Color.parseColor("#181B20"))
            overScrollMode = View.OVER_SCROLL_NEVER
            isHorizontalScrollBarEnabled = false
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(18), 0, dp(18), 0)
                addView(personTile(friend.name, friend.avatarDataUrl))
                addView(addTile { toast("添加成员准备中") })
            })
        }
    }

    private fun groupMemberStrip(group: AppGroup, onAddMember: () -> Unit): View {
        return HorizontalScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(126)
            )
            setBackgroundColor(Color.parseColor("#181B20"))
            overScrollMode = View.OVER_SCROLL_NEVER
            isHorizontalScrollBarEnabled = false
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(18), 0, dp(18), 0)
                group.members.take(12).forEach { member ->
                    addView(personTile(member.displayName, member.avatarDataUrl))
                }
                addView(addTile { onAddMember() })
            })
        }
    }

    private fun personTile(name: String, avatarDataUrl: String?): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(62), LinearLayout.LayoutParams.MATCH_PARENT).apply {
                marginEnd = dp(12)
            }
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            addView(avatarView(name, avatarDataUrl, 46, 17f))
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(8)
                }
                gravity = Gravity.CENTER
                includeFontPadding = false
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
                text = name
                setTextColor(Color.parseColor("#A6AFBD"))
                textSize = 12f
            })
        }
    }

    private fun addTile(onClick: () -> Unit): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(62), LinearLayout.LayoutParams.MATCH_PARENT)
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(46), dp(46))
                background = roundedBg("#283140", 8)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "+"
                setTextColor(Color.parseColor("#CFCFCF"))
                textSize = 27f
            })
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(8)
                }
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "添加"
                setTextColor(Color.parseColor("#A6AFBD"))
                textSize = 12f
            })
        }
    }

    private fun avatarView(name: String, avatarDataUrl: String?, sizeDp: Int, textSizeSp: Float): View {
        val size = dp(sizeDp)
        val bitmap = UserProfileStore.decodeAvatar(avatarDataUrl)
        if (bitmap != null) {
            return ImageView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(size, size)
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(8).toFloat()
                })
            }
        }
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            background = roundedBg("#D6D6D6", 8)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = UserProfileStore.avatarInitial(name)
            setTextColor(Color.parseColor("#283140"))
            textSize = textSizeSp
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun actionRow(title: String, subtitle: String?, action: () -> Unit): View {
        return baseRow().apply {
            addView(labelColumn(title, subtitle), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(chevron())
            setOnClickListener { action() }
        }
    }

    private fun destructiveRow(title: String, action: () -> Unit): View {
        return baseRow().apply {
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#E66B6B"))
                textSize = 15.5f
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(chevron())
            setOnClickListener { action() }
        }
    }

    private fun toggleRow(title: String, key: String, defaultOn: Boolean): View {
        var enabled = prefs.getBoolean(key, defaultOn)
        lateinit var status: TextView
        return baseRow().apply {
            addView(labelColumn(title, null), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            status = TextView(activity).apply {
                minWidth = dp(48)
                gravity = Gravity.CENTER
                includeFontPadding = false
                background = toggleBg(enabled)
                text = if (enabled) "开" else "关"
                setTextColor(Color.parseColor(if (enabled) "#101010" else "#A6AFBD"))
                textSize = 13f
            }
            addView(status, LinearLayout.LayoutParams(dp(48), dp(26)))
            setOnClickListener {
                enabled = !enabled
                prefs.edit().putBoolean(key, enabled).apply()
                status.background = toggleBg(enabled)
                status.text = if (enabled) "开" else "关"
                status.setTextColor(Color.parseColor(if (enabled) "#101010" else "#A6AFBD"))
            }
        }
    }

    private fun baseRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(54)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(18), 0, dp(18), 0)
            setBackgroundColor(Color.parseColor("#181B20"))
            isClickable = true
            foreground = selectableForeground()
        }
    }

    private fun labelColumn(title: String, subtitle: String?): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#F2F5FA"))
                textSize = 15.5f
            })
            if (!subtitle.isNullOrBlank()) {
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = subtitle
                    setTextColor(Color.parseColor("#6F7785"))
                    textSize = 12f
                    maxLines = 1
                    ellipsize = android.text.TextUtils.TruncateAt.END
                })
            }
        }
    }

    private fun chevron(): TextView {
        return TextView(activity).apply {
            includeFontPadding = false
            text = "›"
            setTextColor(Color.parseColor("#6F7785"))
            textSize = 24f
        }
    }

    private fun divider(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                marginStart = dp(18)
            }
            setBackgroundColor(Color.parseColor("#2D2D2D"))
        }
    }

    private fun sectionSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(10)
            )
            setBackgroundColor(Color.parseColor("#101010"))
        }
    }

    private fun roundedBg(color: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = dp(radiusDp).toFloat()
            setColor(Color.parseColor(color))
        }
    }

    private fun toggleBg(enabled: Boolean): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = dp(13).toFloat()
            setColor(Color.parseColor(if (enabled) "#F2F5FA" else "#283140"))
        }
    }

    private fun toast(text: String) {
        Toast.makeText(activity, text, Toast.LENGTH_SHORT).show()
    }

    private companion object {
        const val PAGE_ANIMATION_MS = 260L
        val PAGE_INTERPOLATOR = PathInterpolator(0.2f, 0f, 0f, 1f)
    }
}
