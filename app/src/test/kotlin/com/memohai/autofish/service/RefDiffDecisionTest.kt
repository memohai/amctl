package com.memohai.autofish.service

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class RefDiffDecisionTest {
    @Test
    fun `should rebuild when no cached refs`() {
        assertTrue(
            shouldRebuildRefs(
                hasCachedRefs = false,
                uiSeqChanged = false,
                nowMs = 1_000L,
                lastComputedAtMs = 900L,
                forceIntervalMs = 1_500L,
            ),
        )
    }

    @Test
    fun `should rebuild when ui sequence changed`() {
        assertTrue(
            shouldRebuildRefs(
                hasCachedRefs = true,
                uiSeqChanged = true,
                nowMs = 1_000L,
                lastComputedAtMs = 900L,
                forceIntervalMs = 1_500L,
            ),
        )
    }

    @Test
    fun `should rebuild when force interval elapsed`() {
        assertTrue(
            shouldRebuildRefs(
                hasCachedRefs = true,
                uiSeqChanged = false,
                nowMs = 2_500L,
                lastComputedAtMs = 1_000L,
                forceIntervalMs = 1_500L,
            ),
        )
    }

    @Test
    fun `should not rebuild when cache valid and interval not elapsed`() {
        assertFalse(
            shouldRebuildRefs(
                hasCachedRefs = true,
                uiSeqChanged = false,
                nowMs = 1_800L,
                lastComputedAtMs = 1_000L,
                forceIntervalMs = 1_500L,
            ),
        )
    }
}
