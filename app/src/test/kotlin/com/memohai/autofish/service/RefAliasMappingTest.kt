package com.memohai.autofish.service

import com.memohai.autofish.services.accessibility.BoundsData
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class RefAliasMappingTest {
    @Test
    fun `build observed map should persist ref to token mapping`() {
        val refs = listOf(
            refNode(ref = "@n1", nodeId = "node-a"),
            refNode(ref = "@n2", nodeId = "node-b"),
        )
        val map = buildObservedRefTokenMap(
            refs = refs,
            exactTokenBuilder = { "ex-${it.nodeId}" },
            identityTokenBuilder = { "id-${it.nodeId}" },
        )

        assertEquals("ex-node-a", map["@n1"]?.exactToken)
        assertEquals("id-node-a", map["@n1"]?.identityToken)
        assertEquals("ex-node-b", map["@n2"]?.exactToken)
        assertEquals("id-node-b", map["@n2"]?.identityToken)
    }

    @Test
    fun `resolve ref should return recorded token when alias exists`() {
        val observed = mapOf(
            "@n1" to RefAliasToken(exactToken = "ex-node-a", identityToken = "id-node-a"),
        )

        assertEquals("ex-node-a", resolveRecordedTokenForRef("@n1", observed)?.exactToken)
        assertEquals("id-node-a", resolveRecordedTokenForRef("@n1", observed)?.identityToken)
        assertNull(resolveRecordedTokenForRef("@n2", observed))
    }

    @Test
    fun `find by token should work when ref alias is reordered without layout change`() {
        val previous = listOf(
            refNode(ref = "@n1", nodeId = "node-a"),
            refNode(ref = "@n2", nodeId = "node-b"),
        )
        val observed = buildObservedRefTokenMap(
            refs = previous,
            exactTokenBuilder = { "ex-${it.nodeId}" },
            identityTokenBuilder = { "id-${it.nodeId}" },
        )
        val recordedToken = resolveRecordedTokenForRef("@n1", observed)
        assertNotNull(recordedToken)

        val current = listOf(
            refNode(ref = "@n1", nodeId = "node-b"),
            refNode(ref = "@n2", nodeId = "node-a"),
        )
        val target = findRefNodeByToken(
            refs = current,
            token = recordedToken!!,
            exactTokenBuilder = { "ex-${it.nodeId}" },
            identityTokenBuilder = { "id-${it.nodeId}" },
        )
        assertNotNull(target)
        assertEquals("node-a", target!!.nodeId)
        assertEquals("@n2", target.ref)
    }

    @Test
    fun `find by token should recover by identity when layout changed`() {
        val previous = listOf(
            refNode(ref = "@n4", nodeId = "node-a", left = 0, top = 100, right = 100, bottom = 200, text = "Airplane mode"),
        )
        val observed = buildObservedRefTokenMap(
            refs = previous,
            exactTokenBuilder = { "ex-${it.className}-${it.text}-${it.bounds.left},${it.bounds.top}" },
            identityTokenBuilder = { "id-${it.className}-${it.text}" },
        )
        val recordedToken = resolveRecordedTokenForRef("@n4", observed)
        assertNotNull(recordedToken)

        val current = listOf(
            refNode(ref = "@n5", nodeId = "node-a-new-pos", left = 0, top = 260, right = 100, bottom = 360, text = "Airplane mode"),
        )
        val target = findRefNodeByToken(
            refs = current,
            token = recordedToken!!,
            exactTokenBuilder = { "ex-${it.className}-${it.text}-${it.bounds.left},${it.bounds.top}" },
            identityTokenBuilder = { "id-${it.className}-${it.text}" },
        )
        assertNotNull(target)
        assertEquals("@n5", target!!.ref)
    }

    @Test
    fun `find by token should return null when identity match is ambiguous`() {
        val token = RefAliasToken(exactToken = "ex-old", identityToken = "id-item")
        val current = listOf(
            refNode(ref = "@n3", nodeId = "node-a", text = "Airplane mode"),
            refNode(ref = "@n8", nodeId = "node-b", text = "Airplane mode"),
        )
        val target = findRefNodeByToken(
            refs = current,
            token = token,
            exactTokenBuilder = { "ex-${it.nodeId}" },
            identityTokenBuilder = { "id-item" },
        )
        assertNull(target)
    }

    @Test
    fun `find by token should return null when recorded token is stale`() {
        val token = RefAliasToken(exactToken = "ex-node-a", identityToken = "id-node-a")
        val current = listOf(
            refNode(ref = "@n1", nodeId = "node-b"),
        )
        val target = findRefNodeByToken(
            refs = current,
            token = token,
            exactTokenBuilder = { "ex-${it.nodeId}" },
            identityTokenBuilder = { "id-${it.nodeId}" },
        )
        assertNull(target)
    }

    private fun refNode(
        ref: String,
        nodeId: String,
        left: Int = 0,
        top: Int = 0,
        right: Int = 10,
        bottom: Int = 10,
        text: String? = null,
    ): ServiceServer.RefNode =
        ServiceServer.RefNode(
            ref = ref,
            nodeId = nodeId,
            className = "android.view.View",
            text = text,
            desc = null,
            resId = null,
            bounds = BoundsData(left = left, top = top, right = right, bottom = bottom),
            clickable = true,
            longClickable = false,
            editable = false,
            scrollable = false,
            enabled = true,
            visible = true,
            focused = true,
            windowLayer = 0,
        )
}
