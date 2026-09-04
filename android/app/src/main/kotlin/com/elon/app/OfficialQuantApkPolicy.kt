package com.elon.app

/** Reviewed public release identity, not a login grant or trust-on-first-use pin. */
internal object OfficialQuantApkPolicy {
    const val PROJECT_ID = "yilong-quant"
    const val PACKAGE_NAME = "com.elon.quant"
    const val ACTIVITY_NAME = "com.elon.quant.MainActivity"
    const val SIGNER_SHA256 = "019a3d95366fb4c6fe578c1f7f26fb96e462dc54f41b9a7c7b5a715052e418bb"
    private const val MIN_VERSION_CODE = 5L

    fun appliesTo(projectId: String?): Boolean = projectId?.trim() == PROJECT_ID

    fun accepts(packageName: String?, currentSigners: Set<String>, versionCode: Long?): Boolean =
        packageName == PACKAGE_NAME &&
            currentSigners == setOf(SIGNER_SHA256) &&
            versionCode != null && versionCode >= MIN_VERSION_CODE
}
