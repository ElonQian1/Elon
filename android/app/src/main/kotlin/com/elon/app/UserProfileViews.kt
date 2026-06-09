package com.elon.app

import android.content.Context
import android.graphics.Color
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

internal object UserProfileViews {
    private const val SUMMARY_TAG = "profile_summary_card"

    fun renderSummary(context: Context, binding: ActivityMainBinding, onClick: () -> Unit) {
        val parent = binding.userInfoText.parent as? ViewGroup ?: return
        for (index in parent.childCount - 1 downTo 0) {
            if (parent.getChildAt(index).tag == SUMMARY_TAG) parent.removeViewAt(index)
        }
        val anchorIndex = parent.indexOfChild(binding.userInfoText).takeIf { it >= 0 } ?: 0
        binding.userInfoText.visibility = View.GONE
        val card = createSummaryCard(context, UserProfileStore.load(context), onClick)
        card.tag = SUMMARY_TAG
        parent.addView(card, anchorIndex)
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
                background = roundedRect(Color.parseColor("#283140"), context.dp(10))
                clipToOutline = true
                contentDescription = "头像"
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageBitmap(bitmap)
            }
        }
        return TextView(context).apply {
            layoutParams = ViewGroup.LayoutParams(size, size)
            background = roundedRect(Color.parseColor("#283140"), context.dp(10))
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = UserProfileStore.avatarInitial(profile.displayName)
            setTextColor(Color.parseColor("#F2F5FA"))
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
            setBackgroundColor(Color.parseColor("#191919"))
            if (onClick != null) {
                isClickable = true
                foreground = selectableForeground(context)
                setOnClickListener { onClick() }
            }
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#F2F5FA"))
                textSize = 17f
            })
            if (trailing != null) {
                addView(trailing)
            } else {
                addView(TextView(context).apply {
                    includeFontPadding = false
                    text = value.orEmpty()
                    setTextColor(Color.parseColor("#6F7785"))
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
            setBackgroundColor(Color.parseColor("#181B20"))
        }
    }

    fun spacer(context: Context, heightDp: Int): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                context.dp(heightDp)
            )
            setBackgroundColor(Color.parseColor("#101010"))
        }
    }

    private fun createSummaryCard(context: Context, profile: UserProfile, onClick: () -> Unit): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                context.dp(156)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(context.dp(22), context.dp(18), context.dp(18), context.dp(18))
            setBackgroundColor(Color.parseColor("#0F1217"))
            isClickable = true
            foreground = selectableForeground(context)
            setOnClickListener { onClick() }

            addView(createAvatarView(context, profile, 64, 24f).apply {
                layoutParams = LinearLayout.LayoutParams(context.dp(64), context.dp(64))
            })
            addView(profileTextBlock(context, profile))
            addView(qrThumbnail(context))
            addView(arrow(context))
        }
    }

    private fun profileTextBlock(context: Context, profile: UserProfile): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = context.dp(18)
                marginEnd = context.dp(8)
            }
            orientation = LinearLayout.VERTICAL
            addView(TextView(context).apply {
                includeFontPadding = false
                maxLines = 1
                text = profile.displayName
                setTextColor(Color.parseColor("#F2F5FA"))
                textSize = 24f
                setTypeface(typeface, Typeface.BOLD)
            })
            addView(TextView(context).apply {
                includeFontPadding = false
                maxLines = 1
                text = "账号：${profile.wechatId}"
                setTextColor(Color.parseColor("#6F7785"))
                textSize = 15f
                setPadding(0, context.dp(12), 0, 0)
            })
            addView(TextView(context).apply {
                includeFontPadding = false
                maxLines = 1
                text = profile.signature
                setTextColor(Color.parseColor("#A6AFBD"))
                textSize = 13f
                setPadding(0, context.dp(14), 0, 0)
            })
        }
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
            background = roundedRect(Color.parseColor("#F2F5FA"), context.dp(4))
            contentDescription = "我的二维码"
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(context.dp(3), context.dp(3), context.dp(3), context.dp(3))
            setImageBitmap(bitmap)
        }
    }

    private fun arrow(context: Context): TextView {
        return TextView(context).apply {
            layoutParams = LinearLayout.LayoutParams(context.dp(22), ViewGroup.LayoutParams.WRAP_CONTENT)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "›"
            setTextColor(Color.parseColor("#6F7785"))
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

private fun Context.dp(value: Int): Int =
    (value * resources.displayMetrics.density).toInt()
