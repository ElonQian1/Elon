package com.elon.app.esk.handoff

import android.app.Activity
import android.content.pm.PackageManager
import com.elon.app.OfficialQuantApkPolicy
import com.elon.app.currentPackageSignerSha256
import com.elon.app.projectApkVersionCode
import com.elon.app.readInstalledPackageInfo

internal const val ESK_MAIN_PACKAGE = "com.elon.app"
internal const val ESK_CONSENT_ACTIVITY = "com.elon.app.esk.handoff.EskSnapshotConsentActivity"
internal const val ESK_QUANT_ASSETS_ACTIVITY = "com.elon.quant.assets.EskAssetsActivity"

internal fun acceptsEskSnapshotCaller(packageName: String?, activityName: String?, signers: Set<String>,
    version: Long?, enabled: Boolean, aliasTarget: String?): Boolean =
    packageName == OfficialQuantApkPolicy.PACKAGE_NAME && activityName == ESK_QUANT_ASSETS_ACTIVITY &&
        enabled && aliasTarget == null && version != null && version >= 3 &&
        OfficialQuantApkPolicy.accepts(packageName, signers, version)

internal fun Activity.hasOfficialEskSnapshotCaller(): Boolean = runCatching {
    if (packageName != ESK_MAIN_PACKAGE || callingPackage != OfficialQuantApkPolicy.PACKAGE_NAME) return false
    val caller = callingActivity ?: return false
    if (caller.packageName != callingPackage || caller.className != ESK_QUANT_ASSETS_ACTIVITY) return false
    val installed = readInstalledPackageInfo(packageManager, caller.packageName) ?: return false
    @Suppress("DEPRECATION")
    val activity = packageManager.getActivityInfo(caller, PackageManager.MATCH_DISABLED_COMPONENTS)
    val componentSetting = packageManager.getComponentEnabledSetting(caller)
    val appSetting = packageManager.getApplicationEnabledSetting(caller.packageName)
    val permittedSettings = setOf(PackageManager.COMPONENT_ENABLED_STATE_DEFAULT, PackageManager.COMPONENT_ENABLED_STATE_ENABLED)
    acceptsEskSnapshotCaller(installed.packageName, activity.name, currentPackageSignerSha256(installed),
        installed.projectApkVersionCode(), activity.enabled && activity.applicationInfo.enabled &&
            activity.packageName == caller.packageName && componentSetting in permittedSettings &&
            appSetting in permittedSettings, activity.targetActivity)
}.getOrDefault(false)
