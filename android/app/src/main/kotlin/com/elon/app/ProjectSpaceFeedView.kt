package com.elon.app

import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import kotlin.concurrent.thread

internal class ProjectSpaceFeedView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val openChannel: (ProjectChannel) -> Unit,
    private val openPostComposer: () -> Unit,
    private val openProjectDocuments: () -> Unit
) {
    fun render(
        container: LinearLayout,
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>,
        loading: Boolean
    ) {
        container.addView(announcementBlock(space, messagesByChannel))

        val frame = FrameLayout(activity).apply {
            setPadding(0, dp(4), 0, dp(28))
        }
        val feedColumn = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, 0)
        }
        frame.addView(feedColumn, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ))

        val posts = feedPosts(space, messagesByChannel)
        when {
            posts.isNotEmpty() -> posts.forEach { feedColumn.addView(postCard(it)) }
            loading -> feedColumn.addView(statusRow("正在加载帖子...", "#A6AFBD"))
            else -> feedColumn.addView(statusRow("还没有帖子，点击右侧 + 发布第一条。", "#6F7785"))
        }

        frame.addView(floatingActions(), FrameLayout.LayoutParams(
            dp(48),
            FrameLayout.LayoutParams.WRAP_CONTENT,
            Gravity.END or Gravity.CENTER_VERTICAL
        ).apply {
            marginEnd = dp(6)
        })
        container.addView(frame)
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
        val textValue = latest?.content?.trim()
            ?: announcement?.lastMessage?.trim()
            ?: "不得发布与主题内容不相关的帖子。"

        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(14), dp(20), dp(16))
            background = roundedBackground("#181B20", 14)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(dp(10), dp(14), dp(10), dp(8))
            }
            addView(TextView(activity).apply {
                text = "公告"
                textSize = 15f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F2F5FA"))
            })
            addView(TextView(activity).apply {
                text = parseProjectSpacePostText(textValue).detailText
                textSize = 14f
                setTextColor(Color.parseColor("#A6AFBD"))
                setLineSpacing(dp(3).toFloat(), 1f)
                setPadding(0, dp(7), 0, 0)
            })
        }
    }

    private fun feedPosts(
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>
    ): List<ProjectSpaceFeedPost> {
        val channelsById = space.channels
            .filter { it.isProjectSpaceFeedChannel() }
            .associateBy { it.id }
        return channelsById.values.flatMap { channel ->
            messagesByChannel[channel.id].orEmpty().map { ProjectSpaceFeedPost(channel, it) }
        }.sortedByDescending { parseChatMessageCreatedAt(it.message.createdAt) ?: 0L }
            .take(MAX_FEED_POSTS)
    }

    private fun postCard(post: ProjectSpaceFeedPost): LinearLayout {
        val postText = parseProjectSpacePostText(post.message.content)
        val sender = post.message.senderName?.takeIf { it.isNotBlank() } ?: "项目成员"
        val timeText = parseChatMessageCreatedAt(post.message.createdAt)
            ?.let { formatChatTimelineLabel(it) }
            ?: "刚刚"
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(14), dp(14), dp(14), dp(12))
            background = roundedBackground("#181B20", 10)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openChannel(post.channel) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(dp(10), dp(6), dp(10), dp(8))
            }

            addView(postHeader(sender, timeText, projectSpaceTopicLabel(post.channel)))
            addView(TextView(activity).apply {
                text = postText.title
                textSize = 16f
                setTextColor(Color.parseColor("#F2F5FA"))
                setTypeface(typeface, Typeface.BOLD)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setPadding(0, dp(10), 0, 0)
            })
            addView(TextView(activity).apply {
                text = postText.body.ifBlank { postText.title }
                textSize = 14f
                setTextColor(Color.parseColor("#A6AFBD"))
                setLineSpacing(dp(3).toFloat(), 1f)
                maxLines = 3
                ellipsize = TextUtils.TruncateAt.END
                setPadding(0, dp(8), 0, 0)
            })
            extractProjectPostImageSource(postText.body)?.let { source ->
                addView(postImagePreview(source))
            }
            addView(postMetrics(post))
        }
    }

    private fun postHeader(sender: String, timeText: String, topic: String): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(avatar(sender), LinearLayout.LayoutParams(dp(40), dp(40)).apply {
                marginEnd = dp(10)
            })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                addView(TextView(activity).apply {
                    text = sender
                    textSize = 14f
                    setTextColor(Color.parseColor("#F2F5FA"))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = "回复于$timeText"
                    textSize = 11f
                    setTextColor(Color.parseColor("#6F7785"))
                    setPadding(0, dp(3), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(TextView(activity).apply {
                text = topic
                textSize = 13f
                setTextColor(Color.parseColor("#A6AFBD"))
                gravity = Gravity.CENTER
                maxLines = 1
                background = roundedBackground("#0F1217", 8)
                setPadding(dp(10), dp(5), dp(10), dp(5))
            })
        }
    }

    private fun avatar(sender: String): TextView {
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

    private fun postMetrics(post: ProjectSpaceFeedPost): LinearLayout {
        val countText = if (post.channel.unreadCount > 0) post.channel.unreadCount.toString() else "1"
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(20), dp(12), dp(10), 0)
            addView(metric("♡", "1"), metricParams())
            addView(metric("◌", countText), metricParams())
            addView(metric("♧", "16"), metricParams())
        }
    }

    private fun metric(icon: String, value: String): TextView {
        return TextView(activity).apply {
            text = "$icon $value"
            textSize = 12f
            setTextColor(Color.parseColor("#A6AFBD"))
            gravity = Gravity.CENTER
        }
    }

    private fun metricParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
    }

    private fun floatingActions(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            addView(floatingButton(
                iconRes = R.drawable.ic_popup_history,
                contentDescription = "项目文档",
                onClick = openProjectDocuments
            ))
            addView(floatingButton(
                iconRes = R.drawable.ic_add_circle_simple,
                contentDescription = "发布帖子",
                onClick = openPostComposer
            ), LinearLayout.LayoutParams(dp(48), dp(48)).apply {
                topMargin = dp(10)
            })
        }
    }

    private fun floatingButton(
        iconRes: Int,
        contentDescription: String,
        onClick: () -> Unit
    ): ImageButton {
        return ImageButton(activity).apply {
            setImageResource(iconRes)
            setColorFilter(Color.parseColor("#F2F5FA"))
            background = roundedBackground("#7C8EA8", 24)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(12), dp(12), dp(12), dp(12))
            this.contentDescription = contentDescription
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
        }
    }

    private fun statusRow(textValue: String, colorHex: String): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 14f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(colorHex))
            setPadding(dp(20), dp(44), dp(72), dp(44))
        }
    }

    private fun roundedBackground(colorHex: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private companion object {
        const val MAX_FEED_POSTS = 40
        const val MAX_IMAGE_PREVIEW_BYTES = 5 * 1024 * 1024
    }
}

internal data class ProjectSpaceFeedPost(
    val channel: ProjectChannel,
    val message: ProjectChannelMessage
)

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
