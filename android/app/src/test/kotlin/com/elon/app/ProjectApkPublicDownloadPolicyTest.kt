package com.elon.app

import okhttp3.OkHttpClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class ProjectApkPublicDownloadPolicyTest {
    private val publicServer = "http://main.example:8080"
    private val publicUrl =
        "$publicServer/api/store/projects/yilong-quant/downloads/android"

    private fun target(url: String?, projectId: String?, token: String?) =
        resolveProjectApkDownloadTarget(url, projectId, token, publicServer)

    @Test fun officialQuantUsesTheSamePublicUrlWithOrWithoutLoginToken() {
        val anonymous = target(publicUrl, "yilong-quant", null)
        val loggedIn = target(publicUrl, "yilong-quant", "secret bearer")

        assertEquals(publicUrl, anonymous?.url)
        assertEquals(anonymous, loggedIn)
        assertTrue(anonymous?.isPublic == true)
        assertFalse(anonymous?.url.orEmpty().contains("token="))
    }

    @Test fun officialQuantIgnoresCatalogPathsAndBuildsTheTrustedPublicRoute() {
        listOf(
            "http://user:pass@main.example:8080/api/store/projects/yilong-quant/downloads/android",
            "$publicUrl?token=secret",
            "$publicUrl?download=1",
            "$publicUrl#fragment",
            "http://main.example:8080/api/projects/yilong-quant/downloads/android",
            "http://main.example:8080/api/store/projects/yilong-quant/downloads/android/",
            "https://other.example/download/latest.apk",
        ).forEach { url ->
            assertEquals(url, publicUrl, target(url, "yilong-quant", "secret")?.url)
        }
        assertNull(target("ftp://main.example/download/latest.apk", "yilong-quant", null))
    }

    @Test fun officialQuantRequiresATrustedAbsoluteHttpOrHttpsServer() {
        val secureUrl = publicUrl.replace("http://", "https://")
        assertEquals(
            secureUrl,
            resolveProjectApkDownloadTarget(
                secureUrl,
                "yilong-quant",
                null,
                publicServer.replace("http://", "https://"),
            )?.url,
        )

        listOf(
            "/api/store/projects/yilong-quant/downloads/android",
            "file:///api/store/projects/yilong-quant/downloads/android",
        ).forEach { url ->
            assertNull(url, target(url, "yilong-quant", null))
        }
        assertEquals(
            publicUrl,
            target(
                "http:///api/store/projects/yilong-quant/downloads/android",
                "yilong-quant",
                null,
            )?.url,
        )
        assertEquals(
            publicUrl,
            target(
                "http://127.0.0.1:8080/api/projects/yilong-quant/download/latest.apk",
                "yilong-quant",
                null,
            )?.url,
        )
        listOf(
            "ftp://main.example:8080",
            "http://user:pass@main.example:8080",
            "http://main.example:8080/base",
            "http://main.example:8080?token=secret",
        ).forEach { server ->
            assertNull(resolveProjectApkDownloadTarget(publicUrl, "yilong-quant", null, server))
        }
    }

    @Test fun onlyTheStableProjectIdGetsThePublicBranch() {
        listOf(null, "", "YILONG-QUANT", "yilong-quant-copy", "一龙量化交易").forEach { id ->
            assertNull(target(publicUrl, id, null))
            val authenticated = target(publicUrl, id, "member token")
            assertTrue(authenticated?.isPublic == false)
            assertTrue(authenticated?.url.orEmpty().endsWith("?token=member+token"))
        }
    }

    @Test fun privateProjectsStillRequireAndAppendMemberToken() {
        val privateUrl = "https://downloads.example/project.apk"
        assertNull(target(privateUrl, "private-project", null))
        assertNull(target(privateUrl, "private-project", "  "))

        val downloadTarget = target(privateUrl, "private-project", "member token")
        assertEquals("https://downloads.example/project.apk?token=member+token", downloadTarget?.url)
        assertFalse(downloadTarget?.isPublic ?: true)
    }

    @Test fun publicDownloadsUseAnIsolatedClientWithoutRedirectsOrInterceptors() {
        val authenticatedClient = OkHttpClient.Builder()
            .addInterceptor { chain ->
                chain.proceed(
                    chain.request().newBuilder()
                        .header("Authorization", "Bearer member-secret")
                        .build(),
                )
            }
            .build()
        val publicTarget = requireNotNull(
            target(publicUrl, "yilong-quant", "member-secret"),
        )
        val publicClient = projectApkDownloadClient(authenticatedClient, publicTarget)

        assertNotSame(authenticatedClient, publicClient)
        assertTrue(publicClient.interceptors.isEmpty())
        assertTrue(publicClient.networkInterceptors.isEmpty())
        assertFalse(publicClient.followRedirects)
        assertFalse(publicClient.followSslRedirects)

        val privateTarget = requireNotNull(
            target(
                "https://downloads.example/project.apk",
                "private-project",
                "member-secret",
            ),
        )
        assertSame(authenticatedClient, projectApkDownloadClient(authenticatedClient, privateTarget))
    }
}
