package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProductionCapabilityEvidenceTest {
    @Test
    fun emptyDomNeverMeansUnsupported() {
        listOf(
            evidence(adapterCurrent = false),
            evidence(adapterCurrent = true),
            evidence(adapterCurrent = true, pollingExhausted = true),
            evidence(
                adapterCurrent = true,
                requestAccepted = false,
                requestError = "temporary_command_failure",
            ),
        ).forEach { input ->
            val actual = WebChatProductionCapabilityEvidencePolicy.resolve(input)
            require(actual != WebChatProductionObservationState.ADAPTER_UNSUPPORTED)
        }
    }

    @Test
    fun distinguishesRecoverySyncUnobservedAndFailure() {
        assertEquals(
            WebChatProductionObservationState.SESSION_RECOVERING,
            resolve(evidence(adapterCurrent = false)),
        )
        assertEquals(
            WebChatProductionObservationState.SESSION_RECOVERING,
            resolve(evidence(
                adapterCurrent = true,
                requestAccepted = false,
                requestError = "bridge_not_ready",
            )),
        )
        assertEquals(
            WebChatProductionObservationState.SYNCING,
            resolve(evidence(adapterCurrent = true, requestAccepted = true)),
        )
        assertEquals(
            WebChatProductionObservationState.TEMPORARILY_UNOBSERVED,
            resolve(evidence(
                adapterCurrent = true,
                requestAccepted = true,
                pollingExhausted = true,
            )),
        )
        assertEquals(
            WebChatProductionObservationState.REQUEST_FAILED,
            resolve(evidence(
                adapterCurrent = true,
                requestAccepted = true,
                requestStatus = WebChatConsumerCommandStatus.TIMED_OUT,
            )),
        )
    }

    @Test
    fun cacheOrObservationIsAvailable() {
        assertEquals(
            WebChatProductionObservationState.AVAILABLE,
            resolve(evidence(adapterCurrent = false, cachedCount = 2)),
        )
        assertEquals(
            WebChatProductionObservationState.AVAILABLE,
            resolve(evidence(adapterCurrent = true, observedCount = 1)),
        )
    }

    @Test
    fun unsupportedRequiresAuthoritativeEvidence() {
        assertEquals(
            WebChatProductionObservationState.ADAPTER_UNSUPPORTED,
            resolve(evidence(declaredSupported = false, adapterCurrent = true)),
        )
        assertEquals(
            WebChatProductionObservationState.ADAPTER_UNSUPPORTED,
            resolve(evidence(
                adapterCurrent = true,
                requestAccepted = false,
                requestError = "adapter_unsupported",
            )),
        )
    }

    @Test
    fun resolvesCommandStatusOnlyFromMatchingReceipt() {
        val request = WebChatConsumerCommandResult(
            accepted = true,
            requestId = "request-2",
        )
        val state = state(listOf(
            WebChatConsumerCommandRequest("request-1", WebChatConsumerCommandStatus.SUCCEEDED),
            WebChatConsumerCommandRequest("request-2", WebChatConsumerCommandStatus.PENDING),
        ))

        assertEquals(
            WebChatConsumerCommandStatus.PENDING,
            WebChatProductionCapabilityEvidencePolicy.requestStatus(request, state),
        )
    }

    private fun resolve(evidence: WebChatProductionCapabilityEvidence) =
        WebChatProductionCapabilityEvidencePolicy.resolve(evidence)

    private fun evidence(
        declaredSupported: Boolean = true,
        adapterCurrent: Boolean,
        observedCount: Int = 0,
        cachedCount: Int = 0,
        requestAccepted: Boolean? = null,
        requestError: String? = null,
        requestStatus: WebChatConsumerCommandStatus? = null,
        pollingExhausted: Boolean = false,
    ) = WebChatProductionCapabilityEvidence(
        declaredSupported = declaredSupported,
        adapterCurrent = adapterCurrent,
        observedCount = observedCount,
        cachedCount = cachedCount,
        requestAccepted = requestAccepted,
        requestError = requestError,
        requestStatus = requestStatus,
        pollingExhausted = pollingExhausted,
    )

    private fun state(
        requests: List<WebChatConsumerCommandRequest>,
    ) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = "chat",
        pageUrl = "https://chatgpt.com/",
        features = emptyList(),
        commandRequests = requests,
    )
}
