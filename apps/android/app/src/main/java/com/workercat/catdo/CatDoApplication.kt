package com.workercat.catdo

import android.app.Application
import com.clerk.api.Clerk

class CatDoApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        // Remove credentials from the retired device-code flow. Task data is separate.
        deleteFile("credentials.json")
        deleteFile("credentials.json.bak")
        Clerk.initialize(this, getString(R.string.clerk_publishable_key))
    }
}
