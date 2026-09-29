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
import android.content.Context
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import java.text.DateFormat
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.util.Date
import java.util.Locale
import kotlin.math.sin

private val homeListClockFormatter = DateTimeFormatter.ofPattern("HH:mm", Locale.CHINA)
private val homeListMonthDayFormatter = DateTimeFormatter.ofPattern("M月d日", Locale.CHINA)
private val homeListYearMonthDayFormatter = DateTimeFormatter.ofPattern("yyyy年M月d日", Locale.CHINA)
private val homeListWeekdays = arrayOf("周一", "周二", "周三", "周四", "周五", "周六", "周日")

private fun formatHomeListTime(timestampMs: Long, nowMs: Long = System.currentTimeMillis()): String {
    if (timestampMs <= 0L) return ""
    val zone = ZoneId.systemDefault()
    val dateTime = Instant.ofEpochMilli(timestampMs).atZone(zone)
    val today = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate()
    val date = dateTime.toLocalDate()
    val dayDiff = ChronoUnit.DAYS.between(date, today)
    return when {
        dayDiff == 0L -> dateTime.format(homeListClockFormatter)
        dayDiff == 1L -> "昨天"
        dayDiff == 2L -> "前天"
        dayDiff in 3L..6L -> homeListWeekdays[dateTime.dayOfWeek.value - 1]
        date.year == today.year -> dateTime.format(homeListMonthDayFormatter)
        else -> dateTime.format(homeListYearMonthDayFormatter)
    }
}

