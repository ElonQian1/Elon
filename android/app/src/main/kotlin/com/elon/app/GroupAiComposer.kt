package com.elon.app

import android.view.View
import android.app.Dialog
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.elon.app.googleweb.GoogleWebOfficialActivity
import com.elon.app.googleweb.GoogleWebNavigationPolicy
import com.elon.app.databinding.ActivityMainBinding

/** Group-only UI owner. Engine configuration stays separate from social message delivery. */
internal class GroupAiComposer(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val views: () -> MainInputComposerViews?,
    private val server: String,
    private val owner: () -> String,
    private val restoreWorkModel: () -> Unit,
    private val showWorkModel: () -> Unit,
    private val refreshComposer: () -> Unit,
) : DefaultLifecycleObserver {
    private var group: String? = null
    private var store: GroupAiConfigurationStore? = null
    private var config = GroupAiConfiguration()
    private var picker: GroupAiModelPicker? = null
    private var menu: Dialog? = null
    private var workSettings: GroupWorkAiSettings? = null
    private var account = ""
    private var previousWidth = 0
    private var previousPlanVisibility = View.VISIBLE

    init { activity.lifecycle.addObserver(this) }

    fun configuration(): GroupAiConfiguration = config.copy(modelPath = config.modelPath.toList())

    fun open(id: String) {
        picker?.close()
        workSettings?.close()
        menu?.dismiss()
        if (group == null) {
            previousWidth = views()?.modelButtonShell?.layoutParams?.width ?: dp(142)
            previousPlanVisibility = views()?.planModeButton?.visibility ?: View.VISIBLE
        }
        group = id
        account = owner()
        store = GroupAiConfigurationStore(activity, server, account)
        config = requireNotNull(store).read(id)
        render()
    }

    private fun valid() = group != null && account == owner() && !activity.isDestroyed

    private fun save(next: GroupAiConfiguration) {
        if (!valid()) return
        config = next
        store?.save(requireNotNull(group), next)
        render()
    }

    private fun render() {
        if (!valid()) return
        val ui = views() ?: return
        binding.modelButton.tag = WEB_CHAT_MODEL_BUTTON_OWNER
        ui.modelButtonShell.tag = WEB_CHAT_MODEL_BUTTON_OWNER
        ui.modelButtonShell.layoutParams = ui.modelButtonShell.layoutParams.apply { width = dp(142) }
        ui.planModeButton.visibility = View.GONE
        ui.webToolsButton.visibility = View.GONE
        ui.attachmentButton.visibility = View.VISIBLE
        val provider = config.engine.providerId
        if (provider != null) WebChatComposerProviderPresentation.applyGroup(binding.modelButton,
            WebChatProviderRegistry.get(provider), config.label)
        else {
            WebChatComposerProviderPresentation.clear(binding.modelButton)
            binding.modelButton.text = config.label
        }
        val description = "group-ai-settings:${config.engine.name}；${config.label}"
        binding.modelButton.contentDescription = description
        ui.modelButtonShell.contentDescription = description
        ui.modelButtonShell.setOnClickListener { showSettings() }
        binding.modelButton.setOnClickListener { showSettings() }
        refreshComposer()
    }

    private fun showSettings() {
        if (!valid()) return
        picker?.close()
        workSettings?.close()
        val ui = views() ?: return
        if (config.engine == GroupAiEngine.GOOGLE) {
            showProviders()
            return
        }
        if (!config.usesWebAi) {
            val expectedGroup = requireNotNull(group)
            val expectedAccount = account
            workSettings = GroupWorkAiSettings(activity, server, expectedGroup, requireNotNull(store),
                current = { config.work }, valid = { valid() && group == expectedGroup && account == expectedAccount },
                save = { save(config.copy(work = it)) }, switchAi = ::showProviders).also { it.show() }
            return
        }
        val expectedGroup = group
        picker = GroupAiModelPicker(activity, ui.modelButtonShell, requireNotNull(store).catalog(), config,
            select = { choices -> if (group == expectedGroup) save(config.copy(modelPath = choices)) },
            switchProvider = { if (group == expectedGroup) showProviders() },
        ).also { it.show() }
    }

    private fun showProviders() {
        if (!valid()) return
        val expectedGroup = group
        val expectedAccount = account
        menu = ChatAiChoiceSheet.show(activity, "切换 AI", GroupAiProviderChoices.options(config), onSelected = { id ->
                if (group != expectedGroup || account != expectedAccount || !valid()) return@show
                val engine = GroupAiEngine.valueOf(id)
                if (engine == GroupAiEngine.WORK && config.engine != engine) {
                    menu = AlertDialog.Builder(activity).setTitle("使用工作 AI")
                        .setMessage("群聊 AI 回复将使用所选服务端模型，可能计费。只改变这个群的 AI 通道。")
                        .setNegativeButton("取消", null).setPositiveButton("切换") { _, _ ->
                            if (group == expectedGroup && account == expectedAccount && valid()) {
                                save(config.copy(engine = engine))
                                showSettings()
                            }
                        }.show()
                } else {
                    save(config.copy(engine = engine))
                    if (engine != GroupAiEngine.GOOGLE) showSettings()
                }
            }, actions = if (config.engine == GroupAiEngine.GOOGLE) listOf(
                ChatAiSheetAction("Google 官方页", "group-ai-google-official") {
                    if (group == expectedGroup && account == expectedAccount && valid()) {
                        activity.startActivity(GoogleWebOfficialActivity.createIntent(activity, GoogleWebNavigationPolicy.START_URL))
                    }
                },
            ) else emptyList())?.dialog
    }

    fun close() {
        if (group == null) return
        group = null
        picker?.close()
        picker = null
        workSettings?.close()
        workSettings = null
        menu?.dismiss()
        menu = null
        views()?.let { ui ->
            ui.modelButtonShell.tag = null
            ui.modelButtonShell.layoutParams = ui.modelButtonShell.layoutParams.apply { width = previousWidth }
            ui.planModeButton.visibility = previousPlanVisibility
            ui.modelButtonShell.setOnClickListener { showWorkModel() }
        }
        binding.modelButton.tag = null
        WebChatComposerProviderPresentation.clear(binding.modelButton)
        binding.modelButton.setOnClickListener { showWorkModel() }
        restoreWorkModel()
        refreshComposer()
    }

    override fun onStop(owner: LifecycleOwner) { picker?.close(); workSettings?.close(); menu?.dismiss() }
    override fun onDestroy(owner: LifecycleOwner) { close() }
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()

    companion object {
        fun create(activity: AppCompatActivity, binding: ActivityMainBinding, input: MainInputActions,
            models: MainModelActions, server: String, owner: () -> String) = GroupAiComposer(
            activity, binding, input::inputComposerViewsOrNull, server, owner, models::updateModelButton,
            models::showModelPopupOrLoad, input.sendButtonVisualActions::updateSendButtonVisual,
        )
    }
}
