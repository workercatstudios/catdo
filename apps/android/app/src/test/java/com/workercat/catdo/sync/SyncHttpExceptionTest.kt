package com.workercat.catdo.sync

import org.junit.Assert.*
import org.junit.Test

class SyncHttpExceptionTest {
    @Test fun termsRejectionOffersAccountSpecificRecovery() {
        val error = SyncHttpException(428, "sync")
        assertTrue(error.requiresTerms)
        assertTrue(error.message!!.contains("this CatDo account"))
        assertTrue(error.message!!.contains("Your tasks are saved here"))
        assertEquals("https://catdo.workercat.com/app", TERMS_REVIEW_URL)
    }

    @Test fun otherFailuresDoNotAskForTermsAcceptance() {
        for (status in listOf(401, 403, 413, 503)) {
            assertFalse(SyncHttpException(status, "sync").requiresTerms)
        }
        assertTrue(SyncHttpException(401, "identity").message!!.contains("Sign-in was rejected"))
    }
}
