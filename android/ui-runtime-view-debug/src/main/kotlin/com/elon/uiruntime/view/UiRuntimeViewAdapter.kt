package com.elon.uiruntime.view

import android.os.Build
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import com.google.gson.JsonElement
import com.google.gson.JsonPrimitive
import java.lang.reflect.Method
import java.util.Locale

internal object UiRuntimeViewAdapter {
    data class ApplyResult(
        val beforeValues: Map<String, LivePropertyValue>,
        val effectiveValues: Map<String, LivePropertyValue>,
        val measuredGeometry: Map<String, Double>,
    )

    fun nodeSnapshot(
        view: View,
        runtimeNodeId: String,
        definitionId: String,
        instanceKey: String?,
        parentRuntimeNodeId: String?,
        screenId: String,
        resourceId: String?,
    ): LiveUiNode {
        val density = view.resources.displayMetrics.density
        val fontScale = view.resources.configuration.fontScale
        val location = IntArray(2)
        view.getLocationOnScreen(location)
        val rect = LiveRect(
            left = location[0],
            top = location[1],
            right = location[0] + view.width,
            bottom = location[1] + view.height,
            width = view.width,
            height = view.height,
        )
        val properties = linkedMapOf<String, LivePropertySnapshot>()
        addProperty(properties, "width", dp(view, view.width), dp(view, view.width), resourceId)
        addProperty(properties, "height", dp(view, view.height), dp(view, view.height), resourceId)
        addProperty(properties, "padding.start", dp(view, view.paddingStart), null, resourceId)
        addProperty(properties, "padding.top", dp(view, view.paddingTop), null, resourceId)
        addProperty(properties, "padding.end", dp(view, view.paddingEnd), null, resourceId)
        addProperty(properties, "padding.bottom", dp(view, view.paddingBottom), null, resourceId)
        addProperty(properties, "opacity", floatValue(view.alpha), null, resourceId)
        addProperty(properties, "visibility", enumValue(visibilityName(view.visibility)), null, resourceId)
        addProperty(properties, "translationX", dp(view, view.translationX.toInt()), null, resourceId)
        addProperty(properties, "translationY", dp(view, view.translationY.toInt()), null, resourceId)
        addProperty(properties, "scaleX", floatValue(view.scaleX), null, resourceId)
        addProperty(properties, "scaleY", floatValue(view.scaleY), null, resourceId)
        marginProperties(view, properties, resourceId)
        styleProperties(view, properties, resourceId)
        val capabilities = linkedMapOf(
            "resizeWidth" to true,
            "resizeHeight" to true,
            "padding" to true,
            "margin" to (view.layoutParams is ViewGroup.MarginLayoutParams),
            "opacity" to true,
            "visibility" to true,
            "visualTranslatePreview" to true,
            "freeTranslate" to false,
            "text" to (view is TextView),
            "textSize" to (view is TextView),
            "backgroundColor" to true,
            "contentColor" to (view is TextView),
            "cornerRadius" to supportsCornerRadius(view),
        )
        return LiveUiNode(
            runtimeNodeId = runtimeNodeId,
            definitionId = definitionId,
            instanceKey = instanceKey,
            parentRuntimeNodeId = parentRuntimeNodeId,
            screenId = screenId,
            kind = UiRuntimeRegistry.kind(view),
            text = (view as? TextView)?.text?.toString()?.take(2_000),
            resourceId = resourceId,
            className = view.javaClass.name,
            geometry = LiveGeometry(
                boundsInDisplayPx = rect,
                density = density,
                fontScale = fontScale,
                rotation = view.display?.rotation ?: 0,
                visible = view.visibility == View.VISIBLE && view.isShown,
            ),
            properties = properties,
            capabilities = capabilities,
        )
    }

    fun apply(views: List<View>, operations: List<LivePatchOperation>): ApplyResult {
        require(views.isNotEmpty()) { "找不到目标 View，页面可能已经变化" }
        val before = linkedMapOf<String, LivePropertyValue>()
        val effective = linkedMapOf<String, LivePropertyValue>()
        for (operation in operations) {
            val first = views.first()
            before[operation.property] = read(first, operation.property)
            for (view in views) applyOperation(view, operation)
            effective[operation.property] = read(first, operation.property)
        }
        views.forEach {
            it.requestLayout()
            it.invalidate()
        }
        val first = views.first()
        return ApplyResult(
            beforeValues = before,
            effectiveValues = effective,
            measuredGeometry = mapOf(
                "widthDp" to pxToDp(first, first.width.toFloat()),
                "heightDp" to pxToDp(first, first.height.toFloat()),
            ),
        )
    }

