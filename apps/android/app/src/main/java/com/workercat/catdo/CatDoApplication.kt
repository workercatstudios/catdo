package com.workercat.catdo

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import com.clerk.api.Clerk

class CatDoApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        // Remove credentials from the retired device-code flow. Task data is separate.
        deleteFile("credentials.json")
        deleteFile("credentials.json.bak")
        getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel("catdo_updates", "CatDo updates", NotificationManager.IMPORTANCE_DEFAULT)
        )
        Clerk.initialize(this, getString(R.string.clerk_publishable_key))
    }
}
