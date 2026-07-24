package com.elon.app

import android.content.ClipData
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.util.Locale

internal enum class SocialSidebarTab {
    DATE,
    FAVORITES
}

internal data class SocialTimelineDragPayload(val message: ChatMessage)

private data class LoadedSocialTimelineMessage(
    val lastReceivedAt: Long,
    val message: ChatMessage
)

internal class ChatSocialSideMenuView(
    context: Context,
    private val timelineItems: () -> List<SocialSidebarTimelineItem>,
    private val favoriteItems: () -> List<SocialSidebarFavorite>,
    private val openConversation: (SocialSidebarTimelineItem) -> Unit,
    private val loadTimelineMessage: (SocialSidebarTimelineItem, (Result<ChatMessage>) -> Unit) -> Unit,
    private val openSettings: () -> Unit,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    initialTab: SocialSidebarTab = SocialSidebarTab.DATE,
    initialDate: LocalDate = LocalDate.now()
) : FrameLayout(context) {
    private var selectedTab = initialTab
    private var selectedDate = initialDate
    private var selectedFilter = SocialSidebarContentType.ALL
    private var searchVisible = false
    private var searchQuery = ""
    private val loadedMessages =
        mutableMapOf<SocialSidebarConversationKey, LoadedSocialTimelineMessage>()
    private val root = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(22), dp(36), dp(18), dp(18))
        setBackgroundColor(Color.parseColor("#0D0D0D"))
    }

    init {
        addView(root, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
    }

    fun render() {
        val currentReceivedTimes = timelineItems().associate { it.key to it.lastReceivedAt }
        loadedMessages.entries.removeAll { (key, cached) ->
            currentReceivedTimes[key] != cached.lastReceivedAt
        }
        root.removeAllViews()
        root.addView(topTabs())
        if (searchVisible) root.addView(searchField())
        if (selectedTab == SocialSidebarTab.DATE) {
            root.addView(createSocialSidebarDateStrip(
                context = context,
                selectedDate = selectedDate,
                onDateSelected = { date ->
                    selectedDate = date
                    render()
                },
                dp = dp,
                selectableForeground = selectableForeground
            ))
        }
        root.addView(timelineScroll(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1f
        ))
        root.addView(filterDock())
    }

    private fun topTabs(): LinearLayout = LinearLayout(context).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(48))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(tabText(
            selectedDate.run { "${monthValue}月${dayOfMonth}号" },
            selectedTab == SocialSidebarTab.DATE
        ) {
            selectedTab = SocialSidebarTab.DATE
            render()
        }, LinearLayout.LayoutParams(dp(110), LinearLayout.LayoutParams.MATCH_PARENT))
        addView(tabText("收藏", selectedTab == SocialSidebarTab.FAVORITES) {
            selectedTab = SocialSidebarTab.FAVORITES
            render()
        }, LinearLayout.LayoutParams(dp(82), LinearLayout.LayoutParams.MATCH_PARENT))
        addView(
            View(context),
            LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
        )
        addView(ImageView(context).apply {
            setImageResource(R.drawable.social_sidebar_search)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            contentDescription = "搜索侧栏消息"
            isClickable = true
            foreground = selectableForeground()
            setPadding(dp(12), dp(12), dp(12), dp(12))
            setOnClickListener {
                searchVisible = !searchVisible
                if (!searchVisible) searchQuery = ""
                render()
            }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun tabText(title: String, selected: Boolean, onClick: () -> Unit) =
        TextView(context).apply {
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            text = title
            textSize = 18f
            setSingleLine(true)
            setTypeface(typeface, Typeface.NORMAL)
            setTextColor(Color.parseColor(if (selected) "#4F9DFF" else "#D9D9D9"))
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "$title${if (selected) "，已选中" else ""}"
            setOnClickListener { onClick() }
        }

    private fun searchField(): EditText = EditText(context).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(48)).apply {
            topMargin = dp(8)
            bottomMargin = dp(8)
        }
        background = roundedRect("#272727", 24)
        setPadding(dp(18), 0, dp(18), 0)
        setTextColor(Color.parseColor("#D9D9D9"))
        setHintTextColor(Color.parseColor("#AFAFAF"))
        textSize = 15f
        hint = "搜索名称或消息内容"
        setSingleLine(true)
        setText(searchQuery)
        setSelection(text.length)
        addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                searchQuery = s?.toString().orEmpty()
            }
            override fun afterTextChanged(s: Editable?) {
                post { renderTimelineOnly() }
            }
        })
    }

    private fun renderTimelineOnly() {
        render()
        if (searchVisible) {
            (root.getChildAt(1) as? EditText)?.requestFocus()
        }
    }

    private fun timelineScroll(): ScrollView = ScrollView(context).apply {
        overScrollMode = View.OVER_SCROLL_NEVER
        isVerticalScrollBarEnabled = false
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            val items = displayedItems()
            if (items.isEmpty()) {
                addView(TextView(context).apply {
                    gravity = Gravity.CENTER
                    text = if (selectedTab == SocialSidebarTab.DATE) "这一天暂无其他会话消息" else "暂无收藏内容"
                    textSize = 14f
                    setTextColor(Color.parseColor("#777777"))
                }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(140)))
            } else {
                items.forEach { item ->
                    addView(if (selectedTab == SocialSidebarTab.DATE) dateTimelineRow(item) else favoriteTimelineRow(item))
                }
            }
        })
    }

    private fun displayedItems(): List<SocialSidebarTimelineItem> {
        val base = if (selectedTab == SocialSidebarTab.DATE) {
            timelineItems()
                .filter { item ->
                    Instant.ofEpochMilli(item.lastReceivedAt)
                        .atZone(ZoneId.systemDefault())
                        .toLocalDate() == selectedDate
                }
        } else {
            favoriteItems().map { favorite ->
                SocialSidebarTimelineItem(
                    key = SocialSidebarConversationKey(
                        SocialSidebarConversationType.FRIEND,
                        "favorite:${favorite.id}"
                    ),
                    name = "",
                    avatarDataUrl = null,
                    summary = favorite.message.content.ifBlank {
                        previewTextForChatContent("", favorite.message.attachments)
                    },
                    lastReceivedAt = favorite.message.createdAtMs,
                    unreadCount = 0,
                    message = favorite.message
                )
            }
        }
        val query = searchQuery.trim()
        return base.filter { item ->
            item.matchesSocialSidebarFilter(selectedFilter) &&
                (query.isBlank() || item.name.contains(query, true) || item.summary.contains(query, true))
        }
    }

    private fun dateTimelineRow(item: SocialSidebarTimelineItem): LinearLayout =
        timelineShell(minHeightDp = 154).apply {
            addView(timelineSpine(), LinearLayout.LayoutParams(dp(34), LinearLayout.LayoutParams.MATCH_PARENT))
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                addView(LinearLayout(context).apply {
                    gravity = Gravity.CENTER_VERTICAL
                    orientation = LinearLayout.HORIZONTAL
                    addView(avatar(item), LinearLayout.LayoutParams(dp(54), dp(54)))
                    addView(TextView(context).apply {
                        includeFontPadding = false
                        maxLines = 1
                        ellipsize = TextUtils.TruncateAt.END
                        text = item.name
                        textSize = 18f
                        setTextColor(Color.parseColor("#D9D9D9"))
                    }, LinearLayout.LayoutParams(0, dp(54), 1f).apply { leftMargin = dp(12) })
                    addView(timeText(item.lastReceivedAt), LinearLayout.LayoutParams(dp(64), dp(54)))
                })
                addView(messagePreview(item), LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(10)
                    bottomMargin = dp(20)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                leftMargin = dp(8)
            })
        }

    private fun favoriteTimelineRow(item: SocialSidebarTimelineItem): LinearLayout =
        timelineShell(minHeightDp = 144).apply {
            addView(timelineSpine(), LinearLayout.LayoutParams(dp(34), LinearLayout.LayoutParams.MATCH_PARENT))
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                addView(TextView(context).apply {
                    gravity = Gravity.END or Gravity.CENTER_VERTICAL
                    includeFontPadding = false
                    text = formatSidebarDate(item.lastReceivedAt)
                    textSize = 15f
                    setTextColor(Color.parseColor("#D9D9D9"))
                }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(42)))
                addView(messagePreview(item), LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { bottomMargin = dp(22) })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                leftMargin = dp(8)
            })
        }

    private fun timelineShell(minHeightDp: Int) = LinearLayout(context).apply {
        minimumHeight = dp(minHeightDp)
        orientation = LinearLayout.HORIZONTAL
        clipChildren = false
        clipToPadding = false
    }

    private fun timelineSpine(): FrameLayout = FrameLayout(context).apply {
        addView(View(context).apply {
            setBackgroundColor(Color.parseColor("#D9D9D9"))
        }, LayoutParams(dp(2), LayoutParams.MATCH_PARENT).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            topMargin = dp(24)
        })
        addView(ImageView(context).apply {
            setImageResource(R.drawable.social_sidebar_timeline_dot)
            scaleType = ImageView.ScaleType.FIT_CENTER
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }, LayoutParams(dp(24), dp(24)).apply {
            gravity = Gravity.TOP or Gravity.CENTER_HORIZONTAL
        })
    }

    private fun avatar(item: SocialSidebarTimelineItem): FrameLayout = FrameLayout(context).apply {
        clipChildren = false
        clipToPadding = false
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "打开${item.name}聊天"
        val bitmap = UserProfileStore.decodeAvatar(item.avatarDataUrl)
        addView(ImageView(context).apply {
            scaleType = ImageView.ScaleType.CENTER_CROP
            if (bitmap == null) {
                setImageResource(R.drawable.social_sidebar_avatar_placeholder)
            } else {
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(10).toFloat()
                })
            }
        }, LayoutParams(dp(46), dp(46)).apply { gravity = Gravity.CENTER })
        if (item.unreadCount > 0) addView(unreadBadge(item.unreadCount))
        setOnClickListener {
            requestClose(true)
            postDelayed({ openConversation(item) }, CLOSE_DELAY_MS)
        }
    }

    private fun unreadBadge(count: Int): TextView {
        val value = if (count > 99) "99+" else count.toString()
        val width = when {
            value.length >= 3 -> 34
            value.length == 2 -> 28
            else -> 22
        }
        return TextView(context).apply {
            layoutParams = LayoutParams(dp(width), dp(22)).apply {
                gravity = Gravity.TOP or Gravity.END
                topMargin = -dp(11)
                rightMargin = -dp(11)
            }
            background = roundedRect("#F04B4F", 11)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = value
            textSize = 12f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.WHITE)
        }
    }

    private fun timeText(time: Long) = TextView(context).apply {
        gravity = Gravity.END or Gravity.CENTER_VERTICAL
        includeFontPadding = false
        text = java.text.SimpleDateFormat("HH:mm", Locale.getDefault()).format(java.util.Date(time))
        textSize = 15f
        setTextColor(Color.parseColor("#D9D9D9"))
    }

    private fun messagePreview(item: SocialSidebarTimelineItem): View {
        val message = item.message ?: cachedTimelineMessage(item)
        val type = socialSidebarContentType(message?.content ?: item.summary, message?.attachments.orEmpty())
        val preview = when (type) {
            SocialSidebarContentType.MEDIA -> mediaPreview(item, message)
            SocialSidebarContentType.FILE -> filePreview(message?.attachments?.firstOrNull()?.displayName ?: item.summary)
            else -> textPreview(message?.content?.ifBlank { item.summary } ?: item.summary)
        }
        preview.isLongClickable = true
        preview.contentDescription = "长按拖拽消息到当前聊天"
        preview.setOnLongClickListener { source ->
            source.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
            val ready = item.message ?: cachedTimelineMessage(item)
            if (ready != null) {
                startMessageDrag(source, ready)
            } else {
                loadTimelineMessage(item) { result ->
                    result.onSuccess { loaded ->
                        val current = timelineItems().firstOrNull { it.key == item.key }
                        if (current?.lastReceivedAt == item.lastReceivedAt) {
                            loadedMessages[item.key] =
                                LoadedSocialTimelineMessage(item.lastReceivedAt, loaded)
                            startMessageDrag(source, loaded)
                        } else {
                            Toast.makeText(context, "消息已更新，请重新长按", Toast.LENGTH_SHORT).show()
                            render()
                        }
                    }.onFailure { error ->
                        Toast.makeText(context, error.message ?: "消息读取失败", Toast.LENGTH_SHORT).show()
                    }
                }
            }
            true
        }
        return preview
    }

    private fun cachedTimelineMessage(item: SocialSidebarTimelineItem): ChatMessage? =
        loadedMessages[item.key]
            ?.takeIf { it.lastReceivedAt == item.lastReceivedAt }
            ?.message

    private fun textPreview(value: String): TextView = TextView(context).apply {
        includeFontPadding = false
        maxLines = 4
        ellipsize = TextUtils.TruncateAt.END
        text = value
        textSize = 17f
        setLineSpacing(dp(4).toFloat(), 1f)
        setTextColor(Color.parseColor("#D9D9D9"))
        setPadding(dp(4), dp(8), dp(4), dp(10))
    }

    private fun mediaPreview(item: SocialSidebarTimelineItem, message: ChatMessage?): FrameLayout {
        val isVideo = (message?.attachments.orEmpty().any {
            it.kind == "video" || it.mimeType.orEmpty().startsWith("video/")
        }) || item.summary.contains("视频")
        return FrameLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                if (isVideo) dp(176) else dp(142),
                if (isVideo) dp(96) else dp(128)
            )
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.TRANSPARENT)
                setStroke(dp(1), Color.parseColor("#606060"))
            }
            if (isVideo) {
                addView(ImageView(context).apply {
                    setImageResource(R.drawable.social_sidebar_play)
                    scaleType = ImageView.ScaleType.FIT_CENTER
                }, LayoutParams(dp(30), dp(42)).apply { gravity = Gravity.CENTER })
            } else {
                addView(TextView(context).apply {
                    gravity = Gravity.CENTER
                    text = "▧"
                    textSize = 34f
                    setTextColor(Color.parseColor("#D9D9D9"))
                }, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
            }
        }
    }

    private fun filePreview(value: String): TextView = textPreview(value).apply {
        background = roundedRect("#1A1A1A", 10)
        compoundDrawablePadding = dp(10)
        text = "文件  $value"
    }

    private fun startMessageDrag(source: View, message: ChatMessage) {
        val clip = ClipData.newPlainText("social-message", message.content)
        source.startDragAndDrop(
            clip,
            View.DragShadowBuilder(source),
            SocialTimelineDragPayload(message.copyForDrag()),
            0
        )
    }

    private fun ChatMessage.copyForDrag() = ChatMessage(
        role = role,
        content = content,
        attachments = attachments?.map { it.copy() },
        id = id,
        createdAtMs = createdAtMs
    )

    private fun filterDock(): LinearLayout = createSocialSidebarFilterDock(
        context = context,
        selectedFilter = selectedFilter,
        onFilterSelected = { type ->
            selectedFilter = type
            render()
        },
        openSettings = openSettings,
        dp = dp,
        selectableForeground = selectableForeground
    )

    private fun roundedRect(color: String, radiusDp: Int) = GradientDrawable().apply {
        cornerRadius = dp(radiusDp).toFloat()
        setColor(Color.parseColor(color))
    }

    private fun formatSidebarDate(time: Long): String =
        Instant.ofEpochMilli(time).atZone(ZoneId.systemDefault()).toLocalDate()
            .run { "${monthValue}月${dayOfMonth}日" }

    private companion object {
        const val CLOSE_DELAY_MS = 220L
    }
}
