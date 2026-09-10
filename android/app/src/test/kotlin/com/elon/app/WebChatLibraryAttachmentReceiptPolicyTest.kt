package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatLibraryAttachmentReceiptPolicyTest {
    private val policy = WebChatLibraryAttachmentReceiptPolicy

    @Test fun oldUiAndCommandLimitsDoNotEndAnActiveOperation() {
        for (elapsed in listOf(0L, 16_000L, 20_000L, 24_000L, 30_999L)) {
            assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.WAIT,
                policy.outcome(false, false, null, elapsed))
        }
    }

    @Test fun exactSuccessAfterSlowPreparationReturnsToTheComposer() {
        assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.ATTACHED,
            policy.outcome(true, false, "library_attachment_associated", 22_000))
    }

    @Test fun observedSuccessWinsEvenWhenTheUiCallbackWasDelayed() {
        assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.ATTACHED,
            policy.outcome(true, false, "library_attachment_associated", 50_000))
    }

    @Test fun unrelatedSuccessIsNotAnAttachmentReceipt() {
        for (detail in listOf(null, "", "library_mutation_acknowledged")) {
            assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.UNCONFIRMED,
                policy.outcome(true, false, detail, 100))
        }
    }

    @Test fun failedOrTimedOutReceiptIsNotPresentedAsConfirmed() {
        assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.UNCONFIRMED,
            policy.outcome(false, true, "library_attachment_unconfirmed", 100))
    }

    @Test fun successDetailWithoutSucceededStatusCannotCloseTheBrowser() {
        assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.WAIT,
            policy.outcome(false, false, "library_attachment_associated", 100))
    }

    @Test fun missingReceiptHasABoundedObservationWindow() {
        assertEquals(WebChatLibraryAttachmentReceiptPolicy.Outcome.UNCONFIRMED,
            policy.outcome(false, false, null, 31_000))
    }

    @Test fun deadlinesCoverBothPreparationStepsAndReceiptDelivery() {
        assertTrue(policy.OPERATION_TIMEOUT_MS > 10_000 + 12_000)
        assertTrue(policy.COMMAND_TIMEOUT_MS > policy.OPERATION_TIMEOUT_MS)
        assertTrue(policy.OBSERVATION_TIMEOUT_MS > policy.COMMAND_TIMEOUT_MS)
    }
}
