package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProjectPlazaInstallContractTest {
    @Test
    fun parsesAndCachesMarketplaceInstallAction() {
        val project = parseStoreProject(JSONObject(PROJECT_JSON))
        assertEquals("erp_blueprint", project.installAction?.kind)
        assertEquals("创建我的店铺", project.installAction?.label)

        val cached = parseStoreProject(project.toProjectPlazaCacheJson())
        assertEquals(project.installAction, cached.installAction)
    }

    @Test
    fun missingInstallKindIsIgnored() {
        assertNull(parseStoreProjectInstallAction(JSONObject("""{"label":"创建"}""")))
    }

    @Test
    fun parsesCreatedInstanceWithoutDependingOnRouteShape() {
        val result = parseMarketplaceErpInstanceResult(
            JSONObject(
                """{
                    "source_project_id":"cofficethinking",
                    "instance":{"project_id":"merchant-project-1"},
                    "target_route":"/projects/merchant-project-1?tab=openCommerce&commerce=erp"
                }"""
            ),
            "钱一龙咖啡店"
        )
        assertEquals("cofficethinking", result.sourceProjectId)
        assertEquals("merchant-project-1", result.projectId)
        assertEquals("钱一龙咖啡店", result.projectName)
    }

    private companion object {
        val PROJECT_JSON = """{
            "id":"cofficethinking",
            "name":"cofficethinking",
            "display_name":"一龙商户经营系统",
            "description":"独立店铺项目",
            "template":"local",
            "owner_account":"钱一龙",
            "member_count":1,
            "is_public":true,
            "join_mode":"readonly",
            "install_action":{"kind":"erp_blueprint","label":"创建我的店铺"}
        }"""
    }
}
