package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import kotlin.concurrent.thread
import kotlin.math.roundToInt

internal object UserProfileViews {
    private const val SUMMARY_TAG = "profile_summary_card"

    fun renderSummary(
        context: Context,
        binding: ActivityMainBinding,
        http: OkHttpClient,
        serverUrl: String,
        onClick: () -> Unit
    ) {
        val parent = binding.userInfoText.parent as? ViewGroup ?: return
        for (index in parent.childCount - 1 downTo 0) {
            if (parent.getChildAt(index).tag == SUMMARY_TAG) parent.removeViewAt(index)
        }
        val anchorIndex = parent.indexOfChild(binding.userInfoText).takeIf { it >= 0 } ?: 0
        binding.userInfoText.visibility = View.GONE
        val summary = createSummaryCard(context, UserProfileStore.load(context), onClick)
        summary.root.tag = SUMMARY_TAG
        parent.addView(summary.root, anchorIndex)
        refreshProgression(context, http, serverUrl, summary)
    }

    fun createAvatarView(
        context: Context,
        profile: UserProfile,
        sizeDp: Int,
        textSizeSp: Float
    ): View {
        val size = context.dp(sizeDp)
        val bitmap = UserProfileStore.decodeAvatar(profile.avatarDataUrl)
        if (bitmap != null) {
            return ImageView(context).apply {
                layoutParams = ViewGroup.LayoutParams(size, size)
                background = roundedRect(context.elonColor(R.color.elon_surface_header), size / 2)
                clipToOutline = true
                contentDescription = "头像"
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageBitmap(bitmap)
            }
        }
        return TextView(context).apply {
            layoutParams = ViewGroup.LayoutParams(size, size)
            background = roundedRect(context.elonColor(R.color.elon_button_primary_bg), size / 2)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = UserProfileStore.avatarInitial(profile.displayName)
            setTextColor(context.elonColor(R.color.elon_button_primary_text))
            textSize = textSizeSp
            setTypeface(typeface, Typeface.BOLD)
        }
    }