internal class MainHomeRows(
    private val activity: Context,
    private val timeFormatter: DateFormat,
    private val activeProjectIndexProvider: () -> Int,
    private val openProject: (Int) -> Unit,
    private val showProjectActions: (Int, View?) -> Unit,
    private val openConversation: (Int) -> Unit,
    private val showRenameConversationDialog: (Int) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private val uiColors by lazy { MobileColors(activity) }

    private val statusDecorations = HomeRowStatusDecorations(activity, dp)
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

    fun createFriendRow(
        friend: AppFriend,
        showProjectMarker: Boolean = false,
        projectWorking: Boolean = false,
        projectCompletionCount: Int = 0,
        onClick: () -> Unit
    ): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            minimumHeight = dp(72)
            setBackgroundColor(uiColors.surface)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), dp(12), dp(16), dp(12))
            clipChildren = false
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }

        row.addView(createFriendAvatar(friend, showProjectMarker, projectCompletionCount))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(16)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(statusDecorations.createTitle(
            friend.name,
            when {
                friend.isSocialAi() -> HomeRowBadge.AI
                showProjectMarker -> HomeRowBadge.PROJECT
                else -> null
            },
        ))
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
            text = friend.lastMessage ?: "\u6682\u65e0\u6d88\u606f"
            setTextColor(uiColors.muted)
            textSize = 14f; typeface = Typeface.create("sans-serif", Typeface.NORMAL)
        })
        row.addView(middle)

        createHomeRowTrailing(friend.lastMessageAt, projectWorking)?.let { trailing ->
            row.addView(trailing)
        }
        return row
    }

    fun createGroupRow(
        group: AppGroup,
        showProjectMarker: Boolean = false,
        projectWorking: Boolean = false,
        projectCompletionCount: Int = 0,
        onClick: () -> Unit
    ): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            minimumHeight = dp(72)
            setBackgroundColor(uiColors.surface)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), dp(12), dp(16), dp(12))
            clipChildren = false
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }

        row.addView(createGroupAvatar(group, showProjectMarker, projectCompletionCount))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(16)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(statusDecorations.createTitle(
            group.name,
            if (showProjectMarker) HomeRowBadge.PROJECT else null,
        ))
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
            setTextColor(uiColors.muted)
            textSize = 14f; typeface = Typeface.create("sans-serif", Typeface.NORMAL)
        })
        row.addView(middle)

        createHomeRowTrailing(group.lastMessageAt ?: group.createdAt, projectWorking)?.let { trailing ->
            row.addView(trailing)
        }
        return row
    }

    private fun createHomeRowTrailing(time: Long?, projectWorking: Boolean): View? {
        if (time == null && !projectWorking) return null

        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.MATCH_PARENT
            ).apply {
                marginStart = dp(8)
            }
            if (projectWorking) {
                minimumWidth = dp(44)
            }
            gravity = Gravity.END
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(12), 0, 0)
            clipChildren = false
            clipToPadding = false

            time?.let { value ->
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    )
                    includeFontPadding = false
                    text = formatHomeListTime(value)
                    setTextColor(uiColors.muted)
                    textSize = 12f; typeface = Typeface.create("sans-serif", Typeface.NORMAL); fontFeatureSettings = "tnum"
                })
            }

            if (projectWorking) {
                addView(statusDecorations.createWorkingIndicator(), LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                    topMargin = if (time == null) 0 else dp(2)
                })
            }
        }
    }

    fun createFriendPlaceholder(loggedIn: Boolean, onClick: () -> Unit): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(66)
            )
            setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
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
            setTextColor(uiColors.text)
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            includeFontPadding = false
            maxLines = 1
            text = if (loggedIn) "点击右上角 + 添加好友" else "点击登录后按手机号添加好友"
            setTextColor(uiColors.muted)
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
            setBackgroundColor(if (index == activeProjectIndexProvider()) uiColors.elevated else uiColors.container)
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
            setTextColor(uiColors.text)
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
            setTextColor(uiColors.muted)
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
            setTextColor(uiColors.muted)
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
                    setColor(uiColors.error)
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
            setBackgroundColor(uiColors.container)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openConversation(index) }
            setOnLongClickListener {
                showRenameConversationDialog(index)
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
            setTextColor(uiColors.text)
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
            setTextColor(uiColors.muted)
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
            setBackgroundColor(uiColors.outline)
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

        val baseColor = uiColors.container
        val highlightColor = uiColors.elevated
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
        row.setBackgroundColor(uiColors.container)
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
                setImageResource(R.drawable.ic_home_ai_avatar)
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
            background = GradientDrawable().apply { setColor(uiColors.elevated); cornerRadius = dp(10).toFloat() }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = avatarText(title)
            setTextColor(uiColors.muted)
            textSize = textSizeSp
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun createFriendAvatar(
        friend: AppFriend,
        showProjectMarker: Boolean = false,
        projectCompletionCount: Int = 0
    ): View {
        val size = dp(48)
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            clipChildren = false
            clipToPadding = false
            elevation = 0f
            translationZ = 0f
            val avatar = createAvatarView(friend.name, 48, 18f, friend.avatarDataUrl).apply {
                layoutParams = FrameLayout.LayoutParams(size, size)
            }
            addView(avatar)
            val unreadCount = projectCompletionCount.takeIf { it > 0 } ?: friend.unreadCount
            if (unreadCount > 0) {
                addView(createUnreadBadge(unreadCount))
            }
            if (showProjectMarker) {
                addView(createAvatarCornerDot(uiColors.warning))
            } else if (friend.isOnline) {
                addView(createAvatarCornerDot(uiColors.success))
            }
        }
    }

    private fun createGroupAvatar(
        group: AppGroup,
        showProjectMarker: Boolean = false,
        projectCompletionCount: Int = 0
    ): View {
        val size = dp(48)
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            clipChildren = false
            clipToPadding = false
            elevation = 0f
            translationZ = 0f
            addView(
                if (group.members.isEmpty()) createGroupFallbackAvatar(size)
                else createGroupMemberGrid(group.members.take(9), size)
            )
            val unreadCount = projectCompletionCount.takeIf { it > 0 } ?: group.unreadCount
            if (unreadCount > 0) {
                addView(createUnreadBadge(unreadCount))
            }
            if (showProjectMarker) {
                addView(createAvatarCornerDot(uiColors.warning))
            }
        }
    }

    private fun createAvatarCornerDot(colorHex: Int): View {
        return View(activity).apply {
            val dotSize = dp(10)
            layoutParams = FrameLayout.LayoutParams(dotSize, dotSize).apply {
                gravity = Gravity.BOTTOM or Gravity.END
                bottomMargin = -dp(1)
                rightMargin = -dp(1)
            }
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(colorHex)
                setStroke(dp(2), uiColors.container)
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
                setColor(uiColors.errorContainer)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = badgeText
            setTextColor(uiColors.error)
            textSize = 12f
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun createGroupFallbackAvatar(size: Int): View {
        return TextView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(size, size)
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(uiColors.elevated)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "群"
            setTextColor(uiColors.muted)
            textSize = 17f
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    private fun createGroupMemberGrid(members: List<AppGroupMember>, size: Int): View {
        return FrameLayout(activity).apply {
            layoutParams = FrameLayout.LayoutParams(size, size)
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(uiColors.elevated)
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
                setColor(uiColors.container)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = UserProfileStore.avatarInitial(member.displayName)
            setTextColor(uiColors.muted)
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
