package com.elon.app.esk.platform.access

import com.elon.app.privateaccess.NativeReadApprovalClient

/** The original ESK purpose keeps its exact request and response validation. */
internal class AssetAccessApprovalClient {
    private val transport = NativeReadApprovalClient()
    fun cancel() = transport.cancel()
    fun authorize(base: String, token: String, input: AssetAccessRequest): String =
        transport.authorize(base, token, input.approvalBody()) { input.validateResult(it, System.currentTimeMillis()) }
}