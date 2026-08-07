package com.elon.app

import android.view.View
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.update.AppUpdateManager
import okhttp3.OkHttpClient

internal class MainProfileQuickActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val isBindingInitialized: () -> Boolean,
    private val refreshAccountUi: () -> Unit,
    private val fillPlanPrompt: () -> Unit,
    private val sendQuickCommand: (String) -> Unit,
    private val showProjectRecordDialog: () -> Unit,
    private val showGitProjectDialog: () -> Unit,
    private val openSettings: () -> Unit,
    private val openProfileDetails: () -> Unit,
    private val openAgentCenter: () -> Unit,
    private val showPromotionDialog: () -> Unit,
    private val showGuestImportDialog: () -> Unit,
    private val confirmLogout: () -> Unit
) {
    private val tokenUsageCard by lazy { ProfileTokenUsageCard(activity, binding) }
    private val nodeMarketSheet by lazy { NodeMarketSheet(activity, http, serverUrl) }
    private val nodeTransactionSheet by lazy { NodeTransactionSheet(activity, http, serverUrl) }
    private val nodeBalanceCard by lazy {
        ProfileNodeBalanceCard(
            activity, binding, http, serverUrl,
            openTransactions = { nodeTransactionSheet.show() }
        )
    }
    private val nodeDirectoryCard by lazy {
        ProfileNodeDirectoryCard(
            activity, binding, http, serverUrl,
            openMarket = { nodeMarketSheet.show() }
        )
    }
    private val myNodesCard by lazy { MyNodesCard(activity, binding, http, serverUrl) }
    private val userMemoriesCard by lazy { UserMemoriesCard(activity, binding, http, serverUrl) }
    private val accountSecurityEntry by lazy { ProfileAccountSecurityEntry(activity, binding) }
    private var nodeResourceExpanded = false
    private val stopObservingUpdate = AppUpdateManager.observeProfileStatus(activity) { status ->
        activity.runOnUiThread {
            if (isBindingInitialized()) binding.profileVersionText.text = status
        }
    }

    init {
        activity.lifecycle.addObserver(object : DefaultLifecycleObserver {
            override fun onDestroy(owner: LifecycleOwner) {
                stopObservingUpdate()
            }
        })
    }

    fun setupQuickActions() {
        MainQuickActionBindings(
            activity = activity,
            binding = binding,
            fillPlanPrompt = fillPlanPrompt,
            sendQuickCommand = sendQuickCommand,
            showProjectRecordDialog = showProjectRecordDialog,
            showGitProjectDialog = showGitProjectDialog,
            openSettings = openSettings,
            openAgentCenter = openAgentCenter,
            showPromotionDialog = showPromotionDialog,
            showGuestImportDialog = showGuestImportDialog,
            confirmLogout = confirmLogout,
            openNodeMarket = { nodeMarketSheet.show() }
        ).setupQuickActions()
        setupNodeResourcePanel()
        refreshAccountUi()
        refreshProfileSummary()
        refreshUpdateStatus()
    }

    fun refreshProfileSummary() {
        if (isBindingInitialized()) {
            UserProfileViews.renderSummary(
                activity,
                binding,
                http,
                serverUrl,
                openProfileDetails
            )
            tokenUsageCard.attachAndRefresh()
            renderNodeResourcePanel()
            if (nodeResourceExpanded) {
                refreshNodeResourceCards()
            }
            userMemoriesCard.attachAndRefresh()
            accountSecurityEntry.attachAndRefresh()
            refreshUpdateStatus()
        }
    }

    private fun refreshUpdateStatus() {
        if (isBindingInitialized()) {
            binding.profileVersionText.text = AppUpdateManager.profileStatusLine(activity)
        }
    }

    /** 保留导航控制器的旧入口名；个人页这里现在展示的是 APK 更新状态。 */
    fun refreshServerVersion() = refreshUpdateStatus()

    private fun setupNodeResourcePanel() {
        binding.profileNodeResourceHeader.setOnClickListener {
            nodeResourceExpanded = !nodeResourceExpanded
            renderNodeResourcePanel()
            if (nodeResourceExpanded) {
                refreshNodeResourceCards()
            }
        }
        renderNodeResourcePanel()
    }

    private fun renderNodeResourcePanel() {
        binding.profileNodeResourceContent.visibility =
            if (nodeResourceExpanded) View.VISIBLE else View.GONE
        binding.profileNodeResourceHint.text =
            if (nodeResourceExpanded) "节点积分、全站节点、我的节点"
            else "积分、全站节点、我的节点"
        binding.profileNodeResourceArrow.rotation = if (nodeResourceExpanded) 90f else 0f
        binding.profileNodeResourceHeader.contentDescription =
            if (nodeResourceExpanded) "收起 PC 节点资源" else "展开 PC 节点资源"
    }

    private fun refreshNodeResourceCards() {
        nodeBalanceCard.attachAndRefresh()
        nodeDirectoryCard.attachAndRefresh()
        myNodesCard.attachAndRefresh()
    }
}