    private fun applyOperation(view: View, operation: LivePatchOperation) {
        val dp = { value: Double -> dpToPx(view, value).toInt() }
        when (operation.property) {
            "width" -> view.layoutParams = view.layoutParams.apply { width = dimension(view, operation.value) }
            "height" -> view.layoutParams = view.layoutParams.apply { height = dimension(view, operation.value) }
            "minWidth" -> view.minimumWidth = dp(number(operation.value))
            "minHeight" -> view.minimumHeight = dp(number(operation.value))
            "padding.start" -> view.setPaddingRelative(dp(number(operation.value)), view.paddingTop, view.paddingEnd, view.paddingBottom)
            "padding.top" -> view.setPaddingRelative(view.paddingStart, dp(number(operation.value)), view.paddingEnd, view.paddingBottom)
            "padding.end" -> view.setPaddingRelative(view.paddingStart, view.paddingTop, dp(number(operation.value)), view.paddingBottom)
            "padding.bottom" -> view.setPaddingRelative(view.paddingStart, view.paddingTop, view.paddingEnd, dp(number(operation.value)))
            "margin.start", "margin.top", "margin.end", "margin.bottom" -> applyMargin(view, operation.property, dp(number(operation.value)))
            "backgroundColor" -> applyBackgroundColor(view, parseColor(operation.value.value.asString))
            "contentColor" -> (view as? TextView)?.setTextColor(parseColor(operation.value.value.asString))
                ?: error("目标 View 不支持文字颜色")
            "borderColor" -> applyBorderColor(view, parseColor(operation.value.value.asString))
            "borderWidth" -> applyBorderWidth(view, dp(number(operation.value)))
            "cornerRadius.all" -> applyCornerRadius(view, dpToPx(view, number(operation.value)).toFloat())
            "text" -> (view as? TextView)?.text = operation.value.value.asString
                ?: error("目标 View 不支持文字")
            "textSize" -> (view as? TextView)?.setTextSize(TypedValue.COMPLEX_UNIT_SP, number(operation.value).toFloat())
                ?: error("目标 View 不支持字号")
            "opacity" -> view.alpha = number(operation.value).toFloat().coerceIn(0f, 1f)
            "visibility" -> view.visibility = when (operation.value.value.asString.lowercase(Locale.ROOT)) {
                "visible" -> View.VISIBLE
                "invisible" -> View.INVISIBLE
                "gone" -> View.GONE
                else -> error("visibility 只允许 visible/invisible/gone")
            }
            "translationX" -> view.translationX = dpToPx(view, number(operation.value)).toFloat()
            "translationY" -> view.translationY = dpToPx(view, number(operation.value)).toFloat()
            "scaleX" -> view.scaleX = number(operation.value).toFloat()
            "scaleY" -> view.scaleY = number(operation.value).toFloat()
            else -> error("不支持属性 ${operation.property}")
        }
    }

    private fun read(view: View, property: String): LivePropertyValue = when (property) {
        "width" -> dp(view, view.width)
        "height" -> dp(view, view.height)
        "minWidth" -> dp(view, view.minimumWidth)
        "minHeight" -> dp(view, view.minimumHeight)
        "padding.start" -> dp(view, view.paddingStart)
        "padding.top" -> dp(view, view.paddingTop)
        "padding.end" -> dp(view, view.paddingEnd)
        "padding.bottom" -> dp(view, view.paddingBottom)
        "margin.start" -> dp(view, (view.layoutParams as? ViewGroup.MarginLayoutParams)?.marginStart ?: 0)
        "margin.top" -> dp(view, (view.layoutParams as? ViewGroup.MarginLayoutParams)?.topMargin ?: 0)
        "margin.end" -> dp(view, (view.layoutParams as? ViewGroup.MarginLayoutParams)?.marginEnd ?: 0)
        "margin.bottom" -> dp(view, (view.layoutParams as? ViewGroup.MarginLayoutParams)?.bottomMargin ?: 0)
        "backgroundColor" -> colorValue(backgroundColor(view))
        "contentColor" -> colorValue((view as? TextView)?.currentTextColor ?: Color.TRANSPARENT)
        "borderColor" -> colorValue(reflectInt(view, "getStrokeColor") ?: Color.TRANSPARENT)
        "borderWidth" -> dp(view, reflectInt(view, "getStrokeWidth") ?: 0)
        "cornerRadius.all" -> dpValue(pxToDp(view, cornerRadius(view)))
        "text" -> textValue((view as? TextView)?.text?.toString().orEmpty())
        "textSize" -> spValue((view as? TextView)?.let(::textSizeSp) ?: 0f)
        "opacity" -> floatValue(view.alpha)
        "visibility" -> enumValue(visibilityName(view.visibility))
        "translationX" -> dpValue(pxToDp(view, view.translationX))
        "translationY" -> dpValue(pxToDp(view, view.translationY))
        "scaleX" -> floatValue(view.scaleX)
        "scaleY" -> floatValue(view.scaleY)
        else -> textValue("")
    }