    fun row(
        context: Context,
        title: String,
        value: String? = null,
        trailing: View? = null,
        onClick: (() -> Unit)? = null
    ): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                context.dp(66)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(context.dp(22), 0, context.dp(22), 0)
            setBackgroundColor(context.elonColor(R.color.elon_surface_card))
            if (onClick != null) {
                isClickable = true
                foreground = selectableForeground(context)
                setOnClickListener { onClick() }
            }
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                includeFontPadding = false
                text = title
                setTextColor(context.elonColor(R.color.elon_text_primary))
                textSize = 17f
            })
            if (trailing != null) {
                addView(trailing)
            } else {
                addView(TextView(context).apply {
                    includeFontPadding = false
                    text = value.orEmpty()
                    setTextColor(context.elonColor(R.color.elon_text_tertiary))
                    textSize = 16f
                })
            }
            addView(arrow(context))
        }
    }

    fun divider(context: Context): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                marginStart = context.dp(22)
            }
            setBackgroundColor(context.elonColor(R.color.elon_divider_card))
        }
    }

    fun spacer(context: Context, heightDp: Int): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                context.dp(heightDp)
            )
            setBackgroundColor(context.elonColor(R.color.elon_bg_app))
        }
    }

    private fun createSummaryCard(
        context: Context,
        profile: UserProfile,
        onClick: () -> Unit
    ): SummaryViews {
        val textBlock = profileTextBlock(context, profile)
        val root = LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                context.dp(84)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(context.dp(16), context.dp(12), context.dp(10), context.dp(12))
            setBackgroundResource(R.drawable.profile_panel_identity)
            isClickable = true
            foreground = selectableForeground(context)
            setOnClickListener { onClick() }

            addView(createAvatarView(context, profile, 38, 16f).apply {
                layoutParams = LinearLayout.LayoutParams(context.dp(38), context.dp(38))
            })
            addView(textBlock.root)
            addView(qrThumbnail(context))
        }
        return SummaryViews(root, textBlock.level, textBlock.percent, textBlock.progress)
    }

    private fun profileTextBlock(context: Context, profile: UserProfile): ProfileTextBlock {
        val level = TextView(context).apply {
            includeFontPadding = false
            text = if (AuthManager.isLoggedIn(context)) "Lv.--" else "Lv.0"
            setTextColor(context.elonColor(R.color.elon_text_primary))
            textSize = 11f
        }
        val percent = TextView(context).apply {
            includeFontPadding = false
            text = if (AuthManager.isLoggedIn(context)) "同步中" else "0%"
            setTextColor(context.elonColor(R.color.elon_text_primary))
            textSize = 11f
        }
        val progress = ProfileLevelProgressView(context).apply {
            layoutParams = LinearLayout.LayoutParams(0, context.dp(5), 1f).apply {
                marginStart = context.dp(5)
                marginEnd = context.dp(5)
            }
        }
        val root = LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = context.dp(14)
                marginEnd = context.dp(12)
            }
            orientation = LinearLayout.VERTICAL
            addView(TextView(context).apply {
                includeFontPadding = false
                maxLines = 1
                text = profile.displayName
                setTextColor(context.elonColor(R.color.elon_text_primary))
                textSize = 16f
            })
            addView(TextView(context).apply {
                includeFontPadding = false
                maxLines = 1
                text = "账号：${profile.wechatId}"
                setTextColor(context.elonColor(R.color.elon_text_tertiary))
                textSize = 11f
                setPadding(0, context.dp(5), 0, 0)
            })
            addView(LinearLayout(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = context.dp(5)
                }
                gravity = Gravity.CENTER_VERTICAL
                orientation = LinearLayout.HORIZONTAL
                addView(level)
                addView(progress)
                addView(percent)
            })
        }
        return ProfileTextBlock(root, level, percent, progress)
    }

    private fun qrThumbnail(context: Context): ImageView {
        val size = context.dp(34)
        val bitmap = QrCodeBitmap.create(
            UserProfileStore.personalQrPayload(context),
            size,
            foreground = Color.parseColor("#101010"),
            background = Color.TRANSPARENT
        )
        return ImageView(context).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            background = roundedRect(context.elonColor(R.color.elon_text_primary), context.dp(4))
            contentDescription = "我的二维码"
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(context.dp(3), context.dp(3), context.dp(3), context.dp(3))
            setImageBitmap(bitmap)
        }
    }

    private fun refreshProgression(
        context: Context,
        http: OkHttpClient,
        serverUrl: String,
        summary: SummaryViews
    ) {
        if (!AuthManager.isLoggedIn(context)) return
        val appContext = context.applicationContext
        thread(name = "profile-progression") {
            val state = runCatching {
                val request = AuthManager.applyAuth(
                    appContext,
                    Request.Builder()
                        .url("${serverUrl.trimEnd('/')}/api/me/progression")
                        .get()
                ).build()
                http.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) error("等级同步失败")
                    parseProgression(JSONObject(response.body?.string().orEmpty()))
                }
            }.getOrNull()
            summary.root.post {
                if (summary.root.parent == null || state == null) return@post
                summary.level.text = "Lv.${state.level}"
                summary.percent.text = "${state.percent}%"
                summary.progress.setSegments(state.segments)
            }
        }
    }

    private fun parseProgression(json: JSONObject): ProgressionState {
        val level = json.optInt("level", 1).coerceAtLeast(1)
        val percent = (json.optDouble("level_progress_ratio", 0.0).coerceIn(0.0, 1.0) * 100)
            .roundToInt()
            .coerceIn(0, 100)
        val segments = floatArrayOf(
            json.optRatio("own_codex_progress_ratio", "consumed_progress_ratio"),
            json.optRatio("platform_progress_ratio"),
            json.optRatio("shared_codex_progress_ratio"),
            json.optRatio("provided_progress_ratio")
        )
        return ProgressionState(level, percent, segments)
    }

    private fun JSONObject.optRatio(primary: String, fallback: String? = null): Float {
        val key = if (has(primary)) primary else fallback
        return key?.let { optDouble(it, 0.0).coerceIn(0.0, 1.0).toFloat() } ?: 0f
    }

    private data class SummaryViews(
        val root: LinearLayout,
        val level: TextView,
        val percent: TextView,
        val progress: ProfileLevelProgressView
    )

    private data class ProfileTextBlock(
        val root: LinearLayout,
        val level: TextView,
        val percent: TextView,
        val progress: ProfileLevelProgressView
    )

    private data class ProgressionState(
        val level: Int,
        val percent: Int,
        val segments: FloatArray
    )

    private fun arrow(context: Context): TextView {
        return TextView(context).apply {
            layoutParams = LinearLayout.LayoutParams(context.dp(22), ViewGroup.LayoutParams.WRAP_CONTENT)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "›"
            setTextColor(context.elonColor(R.color.elon_text_tertiary))
            textSize = 32f
        }
    }

    private fun selectableForeground(context: Context): Drawable? {
        val out = TypedValue()
        context.theme.resolveAttribute(android.R.attr.selectableItemBackground, out, true)
        return ContextCompat.getDrawable(context, out.resourceId)
    }

    private fun roundedRect(color: Int, radius: Int): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = radius.toFloat()
            setColor(color)
        }
}

private class ProfileLevelProgressView(context: Context) : View(context) {
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val segmentColors = intArrayOf(
        context.elonColor(R.color.elon_status_success),
        context.elonColor(R.color.elon_button_primary_bg),
        context.elonColor(R.color.elon_status_info),
        context.elonColor(R.color.elon_status_project)
    )
    private var segments = floatArrayOf(0f, 0f, 0f, 0f)

    fun setSegments(value: FloatArray) {
        segments = value.copyOf(4)
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val radius = height / 2f
        paint.color = context.elonColor(R.color.elon_surface_soft)
        canvas.drawRoundRect(0f, 0f, width.toFloat(), height.toFloat(), radius, radius, paint)
        var left = 0f
        val total = segments.sum().coerceAtMost(1f)
        segments.forEachIndexed { index, value ->
            val right = (left + width * value.coerceIn(0f, 1f)).coerceAtMost(width.toFloat())
            if (right > left) {
                paint.color = segmentColors[index]
                canvas.drawRect(left, 0f, right, height.toFloat(), paint)
            }
            left = right
        }
        if (total <= 0f) invalidateOutline()
    }
}

private fun Context.dp(value: Int): Int =
    (value * resources.displayMetrics.density).toInt()
