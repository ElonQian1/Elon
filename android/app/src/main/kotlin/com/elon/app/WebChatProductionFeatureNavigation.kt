package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONObject

internal data class WebChatProductionFeature(
    val id: String,
    val label: String,
    val kind: String,
    val selected: Boolean,
    val requiresUserConfirmation: Boolean,
    val nativeSelector: String,
)

internal object WebChatProductionFeatureParser {
    fun parse(navigation: JSONObject): List<WebChatProductionFeature> {
        val features = navigation.optJSONArray("features") ?: return emptyList()
        val parsed = mutableListOf<WebChatProductionFeature>()
        for (index in 0 until features.length()) {
            val feature = features.optJSONObject(index) ?: continue
            val id = feature.optString("id").trim()
            val label = feature.optString("label").trim()
            if (id.isBlank() || label.isBlank()) continue
            parsed += WebChatProductionFeature(
                id = id,
                label = label,
                kind = feature.optString("kind").trim(),
                selected = feature.optBoolean("selected"),
                requiresUserConfirmation = feature.optBoolean("requires_user_confirmation"),
                nativeSelector = feature.optString("native_adb_content_description")
                    .trim()
                    .ifBlank { "web-chat-feature:$id" },
            )
        }
        return parsed.distinctBy(WebChatProductionFeature::id)
    }
}

internal class WebChatProductionFeatureNavigationCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val mcpPort: () -> WebChatSocialMcpPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeDialog: AlertDialog? = null

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        if (!provider.supports(WebChatProviderCapability.FEATURE_NAVIGATION)) {
            openOfficialFallback()
            return
        }
        val port = mcpPort() ?: return showUnavailable()
        val epoch = requestEpoch
        val cached = readFeatures(port)
        if (cached.isNotEmpty()) {
            showFeatureDialog(provider, port, cached)
            return
        }
        val requested = port.control(JSONObject().put("action", "chatgpt_list_features"))
        if (!requested.optBoolean("control_ok")) {
            showUnavailable()
            return
        }
        Toast.makeText(activity, "正在读取官网功能...", Toast.LENGTH_SHORT).show()
        pollFeatures(provider, port, epoch, attempt = 0)
    }

    fun cancelPending() {
        requestEpoch += 1
        activeDialog?.dismiss()
        activeDialog = null
    }

    private fun pollFeatures(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val features = readFeatures(port)
        if (features.isNotEmpty()) {
            showFeatureDialog(provider, port, features)
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) {
            showUnavailable()
            return
        }
        host.postDelayed(
            { pollFeatures(provider, port, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
    }

    private fun readFeatures(port: WebChatSocialMcpPort): List<WebChatProductionFeature> {
        val navigation = port.control(JSONObject().put("action", "chatgpt_get_navigation"))
        if (!navigation.optBoolean("control_ok")) return emptyList()
        return WebChatProductionFeatureParser.parse(navigation)
    }

    private fun showFeatureDialog(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        features: List<WebChatProductionFeature>,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeProvider() != provider.id) return
        if (features.isEmpty()) return showUnavailable()
        val labels = features.map { feature ->
            if (feature.selected) "${feature.label}（当前）" else feature.label
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("${provider.displayName}功能")
            .setItems(labels.toTypedArray()) { _, index ->
                features.getOrNull(index)?.let { selectFeature(port, it) }
            }
            .setNeutralButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .create()
        dialog.setOnShowListener {
            dialog.listView?.apply {
                contentDescription = "web-chat-feature-navigation:${provider.id.wireValue}"
                post {
                    for (childIndex in 0 until childCount) {
                        val featureIndex = firstVisiblePosition + childIndex
                        getChildAt(childIndex)?.contentDescription =
                            features.getOrNull(featureIndex)?.nativeSelector
                    }
                }
            }
        }
        showTracked(dialog)
    }

    private fun selectFeature(port: WebChatSocialMcpPort, feature: WebChatProductionFeature) {
        if (feature.selected) return
        if (!feature.requiresUserConfirmation) {
            dispatchFeature(port, feature, userConfirmed = false)
            return
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle(feature.label)
            .setMessage("此功能可能包含健康或财务等敏感信息，是否继续打开？")
            .setPositiveButton("继续") { _, _ ->
                dispatchFeature(port, feature, userConfirmed = true)
            }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
    }

    private fun dispatchFeature(
        port: WebChatSocialMcpPort,
        feature: WebChatProductionFeature,
        userConfirmed: Boolean,
    ) {
        val result = port.control(JSONObject()
            .put("action", "chatgpt_select_feature")
            .put("feature_id", feature.id)
            .put("user_confirmed", userConfirmed))
        if (!result.optBoolean("control_ok")) {
            val message = when (result.optString("error")) {
                "stale_feature_id" -> "官网功能已变化，请重新打开列表"
                "user_confirmation_required" -> "需要确认后才能打开此功能"
                "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
                else -> "暂时无法打开此功能，请重试"
            }
            Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在打开${feature.label}", Toast.LENGTH_SHORT).show()
    }

    private fun showUnavailable() {
        if (activity.isFinishing || activity.isDestroyed) return
        val dialog = AlertDialog.Builder(activity)
            .setTitle("官网功能")
            .setMessage("当前网页没有返回可用功能，可在官方页面继续。")
            .setPositiveButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
    }

    private fun showTracked(dialog: AlertDialog) {
        activeDialog?.dismiss()
        activeDialog = dialog
        dialog.setOnDismissListener {
            if (activeDialog === dialog) activeDialog = null
        }
        dialog.show()
    }

    private companion object {
        const val MAX_POLL_ATTEMPTS = 8
        const val POLL_INTERVAL_MS = 250L
    }
}
