package com.elon.app.grid.share

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.util.LruCache
import android.view.Gravity
import android.view.View
import android.widget.*
import androidx.appcompat.app.AlertDialog
import androidx.core.graphics.ColorUtils
import com.elon.app.R
import java.util.Locale

/** Scoped to grid sharing; the existing chat theme and navigation remain the host. */
internal class GridShareStyle(val context: Context) {
    fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
    fun color(id: Int) = context.getColor(id)
    val surface get() = color(R.color.mobile_surface_container)
    val muted get() = color(R.color.mobile_on_surface_variant)
    val primary get() = color(R.color.mobile_primary)
    val ink get() = color(R.color.mobile_on_surface)
    fun tone(value: String?): Int = when (GridSharePresentation.number(value)?.signum()) {
        1 -> color(R.color.mobile_financial_positive)
        -1 -> color(R.color.mobile_financial_negative)
        else -> ink
    }
    fun direction(value: String?): Int = when (value) { "LONG" -> color(R.color.mobile_financial_positive); "SHORT" -> color(R.color.mobile_financial_negative); else -> primary }
    fun panel(fill: Int = surface, radius: Int = 20, border: Int? = null) = GradientDrawable().apply {
        setColor(fill); cornerRadius = dp(radius).toFloat(); border?.let { setStroke(dp(1), it) }
    }
    fun column(padding: Int = 16) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL; setPadding(dp(padding), dp(padding), dp(padding), dp(padding))
    }
    fun row() = LinearLayout(context).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
    fun text(value: String, size: Float = 14f, quiet: Boolean = false) = TextView(context).apply {
        text = value; textSize = size; setTextColor(if (quiet) muted else ink)
        setPadding(0, dp(4), 0, dp(4)); setLineSpacing(dp(2).toFloat(), 1f)
    }
    fun pill(value: String, tint: Int) = text(value, 12f).apply {
        setTextColor(tint); typeface = Typeface.DEFAULT_BOLD
        setPadding(dp(9), dp(4), dp(9), dp(4)); background = panel(ColorUtils.blendARGB(surface, tint, .12f), 8)
    }
    fun button(label: String, action: () -> Unit) = androidx.appcompat.widget.AppCompatButton(context).apply {
        text = label; isAllCaps = false; textSize = 14f; minHeight = dp(48); minimumHeight = dp(48)
        setTextColor(primary); backgroundTintList = ColorStateList.valueOf(color(R.color.mobile_primary_container))
        setOnClickListener { action() }
    }
    fun divider() = View(context).apply { setBackgroundColor(color(R.color.mobile_outline_variant)); layoutParams = LinearLayout.LayoutParams(-1, dp(1)) }
    fun spaced(view: View, top: Int = 8) = view.apply {
        layoutParams = LinearLayout.LayoutParams(-1, layoutParams?.height ?: -2).apply { topMargin = dp(top) }
    }
    fun logo(symbol: String, size: Int = 40): View {
        val token = GridSharePresentation.token(symbol)
        val bitmap = bitmaps.get(token) ?: runCatching {
            context.assets.open("grid-token-icons/${token.lowercase(Locale.ROOT)}.png").use { BitmapFactory.decodeStream(it) }
        }.getOrNull()?.also { bitmaps.put(token, it) }
        return FrameLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(dp(size), dp(size))
            background = panel(color(R.color.mobile_primary_container), size / 2); clipToOutline = true
            addView(if (bitmap != null) ImageView(context).apply { setImageBitmap(bitmap); scaleType = ImageView.ScaleType.FIT_CENTER }
            else text(token.take(2), 16f).apply { gravity = Gravity.CENTER; setTextColor(color(R.color.mobile_on_primary_container)); typeface = Typeface.DEFAULT_BOLD }, FrameLayout.LayoutParams(-1, -1))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
    }
    fun range(position: Float?): View = object : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
        override fun onDraw(canvas: Canvas) {
            val inset = dp(4).toFloat(); val y = height / 2f; val radius = dp(4).toFloat()
            paint.color = color(R.color.mobile_outline_variant)
            canvas.drawRoundRect(inset, y - radius, width - inset, y + radius, radius, radius, paint)
            if (position != null) {
                val x = inset + (width - 2 * inset) * position
                paint.color = primary; canvas.drawRoundRect(inset, y - radius, x, y + radius, radius, radius, paint)
                canvas.drawCircle(x, y, dp(6).toFloat(), paint)
                paint.color = surface; canvas.drawCircle(x, y, dp(2).toFloat(), paint)
            }
        }
    }.apply { layoutParams = LinearLayout.LayoutParams(-1, dp(24)); importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO }
    fun dialogTitle(title: String) = text(title, 20f).apply { typeface = Typeface.DEFAULT_BOLD; setPadding(dp(20), dp(20), dp(20), dp(8)); setBackgroundColor(surface) }
    fun styleDialog(dialog: AlertDialog) {
        dialog.window?.apply {
            setBackgroundDrawable(panel(surface, 24))
            setLayout(minOf(dp(520), context.resources.displayMetrics.widthPixels - dp(24)), -2)
        }
        listOf(AlertDialog.BUTTON_POSITIVE, AlertDialog.BUTTON_NEGATIVE, AlertDialog.BUTTON_NEUTRAL).forEach { id ->
            dialog.getButton(id)?.apply { setTextColor(primary); minHeight = dp(48); isAllCaps = false }
        }
    }
    companion object { private val bitmaps = LruCache<String, Bitmap>(48) }
}
