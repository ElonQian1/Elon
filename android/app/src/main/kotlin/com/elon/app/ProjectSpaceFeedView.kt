package com.elon.app

import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import kotlin.concurrent.thread

internal class ProjectSpaceFeedView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val openPost: (ProjectChannel, ProjectChannelMessage) -> Unit,
    private val openPostComposer: () -> Unit,
    private val openAnnouncementEditor: (ProjectChannel, String) -> Unit
) {
    private val metricPrefs = activity.getSharedPreferences(POST_METRIC_PREFS, Context.MODE_PRIVATE)

    fun render(
        container: LinearLayout,
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>,
        loading: Boolean
    ) {
        val feedShell = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(22)
            }
        }
        feedShell.addView(announcementBlock(space, messagesByChannel))

        val frame = FrameLayout(activity).apply {
            minimumHeight = dp(464)
            setPadding(0, 0, 0, dp(34))
            background = roundedBackground(
                colorHex = "#101010",
                topStartDp = 18,
                topEndDp = 18,
                bottomEndDp = 0,
                bottomStartDp = 0
            )
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = -dp(10)
            }
        }
        val feedColumn = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(10), 0, 0)
        }
        frame.addView(feedColumn, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ))

        val posts = feedPosts(space, messagesByChannel)
        when {
            posts.isNotEmpty() -> posts.forEach { feedColumn.addView(postCard(it)) }
            loading -> feedColumn.addView(emptyState("正在加载帖子...", showButton = false))
            else -> feedColumn.addView(emptyState("还没有帖子，点击+好发布内容", showButton = true))
        }

        feedShell.addView(frame)
        container.addView(feedShell)
    }

    private fun announcementBlock(
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>
    ): LinearLayout {
        val announcement = space.channels.firstOrNull { it.kind == "announcements" }
        val latest = announcement?.let { channel ->
            messagesByChannel[channel.id]
                .orEmpty()
                .maxByOrNull { parseChatMessageCreatedAt(it.createdAt) ?: 0L }
        }
        val textValue = cleanAnnouncementText(latest?.content)
            ?: cleanAnnouncementText(announcement?.lastMessage)
            ?: "不得发布与主题内容不相关的帖子。"
        val displayText = parseProjectSpacePostText(textValue).detailText
            .takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
            ?: "不得发布与主题内容不相关的帖子。"
        val announcementChannel = announcement
        val editable = announcementChannel != null && canEditProjectAnnouncement(space.project.role)

        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(34))
            background = roundedBackground(
                colorHex = "#40FFFFFF",
                topStartDp = 18,
                topEndDp = 18,
                bottomEndDp = 0,
                bottomStartDp = 0
            )
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            if (editable) {
                isClickable = true
                foreground = selectableForeground()
                contentDescription = "编辑项目公告"
                setOnClickListener {
                    announcementChannel?.let { openAnnouncementEditor(it, displayText) }
                }
            }
            addView(TextView(activity).apply {
                text = "公告"
                textSize = 15f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#101010"))
            })
            addView(TextView(activity).apply {
                text = displayText
                textSize = 14f
                setTextColor(Color.parseColor("#333333"))
                setLineSpacing(dp(3).toFloat(), 1f)
                setPadding(0, dp(7), 0, 0)
            })
        }
    }

    private fun cleanAnnouncementText(value: String?): String? {
        return value?.trim()?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun feedPosts(
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>
    ): List<ProjectSpaceFeedPost> {
        val channelsById = space.channels
            .filter { it.isProjectSpaceFeedChannel() }
            .associateBy { it.id }
        return channelsById.values.flatMap { channel ->
            val channelMessages = messagesByChannel[channel.id].orEmpty()
            val replyCounts = projectSpaceReplyCountsByPost(channelMessages)
            channelMessages
                .filter { it.isProjectSpaceFeedPost() }
                .map { ProjectSpaceFeedPost(channel, it, replyCounts[it.id] ?: 0) }
        }.sortedByDescending { parseChatMessageCreatedAt(it.message.createdAt) ?: 0L }
            .take(MAX_FEED_POSTS)
    }

    private fun postCard(post: ProjectSpaceFeedPost): LinearLayout {
        val postText = parseProjectSpacePostText(post.message.content)
        val sender = post.message.senderName.cleanProjectSpaceDisplayName() ?: "项目成员"
        val timeText = parseChatMessageCreatedAt(post.message.createdAt)
            ?.let { formatChatTimelineLabel(it) }
            ?: "刚刚"
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(14), dp(14), dp(14), dp(12))
            background = roundedBackground("#222222", 10)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openPost(post.channel, post.message) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(dp(10), dp(6), dp(10), dp(8))
            }

            addView(postHeader(sender, post.message.senderAvatarDataUrl, timeText, projectSpaceTopicLabel(post.channel)))
            addView(TextView(activity).apply {
                text = postText.title
                textSize = 16f
                setTextColor(Color.parseColor("#D6D6D6"))
                setTypeface(typeface, Typeface.BOLD)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setPadding(0, dp(10), 0, 0)
            })
            addView(TextView(activity).apply {
                text = postText.body.ifBlank { postText.title }
                textSize = 14f
                setTextColor(Color.parseColor("#A8A8A8"))
                setLineSpacing(dp(3).toFloat(), 1f)
                maxLines = 3
                ellipsize = TextUtils.TruncateAt.END
                setPadding(0, dp(8), 0, 0)
            })
            extractProjectPostImageSource(postText.body)?.let { source ->
                addView(postImagePreview(source))
            }
            addView(postMetrics(post, postText))
        }
    }

    private fun postHeader(sender: String, avatarDataUrl: String?, timeText: String, topic: String): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(avatar(sender, avatarDataUrl), LinearLayout.LayoutParams(dp(40), dp(40)).apply {
                marginEnd = dp(10)
            })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                addView(TextView(activity).apply {
                    text = sender
                    textSize = 14f
                    setTextColor(Color.parseColor("#D6D6D6"))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = "回复于$timeText"
                    textSize = 11f
                    setTextColor(Color.parseColor("#777777"))
                    setPadding(0, dp(3), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(TextView(activity).apply {
                text = topic
                textSize = 13f
                setTextColor(Color.parseColor("#A8A8A8"))
                gravity = Gravity.CENTER
                maxLines = 1
                background = roundedBackground("#151515", 8)
                setPadding(dp(10), dp(5), dp(10), dp(5))
            })
        }
    }

    private fun avatar(sender: String, avatarDataUrl: String?): View {
        val bitmap = UserProfileStore.decodeAvatar(avatarDataUrl.cleanProjectSpaceDisplayName())
        if (bitmap != null) {
            return TextView(activity).apply {
                background = RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(20).toFloat()
                    setAntiAlias(true)
                }
            }
        }
        return TextView(activity).apply {
            text = sender.firstOrNull()?.toString() ?: "成"
            gravity = Gravity.CENTER
            includeFontPadding = false
            textSize = 16f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#101010"))
            background = roundedBackground("#D8D8D8", 20)
        }
    }

    private fun postImagePreview(source: String): ImageView {
        val image = ImageView(activity).apply {
            scaleType = ImageView.ScaleType.CENTER_CROP
            setBackgroundColor(Color.parseColor("#22262C"))
            setImageResource(android.R.drawable.ic_menu_gallery)
            tag = source
            layoutParams = LinearLayout.LayoutParams(
                dp(220),
                dp(112)
            ).apply {
                topMargin = dp(12)
            }
        }
        thread(name = "project-post-image-preview") {
            val bitmap = runCatching {
                val bytes = ChatImageDiskCache.readBytes(activity, source, MAX_IMAGE_PREVIEW_BYTES)
                BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
            }.getOrNull()
            if (bitmap != null) {
                activity.runOnUiThread {
                    if (image.tag == source) image.setImageBitmap(bitmap)
                }
            }
        }
        return image
    }

    private fun postMetrics(post: ProjectSpaceFeedPost, postText: ProjectSpacePostText): LinearLayout {
        val key = post.metricKey()
        val shareCount = metricPrefs.getInt("$key:shares", 0)
        val liked = metricPrefs.getBoolean("$key:liked", false)
        val likeCount = if (liked) 1 else 0
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(8), dp(10), dp(8), 0)
            addView(metricButton(
                iconRes = R.drawable.ic_project_post_share,
                value = shareCount.toString(),
                description = "分享帖子",
                onClick = { views ->
                    sharePost(post, postText) {
                        val nextCount = metricPrefs.getInt("$key:shares", 0) + 1
                        metricPrefs.edit().putInt("$key:shares", nextCount).apply()
                        views.value.text = nextCount.toString()
                    }
                }
            ), metricParams())
            addView(metricButton(
                iconRes = R.drawable.ic_project_post_chat,
                value = post.replyCount.coerceAtLeast(0).toString(),
                description = "查看${post.replyCount.coerceAtLeast(0)}条讨论",
                onClick = { openPost(post.channel, post.message) }
            ), metricParams())
            addView(metricButton(
                iconRes = if (liked) R.drawable.ic_project_post_like_filled else R.drawable.ic_project_post_like,
                value = likeCount.toString(),
                description = if (liked) "取消点赞" else "点赞",
                selected = liked,
                onClick = { views ->
                    val nextLiked = !metricPrefs.getBoolean("$key:liked", false)
                    metricPrefs.edit().putBoolean("$key:liked", nextLiked).apply()
                    updateMetricButton(
                        views = views,
                        iconRes = if (nextLiked) R.drawable.ic_project_post_like_filled else R.drawable.ic_project_post_like,
                        value = if (nextLiked) "1" else "0",
                        selected = nextLiked,
                        description = if (nextLiked) "取消点赞" else "点赞"
                    )
                }
            ), metricParams())
        }
    }

    private fun metricButton(
        iconRes: Int,
        value: String,
        description: String,
        selected: Boolean = false,
        onClick: (MetricButtonViews) -> Unit
    ): LinearLayout {
        val color = metricColor(selected)
        val icon = ImageView(activity).apply {
            setImageResource(iconRes)
            setColorFilter(color)
        }
        val valueText = TextView(activity).apply {
            text = value
            textSize = 13f
            includeFontPadding = false
            setTextColor(color)
            gravity = Gravity.CENTER
        }
        val views = MetricButtonViews(icon, valueText)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            isClickable = true
            foreground = selectableForeground()
            contentDescription = description
            minimumHeight = dp(32)
            setPadding(dp(8), dp(6), dp(8), dp(6))
            addView(icon, LinearLayout.LayoutParams(dp(18), dp(18)).apply {
                marginEnd = dp(5)
            })
            addView(valueText, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
            setOnClickListener { onClick(views) }
        }
    }

    private fun updateMetricButton(
        views: MetricButtonViews,
        iconRes: Int,
        value: String,
        selected: Boolean,
        description: String
    ) {
        val color = metricColor(selected)
        views.icon.setImageResource(iconRes)
        views.icon.setColorFilter(color)
        views.value.text = value
        views.value.setTextColor(color)
        (views.icon.parent as? View)?.contentDescription = description
    }

    private fun metricColor(selected: Boolean): Int {
        return Color.parseColor(if (selected) "#58BE6A" else "#A8A8A8")
    }

    private fun sharePost(
        post: ProjectSpaceFeedPost,
        postText: ProjectSpacePostText,
        onShared: () -> Unit
    ) {
        val topic = projectSpaceTopicLabel(post.channel)
        val shareText = buildString {
            append("【").append(postText.title).append("】")
            postText.body.trim().takeIf { it.isNotBlank() }?.let { body ->
                append("\n\n").append(body)
            }
            append("\n\n来自项目话题：").append(topic)
        }
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_SUBJECT, postText.title)
            putExtra(Intent.EXTRA_TEXT, shareText)
        }
        runCatching {
            activity.startActivity(Intent.createChooser(intent, "分享帖子"))
        }.onSuccess {
            onShared()
        }.onFailure { error ->
            Toast.makeText(activity, error.message ?: "无法打开系统分享", Toast.LENGTH_SHORT).show()
        }
    }

    private fun metricParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
    }

    private fun emptyState(textValue: String, showButton: Boolean): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(dp(20), dp(72), dp(20), dp(104))
            addView(TextView(activity).apply {
                text = textValue
                textSize = 15f
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#A8A8A8"))
            })
            if (showButton) {
                addView(TextView(activity).apply {
                    text = "+"
                    textSize = 34f
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setTextColor(Color.parseColor("#D6D6D6"))
                    background = roundedBackground("#30333A", 24)
                    isClickable = true
                    foreground = selectableForeground()
                    setOnClickListener { openPostComposer() }
                    contentDescription = "发布帖子"
                }, LinearLayout.LayoutParams(dp(48), dp(48)).apply {
                    topMargin = dp(28)
                })
            }
        }
    }

    private fun roundedBackground(colorHex: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun roundedBackground(
        colorHex: String,
        topStartDp: Int,
        topEndDp: Int,
        bottomEndDp: Int,
        bottomStartDp: Int
    ): GradientDrawable {
        val topStart = dp(topStartDp).toFloat()
        val topEnd = dp(topEndDp).toFloat()
        val bottomEnd = dp(bottomEndDp).toFloat()
        val bottomStart = dp(bottomStartDp).toFloat()
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadii = floatArrayOf(
                topStart, topStart,
                topEnd, topEnd,
                bottomEnd, bottomEnd,
                bottomStart, bottomStart
            )
        }
    }

    private companion object {
        const val MAX_FEED_POSTS = 40
        const val MAX_IMAGE_PREVIEW_BYTES = 5 * 1024 * 1024
        const val POST_METRIC_PREFS = "project_post_metrics"
    }

    private data class MetricButtonViews(
        val icon: ImageView,
        val value: TextView
    )
}

internal data class ProjectSpaceFeedPost(
    val channel: ProjectChannel,
    val message: ProjectChannelMessage,
    val replyCount: Int
) {
    fun metricKey(): String {
        val messageId = message.id.trim()
        if (messageId.isNotBlank()) return "post:$messageId"
        return "post:${channel.id}:${message.createdAt}:${message.content.hashCode()}"
    }
}

private fun extractProjectPostImageSource(text: String): String? {
    val markdown = Regex("""!\[[^]]*]\(([^)]+)\)""").find(text)
        ?.groupValues
        ?.getOrNull(1)
        ?.trim()
    if (!markdown.isNullOrBlank()) return markdown
    return Regex(
        """https?://\S+\.(?:png|jpe?g|webp|gif)(?:\?\S*)?""",
        RegexOption.IGNORE_CASE
    ).find(text)?.value
}
