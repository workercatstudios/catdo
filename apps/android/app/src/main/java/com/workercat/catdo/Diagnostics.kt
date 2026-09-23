package com.workercat.catdo

import com.google.firebase.crashlytics.FirebaseCrashlytics

/** Small, fixed event names only. Never send credentials, task content, or account IDs. */
object Diagnostics {
    fun event(name: String) {
        runCatching { FirebaseCrashlytics.getInstance().log(name) }
    }

    fun failure(stage: String, error: Throwable) {
        runCatching {
            FirebaseCrashlytics.getInstance().apply {
                log(stage)
                recordException(error)
            }
        }
    }
}
