package com.memohai.autofish.service

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class ServiceServerObserveTest {

    @Test
    fun `stableObservedTopActivity returns stable value`() {
        assertEquals(
            "com.android.settings/.Settings",
            stableObservedTopActivity(
                "com.android.settings/.Settings",
                "com.android.settings/.Settings",
            ),
        )
    }

    @Test
    fun `stableObservedTopActivity returns null on mismatch or missing values`() {
        assertNull(stableObservedTopActivity("com.a/.Main", "com.b/.Main"))
        assertNull(stableObservedTopActivity(null, "com.a/.Main"))
        assertNull(stableObservedTopActivity("com.a/.Main", null))
    }
}
