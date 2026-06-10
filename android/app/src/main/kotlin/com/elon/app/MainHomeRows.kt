package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import java.text.DateFormat
import java.util.Date
import kotlin.math.sin

internal class MainHomeRows(
    private val activity: AppCompatActivity,
    private val timeFormatter: DateFormat,
    private val activeProjectIndexProvider: () -> Int,
    private val openProject: (Int) -> Unit,
    private val showProjectActions: (Int, View?) -> Unit,
    private val openConversation: (Int) -> Unit,
    private val showConversationActions: (Int) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private var conversationHomeRowAnimator: ValueAnimator? = null
    private var conversationHomeRowTarget: View? = null
    private val conversationHomeRowDetachListener = object : View.OnAttachStateChangeListener {
        override fun onViewAttachedToWindow(v: View) = Unit
        override fun onViewDetachedFromWindow(v: View) {
            if (conversationHomeRowTarget === v) {
                cancelHomeRowShimmer()
            }
        }
    }

    fun createFriendRow(friend: AppFriend, onClick: () -> Unit): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            )
            setBackgroundColor(Color.parseColor("#181B20"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(14), 0)
            clipChildren = false
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }

        row.addView(createFriendAvatar(friend))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(12)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = friend.name
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(4)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = friend.lastMessage ?: friend.phone ?: friend.account
            setTextColor(Color.parseColor(if (friend.unreadCount > 0) "#A6AFBD" else "#A6AFBD"))
            textSize = 13f
        })
        row.addView(middle)

        friend.lastMessageAt?.let { time ->
            row.addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    gravity = Gravity.TOP
                    marginStart = dp(8)
                    topMargin = dp(18)
                }
                includeFontPadding = false
                text = timeFormatter.format(Date(time))
                setTextColor(Color.parseColor("#A6AFBD"))
                textSize = 12f
            })
        }
        return row
    }

    fun createGroupRow(group: AppGroup, onClick: () -> Unit): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            )
            setBackgroundColor(Color.parseColor("#181B20"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(14), 0)
            clipChildren = false
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }

        row.addView(createGroupAvatar(group))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(12)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = group.name
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(4)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = group.lastMessage ?: "${group.memberCount} 位成员"
            setTextColor(Color.parseColor(if (group.unreadCount > 0) "#A6AFBD" else "#A6AFBD"))
            textSize = 13f
        })
        row.addView(middle)

        (group.lastMessageAt ?: group.createdAt)?.let { time ->
            row.addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    gravity = Gravity.TOP
                    marginStart = dp(8)
                    topMargin = dp(18)
                }
                includeFontPadding = false
                text = timeFormatter.format(Date(time))
                setTextColor(Color.parseColor("#A6AFBD"))
                textSize = 12f
            })
        }
        return row
    }

    fun createFriendPlaceholder(loggedIn: Boolean, onClick: () -> Unit): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(66)
            )
            setBackgroundColor(Color.parseColor("#181B20"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }
        row.addView(createAvatarView("+", 44, 20f))
        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(10)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            includeFontPadding = false
            maxLines = 1
            text = if (loggedIn) "暂无好友" else "登录后显示好友"
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            includeFontPadding = false
            maxLines = 1
            text = if (loggedIn) "点击右上角 + 添加好友" else "点击登录后按手机号添加好友"
            setTextColor(Color.parseColor("#A6AFBD"))
            textSize = 13f
        })
        row.addView(middle)
        return row
    }

    fun createProjectRow(index: Int, project: AppProject): View {
        val wrapper = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            ).apply {
                topMargin = if (index == 0) 0 else 1
            }
        }

        val row = LinearLayout(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.parseColor(if (index == activeProjectIndexProvider()) "#283140" else "#181B20"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(index) }
            setOnLongClickListener { anchor ->
                showProjectActions(index, anchor)
                true
            }
        }

        row.addView(createAvatarView(project.title, 44, 18f))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(12)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = project.title
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(5)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            val projectKind = project.projectKindLabel()
            text = "$projectKind · ${project.projectOriginLabel()} · ${project.displayConversationCount()} 个会话 · ${project.stage}"
            setTextColor(Color.parseColor("#A6AFBD"))
            textSize = 13f
        })
        row.addView(middle)

        row.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(8)
                topMargin = dp(17)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(project.updatedAt))
            setTextColor(Color.parseColor("#A6AFBD"))
            textSize = 13f
        })
        wrapper.addView(row)

        if (index == activeProjectIndexProvider()) {
            wrapper.addView(View(activity).apply {
                layoutParams = FrameLayout.LayoutParams(dp(8), dp(8)).apply {
                    gravity = Gravity.START or Gravity.TOP
                    leftMargin = dp(10)
                    topMargin = dp(10)
                }
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor("#FF4D4F"))
                }
            })
        }

        return wrapper
    }

    fun createConversationRow(index: Int, conversation: AppConversation, active: Boolean): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(66)
            )
            setBackgroundColor(Color.parseColor("#181B20"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openConversation(index) }
            setOnLongClickListener {
                showConversationActions(index)
                true
            }
        }

        row.addView(createAvatarView(conversation.title, 44, 17f))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(10)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = conversation.title
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(4)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = conversation.subtitle
            setTextColor(conversationSubtitleColor(conversation.subtitle))
            textSize = 13f
        })
        row.addView(middle)

        row.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(7)
                topMargin = dp(16)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(conversation.updatedAt))
            setTextColor(Color.parseColor("#A6AFBD"))
            textSize = 12f
        })
        updateConversationRowShimmer(row, active, false)
        return row
    }

    fun updateConversationRowShimmer(row: View, active: Boolean, homeRow: Boolean) {
        if (active) {
            startConversationRowShimmer(row, homeRow)
        } else {
            stopConversationRowShimmer(row, homeRow)
        }
    }

    fun createConversationDivider(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                marginStart = dp(68)
            }
            setBackgroundColor(Color.parseColor("#24282F"))
        }
    }

    fun cancelHomeRowShimmer() {
        conversationHomeRowAnimator?.cancel()
        conversationHomeRowAnimator = null
        conversationHomeRowTarget?.removeOnAttachStateChangeListener(conversationHomeRowDetachListener)
        conversationHomeRowTarget = null
    }

    private fun startConversationRowShimmer(row: View, homeRow: Boolean) {
        if (
            homeRow &&
            conversationHomeRowTarget === row &&
            conversationHomeRowAnimator?.isRunning == true
        ) {
            return
        }
        if (homeRow) {
            cancelHomeRowShimmer()
        }

        val baseColor = Color.parseColor("#181B20")
        val highlightColor = Color.parseColor("#283140")
        row.setBackgroundColor(baseColor)

        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1350L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { valueAnimator ->
                val fraction = valueAnimator.animatedFraction
                val pulse = sin(Math.PI * fraction).toFloat()
                row.setBackgroundColor(blendColor(baseColor, highlightColor, pulse))
            }
        }

        if (homeRow) {
            conversationHomeRowAnimator = animator
            conversationHomeRowTarget = row
            row.addOnAttachStateChangeListener(conversationHomeRowDetachListener)
        } else {
            row.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
                override fun onViewAttachedToWindow(v: View) = Unit
                override fun onViewDetachedFromWindow(v: View) {
                    animator.cancel()
                }
            })
        }
        animator.start()
    }

    private fun stopConversationRowShimmer(row: View, homeRow: Boolean) {
        if (homeRow) {
            cancelHomeRowShimmer()
        }
        row.setBackgroundColor(Color.parseColor("#181B20"))
    }

    private fun createAvatarView(
        title: String,
        sizeDp: Int,
        textSizeSp: Float,
        avatarDataUrl: String? = null
    ): View {
        val size = dp(sizeDp)
        if (title.startsWith(activity.getString(R.string.app_name))) {
            return ImageView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(size, size)
                contentDescription = activity.getString(R.string.app_name)
                scaleType = ImageView.ScaleType.FIT_CENTER
                setImageResource(R.drawable.ic_app_brand)
            }
        }

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
            setBackgroundResource(R.drawable.bg_mock_avatar)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = avatarText(title)
            setTextColor(Color.parseColor("#283140"))
            textSize = textSizeSp
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun createFriendAvatar(friend: AppFriend): View {
        val size = dp(44)
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            clipChildren = false
            clipToPadding = false
            elevation = dp(4).toFloat()
            translationZ = dp(4).toFloat()
            val avatar = createAvatarView(friend.name, 44, 17f, friend.avatarDataUrl).apply {
                layoutParams = FrameLayout.LayoutParams(size, size)
            }
            addView(avatar)
            if (friend.unreadCount > 0) {
                addView(createUnreadBadge(friend.unreadCount))
            }
            // 在线绿点（右下角，10dp，与未读红点不重叠）
            if (friend.isOnline) {
                addView(android.view.View(activity).apply {
                    val dotSize = dp(10)
                    layoutParams = FrameLayout.LayoutParams(dotSize, dotSize).apply {
                        gravity = Gravity.BOTTOM or Gravity.END
                        bottomMargin = -dp(1)
                        rightMargin = -dp(1)
                    }
                    background = GradientDrawable().apply {
                        shape = GradientDrawable.OVAL
                        setColor(Color.parseColor("#58BE6A"))
                        setStroke(dp(2), Color.parseColor("#181B20"))
                    }
                })
            }
        }
    }

    private fun createGroupAvatar(group: AppGroup): View {
        val size = dp(44)
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            clipChildren = false
            clipToPadding = false
            elevation = dp(4).toFloat()
            translationZ = dp(4).toFloat()
            addView(
                if (group.members.isEmpty()) createGroupFallbackAvatar(size)
                else createGroupMemberGrid(group.members.take(9), size)
            )
            if (group.unreadCount > 0) {
                addView(createUnreadBadge(group.unreadCount))
            }
        }
    }

    private fun createUnreadBadge(unreadCount: Int): TextView {
        val badgeText = if (unreadCount > 99) "99+" else unreadCount.toString()
        val badgeHeight = dp(22)
        val badgeWidth = when {
            badgeText.length >= 3 -> dp(34)
            badgeText.length == 2 -> dp(28)
            else -> badgeHeight
        }
        return TextView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(badgeWidth, badgeHeight).apply {
                gravity = Gravity.TOP or Gravity.END
                topMargin = -badgeHeight / 2
                rightMargin = -badgeHeight / 2
            }
            background = GradientDrawable().apply {
                shape = GradientDrawable.RECTANGLE
                cornerRadius = badgeHeight / 2f
                setColor(Color.parseColor("#F04B4F"))
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = badgeText
            setTextColor(Color.WHITE)
            textSize = 12f
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun createGroupFallbackAvatar(size: Int): View {
        return TextView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(size, size)
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.parseColor("#D6D6D6"))
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "群"
            setTextColor(Color.parseColor("#283140"))
            textSize = 17f
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun createGroupMemberGrid(members: List<AppGroupMember>, size: Int): View {
        return FrameLayout(activity).apply {
            layoutParams = FrameLayout.LayoutParams(size, size)
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.parseColor("#F2F5FA"))
            }

            val compactGrid = members.size <= 4
            val tileSize = if (compactGrid) dp(18) else dp(12)
            val gap = if (compactGrid) dp(3) else dp(2)
            val textSize = if (compactGrid) 9.5f else 7.5f
            val positions = groupAvatarPositions(members.size, size, tileSize, gap)
            members.forEachIndexed { index, member ->
                val position = positions.getOrNull(index) ?: return@forEachIndexed
                addView(
                    createGroupMemberTile(member, textSize),
                    FrameLayout.LayoutParams(tileSize, tileSize).apply {
                        leftMargin = position.first
                        topMargin = position.second
                    }
                )
            }
        }
    }

    private fun groupAvatarPositions(count: Int, size: Int, tileSize: Int, gap: Int): List<Pair<Int, Int>> {
        if (count == 2) {
            val contentWidth = tileSize * 2 + gap
            val left = (size - contentWidth) / 2
            val top = (size - tileSize) / 2
            return listOf(left to top, (left + tileSize + gap) to top)
        }
        if (count == 3) {
            val contentWidth = tileSize * 2 + gap
            val left = (size - contentWidth) / 2
            val top = (size - (tileSize * 2 + gap)) / 2
            return listOf(
                ((size - tileSize) / 2) to top,
                left to (top + tileSize + gap),
                (left + tileSize + gap) to (top + tileSize + gap)
            )
        }

        val columns = if (count <= 4) 2 else 3
        val rows = ((count + columns - 1) / columns).coerceAtMost(columns)
        val contentWidth = tileSize * columns + gap * (columns - 1)
        val contentHeight = tileSize * rows + gap * (rows - 1)
        val startLeft = (size - contentWidth) / 2
        val startTop = (size - contentHeight) / 2
        return List(count) { index ->
            val row = index / columns
            val col = index % columns
            (startLeft + col * (tileSize + gap)) to (startTop + row * (tileSize + gap))
        }
    }

    private fun createGroupMemberTile(member: AppGroupMember, textSizeSp: Float): View {
        val bitmap = UserProfileStore.decodeAvatar(groupMemberAvatarDataUrl(member))
        if (bitmap != null) {
            return ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(3).toFloat()
                })
            }
        }
        return TextView(activity).apply {
            background = GradientDrawable().apply {
                cornerRadius = dp(3).toFloat()
                setColor(Color.parseColor("#EFEFEF"))
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = UserProfileStore.avatarInitial(member.displayName)
            setTextColor(Color.parseColor("#283140"))
            textSize = textSizeSp
            setTypeface(typeface, Typeface.BOLD)
            maxLines = 1
        }
    }

    private fun groupMemberAvatarDataUrl(member: AppGroupMember): String? {
        member.avatarDataUrl?.takeIf { it.isNotBlank() }?.let { return it }
        if (member.id.isNotBlank() && member.id == AuthManager.effectiveUserId(activity)) {
            return UserProfileStore.load(activity).avatarDataUrl
        }
        return null
    }
}