    private fun styleProperties(view: View, properties: MutableMap<String, LivePropertySnapshot>, resourceId: String?) {
        addProperty(properties, "backgroundColor", colorValue(backgroundColor(view)), null, resourceId)
        if (supportsCornerRadius(view)) addProperty(properties, "cornerRadius.all", dpValue(pxToDp(view, cornerRadius(view))), null, resourceId)
        val strokeColor = reflectInt(view, "getStrokeColor")
        val strokeWidth = reflectInt(view, "getStrokeWidth")
        if (strokeColor != null && strokeWidth != null) {
            addProperty(properties, "borderColor", colorValue(strokeColor), null, resourceId)
            addProperty(properties, "borderWidth", dp(view, strokeWidth), null, resourceId)
        }
        if (view is TextView) {
            addProperty(properties, "text", textValue(view.text?.toString().orEmpty()), null, resourceId)
            addProperty(properties, "textSize", spValue(textSizeSp(view)), null, resourceId)
            addProperty(properties, "contentColor", colorValue(view.currentTextColor), null, resourceId)
        }
    }

    private fun marginProperties(view: View, properties: MutableMap<String, LivePropertySnapshot>, resourceId: String?) {
        val margins = view.layoutParams as? ViewGroup.MarginLayoutParams ?: return
        addProperty(properties, "margin.start", dp(view, margins.marginStart), null, resourceId)
        addProperty(properties, "margin.top", dp(view, margins.topMargin), null, resourceId)
        addProperty(properties, "margin.end", dp(view, margins.marginEnd), null, resourceId)
        addProperty(properties, "margin.bottom", dp(view, margins.bottomMargin), null, resourceId)
    }

    private fun addProperty(target: MutableMap<String, LivePropertySnapshot>, name: String, effective: LivePropertyValue, measured: LivePropertyValue?, resourceId: String?) {
        target[name] = LivePropertySnapshot(
            effective = effective,
            measured = measured,
            commitMode = if (resourceId == null) "SESSION_ONLY" else "CODEX",
        )
    }

    private fun applyMargin(view: View, property: String, value: Int) {
        val params = view.layoutParams as? ViewGroup.MarginLayoutParams
            ?: error("目标 View 不支持 margin")
        when (property) {
            "margin.start" -> params.marginStart = value
            "margin.top" -> params.topMargin = value
            "margin.end" -> params.marginEnd = value
            "margin.bottom" -> params.bottomMargin = value
        }
        view.layoutParams = params
    }

    private fun applyBackgroundColor(view: View, color: Int) {
        if (!invoke(view, "setCardBackgroundColor", Int::class.javaPrimitiveType!!, color)) {
            view.backgroundTintList = ColorStateList.valueOf(color)
        }
    }

    private fun applyCornerRadius(view: View, radiusPx: Float) {
        if (invoke(view, "setRadius", Float::class.javaPrimitiveType!!, radiusPx)) return
        if (invoke(view, "setCornerRadius", Int::class.javaPrimitiveType!!, radiusPx.toInt())) return
        val background = view.background?.mutate() as? GradientDrawable
            ?: error("目标 View 不支持圆角")
        background.cornerRadius = radiusPx
    }

    private fun applyBorderColor(view: View, color: Int) {
        if (!invoke(view, "setStrokeColor", Int::class.javaPrimitiveType!!, color)) {
            error("目标 View 不支持边框颜色")
        }
    }

