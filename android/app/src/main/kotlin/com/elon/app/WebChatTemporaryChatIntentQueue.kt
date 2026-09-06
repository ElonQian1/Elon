package com.elon.app

internal sealed interface WebChatTemporaryChatIntentDecision {
    object Idle : WebChatTemporaryChatIntentDecision
    object AwaitingControl : WebChatTemporaryChatIntentDecision
    object AwaitingConfirmation : WebChatTemporaryChatIntentDecision
    data class Apply(val controlId: String, val selected: Boolean) :
        WebChatTemporaryChatIntentDecision
    data class Confirmed(val selected: Boolean) : WebChatTemporaryChatIntentDecision
}

/**
 * Keeps one user intent while the current official control id is discovered.
 * A mutation is emitted once and completion remains unconfirmed until the page reports it.
 */
internal class WebChatTemporaryChatIntentQueue {
    private var desiredSelected: Boolean? = null
    private var mutationIssued = false
    private var rejectedControlId: String? = null

    fun begin(desiredSelected: Boolean): Boolean {
        if (this.desiredSelected != null) return false
        this.desiredSelected = desiredSelected
        mutationIssued = false
        rejectedControlId = null
        return true
    }

    fun evaluate(control: WebChatConsumerControl?): WebChatTemporaryChatIntentDecision {
        val desired = desiredSelected ?: return WebChatTemporaryChatIntentDecision.Idle
        if (control == null) return WebChatTemporaryChatIntentDecision.AwaitingControl
        if (control.selected == desired) {
            return WebChatTemporaryChatIntentDecision.Confirmed(desired)
        }
        if (control.id == rejectedControlId || !control.supportsSelectedState) {
            return WebChatTemporaryChatIntentDecision.AwaitingControl
        }
        if (mutationIssued) return WebChatTemporaryChatIntentDecision.AwaitingConfirmation
        mutationIssued = true
        return WebChatTemporaryChatIntentDecision.Apply(control.id, desired)
    }

    fun mutationRejected(controlId: String) {
        mutationIssued = false
        rejectedControlId = controlId
    }

    fun clear() {
        desiredSelected = null
        mutationIssued = false
        rejectedControlId = null
    }
}
