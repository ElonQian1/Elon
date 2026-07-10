package com.elon.uiruntime.view

import android.view.View

/**
 * 为当前 View 声明跨截图、跨会话稳定的定义 ID。
 *
 * 建议使用 `screen.component.part`，例如 `checkout.pay_button`。重复列表项通过
 * [instanceKey] 传入业务稳定键（SKU、订单号等），不要使用列表下标。
 * 该 API 只存在于 Debug Runtime；Release 包不会包含此模块。
 */
fun <T : View> T.uiNode(
    definitionId: String,
    instanceKey: String? = null,
): T = apply {
    require(definitionId.isNotBlank()) { "definitionId 不能为空" }
    setTag(R.id.yilong_ui_node_id, definitionId.trim())
    setTag(R.id.yilong_ui_instance_key, instanceKey?.trim()?.takeIf(String::isNotEmpty))
}
