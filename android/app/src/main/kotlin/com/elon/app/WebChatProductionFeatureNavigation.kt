package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal data class WebChatProductionFeature(
    val id: String,
    val label: String,
    val kind: String,
    val selected: Boolean,
    val requiresUserConfirmation: Boolean,
    val officialCompletion: Boolean,
    val nativeSelector: String,
) {
    fun navigationLabel(): String = when {
        selected && officialCompletion -> "$label（当前·官网）"
        selected -> "$label（当前）"
        officialCompletion -> "$label（官网）"
        else -> label
    }
}

internal object WebChatProductionFeatureParser {
    fun parse(features: List<WebChatConsumerFeature>): List<WebChatProductionFeature> {
        return features.mapNotNull { feature ->
            val id = feature.id.trim()
            val label = feature.label.trim()
            if (id.isBlank() || label.isBlank()) return@mapNotNull null
            WebChatProductionFeature(
                id = id,
                label = label,
                kind = feature.kind.trim(),
                selected = feature.selected,
                requiresUserConfirmation = feature.requiresUserConfirmation,
                officialCompletion = WebChatProductionFeatureCompletionPolicy
                    .requiresOfficialCompletion(feature.kind),
                nativeSelector = feature.nativeSelector
                    .trim()
                    .ifBlank { "web-chat-feature:$id" },
            )
        }
            .distinctBy(WebChatProductionFeature::id)
    }
}

internal class WebChatProductionFeatureNavigationCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
) {
    private var requestEpoch = 0
    private var activeDialog: AlertDialog? = null
    private var activeSheet: WebChatActionSheetHandle? = null
    private var featureById = emptyMap<String, WebChatProductionFeature>()

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        if (!provider.supports(WebChatProviderCapability.FEATURE_NAVIGATION)) {
            openOfficialFallback()
            return
        }
        val port = consumerPort() ?: return showUnavailable()
        val epoch = requestEpoch
        showFeatureDialog(provider, port, readFeatures(provider.id, port))
        val requested = port.requestFeatures()
        if (requested.accepted) pollFeatures(provider, port, epoch, attempt = 0)
    }

    fun cancelPending() {
        requestEpoch += 1
        activeSheet?.dismiss()
        activeSheet = null
        featureById = emptyMap()
        activeDialog?.dismiss()
        activeDialog = null
    }

    private fun pollFeatures(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val observed = port.state().features
        val features = WebChatProductionFeatureParser.parse(observed)
        if (features.isNotEmpty()) {
            interactionCache.features(provider.id, observed)
            showFeatureDialog(provider, port, features)
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) return
        host.postDelayed(
            { pollFeatures(provider, port, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
    }

    private fun readFeatures(
        providerId: WebChatProviderId,
        port: WebChatConsumerPort,
    ): List<WebChatProductionFeature> = WebChatProductionFeatureParser.parse(
        interactionCache.features(providerId, port.state().features),
    )

    private fun showFeatureDialog(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        features: List<WebChatProductionFeature>,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeProvider() != provider.id) return
        featureById = features.associateBy(WebChatProductionFeature::id)
        val availableItems = features.map { feature ->
            WebChatActionSheetItem(
                id = feature.id,
                title = feature.label,
                subtitle = when {
                    feature.selected && feature.officialCompletion -> "当前功能 · 在官网中继续"
                    feature.selected -> "当前功能"
                    feature.officialCompletion -> "在官网中完成"
                    else -> null
                },
                selected = feature.selected,
                contentDescription = feature.nativeSelector,
            )
        }
        val items = availableItems.ifEmpty {
            listOf(WebChatProductionInteractionPlaceholder.item(
                provider.id,
                surface = "features",
                title = "官网功能",
            ))
        }
        activeSheet?.let {
            it.updateItems(items)
            return
        }
        val sheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "${provider.displayName}功能",
            items = items,
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网完整功能",
                    contentDescription = "web-chat-feature-official:${provider.id.wireValue}",
                    action = openOfficialFallback,
                ),
            ),
            onCancelled = { requestEpoch += 1 },
            onDismissed = {
                activeSheet = null
                featureById = emptyMap()
            },
        ) { item ->
            if (activeProvider() != provider.id) return@showUpdatable
            featureById[item.id]?.let { selectFeature(provider, port, it) }
        }
        activeSheet = sheet
    }

    private fun selectFeature(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        feature: WebChatProductionFeature,
    ) {
        if (feature.selected) {
            if (feature.officialCompletion) openOfficialFallback()
            return
        }
        if (!feature.requiresUserConfirmation) {
            dispatchFeature(provider, port, feature, userConfirmed = false)
            return
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle(feature.label)
            .setMessage("此功能可能包含健康或财务等敏感信息，是否继续打开？")
            .setPositiveButton("继续") { _, _ ->
                dispatchFeature(provider, port, feature, userConfirmed = true)
            }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
    }

    private fun dispatchFeature(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        feature: WebChatProductionFeature,
        userConfirmed: Boolean,
    ) {
        val result = port.selectFeature(feature.id, userConfirmed)
        if (!result.accepted) {
            val message = when (result.error) {
                "stale_feature_id" -> "官网功能已变化，请重新打开列表"
                "user_confirmation_required" -> "需要确认后才能打开此功能"
                "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
                else -> "暂时无法打开此功能，请重试"
            }
            Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
            return
        }
        if (!feature.officialCompletion) {
            Toast.makeText(activity, "正在打开${feature.label}", Toast.LENGTH_SHORT).show()
            return
        }
        val requestId = result.requestId
        if (requestId == null) {
            Toast.makeText(activity, "官网没有返回可确认的打开状态，请重试", Toast.LENGTH_SHORT).show()
            return
        }
        val epoch = requestEpoch
        Toast.makeText(activity, "正在打开${feature.label}官网页面...", Toast.LENGTH_SHORT).show()
        pollOfficialCompletion(provider, port, feature, requestId, epoch, attempt = 0)
    }

    private fun pollOfficialCompletion(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        feature: WebChatProductionFeature,
        requestId: String,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        when (WebChatProductionFeatureCompletionPolicy.evaluate(feature, requestId, port.state())) {
            WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL -> {
                Toast.makeText(activity, "已打开${feature.label}", Toast.LENGTH_SHORT).show()
                openOfficialFallback()
            }
            WebChatProductionFeatureCompletionDecision.FAILED ->
                Toast.makeText(activity, "官网未能打开${feature.label}，请重试", Toast.LENGTH_SHORT).show()
            WebChatProductionFeatureCompletionDecision.WAITING -> {
                if (attempt >= MAX_FEATURE_SETTLE_ATTEMPTS) {
                    showFeatureSettleTimeout(feature)
                    return
                }
                host.postDelayed({
                    pollOfficialCompletion(provider, port, feature, requestId, epoch, attempt + 1)
                }, POLL_INTERVAL_MS)
            }
        }
    }

    private fun showFeatureSettleTimeout(feature: WebChatProductionFeature) {
        val dialog = AlertDialog.Builder(activity)
            .setTitle(feature.label)
            .setMessage("官网页面仍在加载。可以打开官方页继续，或稍后重新选择此功能。")
            .setPositiveButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
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
        const val MAX_FEATURE_SETTLE_ATTEMPTS = 32
        const val POLL_INTERVAL_MS = 250L
    }
}
