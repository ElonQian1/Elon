package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

class PersonalQrCodeActivity : AppCompatActivity() {
    private var qrBitmap: Bitmap? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(buildContent())
    }

    override fun onDestroy() {
        qrBitmap?.recycle()
        qrBitmap = null
        super.onDestroy()
    }

    private fun buildContent(): View {
        val profile = UserProfileStore.load(this)
        val userId = AuthManager.effectiveUserId(this)
        val qrSize = (resources.displayMetrics.widthPixels - dp(112))
            .coerceIn(dp(220), dp(320))
        qrBitmap = QrCodeBitmap.create(UserProfileStore.personalQrPayload(this), qrSize)

        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor("#0B1118"))
            addView(topBar())
            addView(ScrollView(this@PersonalQrCodeActivity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    0,
                    1f
                )
                addView(LinearLayout(this@PersonalQrCodeActivity).apply {
                    orientation = LinearLayout.VERTICAL
                    gravity = Gravity.CENTER_HORIZONTAL
                    setPadding(dp(22), dp(26), dp(22), dp(34))
                    addView(qrCard(profile, userId, qrSize))
                    addView(copyButton(userId), LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        dp(48)
                    ).apply {
                        topMargin = dp(18)
                    })
                })
            })
        }
    }

    private fun topBar(): LinearLayout {
        return LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(56)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(6), 0, dp(18), 0)
            setBackgroundColor(Color.parseColor("#0B1118"))
            addView(TextView(this@PersonalQrCodeActivity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(56), LinearLayout.LayoutParams.MATCH_PARENT)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "‹"
                setTextColor(Color.parseColor("#F8F7F4"))
                textSize = 34f
                setOnClickListener { finish() }
            })
            addView(TextView(this@PersonalQrCodeActivity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "我的二维码"
                setTextColor(Color.parseColor("#F8F7F4"))
                textSize = 20f
            })
            addView(View(this@PersonalQrCodeActivity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(56), 1)
            })
        }
    }

    private fun qrCard(profile: UserProfile, userId: String, qrSize: Int): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            background = roundedRect("#F8F7F4", 12)
            setPadding(dp(20), dp(20), dp(20), dp(18))
            addView(identityHeader(profile, userId))
            addView(ImageView(this@PersonalQrCodeActivity).apply {
                layoutParams = LinearLayout.LayoutParams(qrSize, qrSize).apply {
                    gravity = Gravity.CENTER_HORIZONTAL
                    topMargin = dp(20)
                }
                contentDescription = "个人二维码"
                scaleType = ImageView.ScaleType.FIT_CENTER
                setImageBitmap(qrBitmap)
            })
            addView(TextView(this@PersonalQrCodeActivity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(16)
                }
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "扫码识别我的一龙账号"
                setTextColor(Color.parseColor("#20262E"))
                textSize = 14f
            })
        }
    }

    private fun identityHeader(profile: UserProfile, userId: String): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(UserProfileViews.createAvatarView(
                this@PersonalQrCodeActivity,
                profile,
                56,
                20f
            ).apply {
                layoutParams = LinearLayout.LayoutParams(dp(56), dp(56))
            })
            addView(LinearLayout(this@PersonalQrCodeActivity).apply {
                orientation = LinearLayout.VERTICAL
                addView(TextView(this@PersonalQrCodeActivity).apply {
                    includeFontPadding = false
                    maxLines = 1
                    text = profile.displayName
                    setTextColor(Color.parseColor("#0B1118"))
                    textSize = 20f
                    setTypeface(typeface, Typeface.BOLD)
                })
                addView(TextView(this@PersonalQrCodeActivity).apply {
                    includeFontPadding = false
                    maxLines = 1
                    text = "账号：${profile.wechatId}"
                    setTextColor(Color.parseColor("#20262E"))
                    textSize = 13f
                    setPadding(0, dp(8), 0, 0)
                })
                addView(TextView(this@PersonalQrCodeActivity).apply {
                    includeFontPadding = false
                    maxLines = 1
                    text = "ID：$userId"
                    setTextColor(Color.parseColor("#80BEBEBA"))
                    textSize = 12f
                    setPadding(0, dp(6), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(14)
            })
        }
    }

    private fun copyButton(userId: String): TextView {
        return TextView(this).apply {
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "复制账号 ID"
            textSize = 15f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#F8F7F4"))
            background = roundedRect("#20262E", 8, "#667B8793")
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            setOnClickListener {
                val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(ClipData.newPlainText("一龙账号 ID", userId))
                Toast.makeText(this@PersonalQrCodeActivity, "账号 ID 已复制", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun selectableForeground(): Drawable? {
        val out = TypedValue()
        theme.resolveAttribute(android.R.attr.selectableItemBackground, out, true)
        return ContextCompat.getDrawable(this, out.resourceId)
    }

    private fun roundedRect(fillColor: String, radiusDp: Int, strokeColor: String? = null): GradientDrawable =
        GradientDrawable().apply {
            setColor(Color.parseColor(fillColor))
            cornerRadius = dp(radiusDp).toFloat()
            strokeColor?.let { setStroke(dp(1), Color.parseColor(it)) }
        }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density).toInt()
}
