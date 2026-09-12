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
    private val memberServer = "http://main.example:8080"
    private val memberUrl =
        "$memberServer/api/store/projects/yilong-quant/downloads/android"

    private fun target(url: String?, projectId: String?, token: String?) =
        resolveProjectApkDownloadTarget(url, projectId, token, memberServer)

    @Test fun officialQuantRequiresAMemberTokenAndKeepsItOutOfTheUrl() {
        val anonymous = target(memberUrl, "yilong-quant", null)
        val loggedIn = target(memberUrl, "yilong-quant", "secret bearer")

        assertNull(anonymous)
        assertEquals(memberUrl, loggedIn?.url)
        assertEquals("secret bearer", loggedIn?.bearerToken)
        assertTrue(loggedIn?.isolatedClient == true)
        assertFalse(loggedIn?.url.orEmpty().contains("token="))
    }

    @Test fun officialQuantIgnoresCatalogPathsAndBuildsTheTrustedMemberRoute() {
        listOf(
            "http://user:pass@main.example:8080/api/store/projects/yilong-quant/downloads/android",
            "$memberUrl?token=secret",
            "$memberUrl?download=1",
            "$memberUrl#fragment",
            "http://main.example:8080/api/projects/yilong-quant/downloads/android",
            "http://main.example:8080/api/store/projects/yilong-quant/downloads/android/",
            "https://other.example/download/latest.apk",
        ).forEach { url ->
            assertEquals(url, memberUrl, target(url, "yilong-quant", "secret")?.url)
        }
        assertNull(target("ftp://main.example/download/latest.apk", "yilong-quant", null))
    }

    @Test fun officialQuantRequiresATrustedAbsoluteHttpOrHttpsServer() {
        val secureUrl = memberUrl.replace("http://", "https://")
        assertEquals(
            secureUrl,
            resolveProjectApkDownloadTarget(
                secureUrl,
                "yilong-quant",
                "member-token",
                memberServer.replace("http://", "https://"),
            )?.url,
        )

        listOf(
            "/api/store/projects/yilong-quant/downloads/android",
            "file:///api/store/projects/yilong-quant/downloads/android",
        ).forEach { url ->
            assertNull(url, target(url, "yilong-quant", "member-token"))
        }
        assertEquals(
            memberUrl,
            target(
                "http:///api/store/projects/yilong-quant/downloads/android",
                "yilong-quant",
                "member-token",
            )?.url,
        )
        assertEquals(
            memberUrl,
            target(
                "http://127.0.0.1:8080/api/projects/yilong-quant/download/latest.apk",
                "yilong-quant",
                "member-token",
            )?.url,
        )
        listOf(
            "ftp://main.example:8080",
            "http://user:pass@main.example:8080",
            "http://main.example:8080/base",
            "http://main.example:8080?token=secret",
        ).forEach { server ->
            assertNull(resolveProjectApkDownloadTarget(memberUrl, "yilong-quant", "member-token", server))
        }
    }

    @Test fun onlyTheStableProjectIdGetsTheProtectedBranch() {
        listOf(null, "", "YILONG-QUANT", "yilong-quant-copy", "一龙量化交易").forEach { id ->
            assertNull(target(memberUrl, id, null))
            val authenticated = target(memberUrl, id, "member token")
            assertTrue(authenticated?.isolatedClient == false)
            assertNull(authenticated?.bearerToken)
            assertTrue(authenticated?.url.orEmpty().endsWith("?token=member+token"))
        }
    }

    @Test fun privateProjectsStillRequireAndAppendMemberToken() {
        val privateUrl = "https://downloads.example/project.apk"
        assertNull(target(privateUrl, "private-project", null))
        assertNull(target(privateUrl, "private-project", "  "))

        val downloadTarget = target(privateUrl, "private-project", "member token")
        assertEquals("https://downloads.example/project.apk?token=member+token", downloadTarget?.url)
        assertFalse(downloadTarget?.isolatedClient ?: true)
        assertNull(downloadTarget?.bearerToken)
    }

    @Test fun protectedOfficialDownloadsUseAnIsolatedClientWithoutRedirectsOrInterceptors() {
        val authenticatedClient = OkHttpClient.Builder()
            .addInterceptor { chain ->
                chain.proceed(
                    chain.request().newBuilder()
                        .header("Authorization", "Bearer member-secret")
                        .build(),
                )
            }
            .build()
        val memberTarget = requireNotNull(
            target(memberUrl, "yilong-quant", "member-secret"),
        )
        val memberClient = projectApkDownloadClient(authenticatedClient, memberTarget)

        assertNotSame(authenticatedClient, memberClient)
        assertTrue(memberClient.interceptors.isEmpty())
        assertTrue(memberClient.networkInterceptors.isEmpty())
        assertFalse(memberClient.followRedirects)
        assertFalse(memberClient.followSslRedirects)

        val privateTarget = requireNotNull(
            target(
                "https://downloads.example/project.apk",
                "private-project",
                "member-secret",
            ),
        )
        assertSame(authenticatedClient, projectApkDownloadClient(authenticatedClient, privateTarget))
    }

    @Test fun officialMemberTokenUsesTheAuthorizationHeaderOnly() {
        val request = projectApkDownloadRequest(memberUrl, "member-secret")

        assertEquals("Bearer member-secret", request.header("Authorization"))
        assertFalse(request.url.toString().contains("member-secret"))
        assertNull(projectApkDownloadRequest(memberUrl, null).header("Authorization"))
    }
}