    private fun applyBorderWidth(view: View, widthPx: Int) {
        if (!invoke(view, "setStrokeWidth", Int::class.javaPrimitiveType!!, widthPx)) {
            error("目标 View 不支持边框宽度")
        }
    }

    private fun supportsCornerRadius(view: View): Boolean =
        hasMethod(view, "getRadius") || hasMethod(view, "getCornerRadius") || view.background is GradientDrawable

    private fun cornerRadius(view: View): Float = reflectNumber(view, "getRadius")?.toFloat()
        ?: reflectNumber(view, "getCornerRadius")?.toFloat()
        ?: (view.background as? GradientDrawable)?.cornerRadius
        ?: 0f

    private fun backgroundColor(view: View): Int = reflectColorStateList(view, "getCardBackgroundColor")
        ?: view.backgroundTintList?.defaultColor
        ?: (view.background as? ColorDrawable)?.color
        ?: Color.TRANSPARENT

    private fun hasMethod(view: View, method: String): Boolean =
        view.javaClass.methods.any { it.name == method && it.parameterCount == 0 }

    private fun reflectMethod(view: View, method: String): Method? =
        view.javaClass.methods.firstOrNull { it.name == method && it.parameterCount == 0 }

    private fun reflectNumber(view: View, method: String): Number? =
        runCatching { reflectMethod(view, method)?.invoke(view) as? Number }.getOrNull()

    private fun reflectInt(view: View, method: String): Int? = reflectNumber(view, method)?.toInt()

    private fun reflectColorStateList(view: View, method: String): Int? = runCatching {
        (reflectMethod(view, method)?.invoke(view) as? ColorStateList)?.defaultColor
    }.getOrNull()

    private fun invoke(view: View, method: String, parameterType: Class<*>, value: Any): Boolean =
        runCatching {
            view.javaClass.getMethod(method, parameterType).invoke(view, value)
            true
        }.getOrDefault(false)

    private fun dimension(view: View, value: LivePropertyValue): Int {
        if (value.valueType.equals("dimension", ignoreCase = true)) {
            return when (value.value.asString.lowercase(Locale.ROOT)) {
                "wrapcontent", "wrap_content" -> ViewGroup.LayoutParams.WRAP_CONTENT
                "matchparent", "match_parent", "fill" -> ViewGroup.LayoutParams.MATCH_PARENT
                else -> error("未知 dimension: ${value.value.asString}")
            }
        }
        return dpToPx(view, number(value)).toInt()
    }

    private fun number(value: LivePropertyValue): Double = value.value.asDouble
    private fun parseColor(value: String): Int = Color.parseColor(value)
    private fun pxToDp(view: View, value: Float): Double = value / view.resources.displayMetrics.density.toDouble()
    private fun dpToPx(view: View, value: Double): Double = value * view.resources.displayMetrics.density
    private fun textSizeSp(view: TextView): Float = if (Build.VERSION.SDK_INT >= 34) {
        // Android 14 introduced non-linear font scaling; dividing by scaledDensity no
        // longer reverses the SP conversion at larger font scales.
        TypedValue.deriveDimension(
            TypedValue.COMPLEX_UNIT_SP,
            view.textSize,
            view.resources.displayMetrics,
        )
    } else {
        view.textSize / view.resources.displayMetrics.scaledDensity
    }
    private fun dp(view: View, px: Int): LivePropertyValue = dpValue(pxToDp(view, px.toFloat()))
    private fun dpValue(value: Double): LivePropertyValue = LivePropertyValue("dp", JsonPrimitive(round(value)))
    private fun spValue(value: Float): LivePropertyValue = LivePropertyValue("sp", JsonPrimitive(round(value.toDouble())))
    private fun floatValue(value: Float): LivePropertyValue = LivePropertyValue("float", JsonPrimitive(round(value.toDouble())))
    private fun textValue(value: String): LivePropertyValue = LivePropertyValue("text", JsonPrimitive(value))
    private fun enumValue(value: String): LivePropertyValue = LivePropertyValue("enum", JsonPrimitive(value))
    private fun colorValue(value: Int): LivePropertyValue = LivePropertyValue("argb", JsonPrimitive(String.format(Locale.ROOT, "#%08X", value)))
    private fun round(value: Double): Double = kotlin.math.round(value * 100.0) / 100.0
    private fun visibilityName(value: Int): String = when (value) {
        View.VISIBLE -> "visible"
        View.INVISIBLE -> "invisible"
        else -> "gone"
    }
}
