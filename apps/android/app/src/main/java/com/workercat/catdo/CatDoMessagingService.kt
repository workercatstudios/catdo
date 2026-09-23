package com.workercat.catdo

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.os.Build
import android.content.pm.PackageManager
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

// Firebase's installation ID mode calls onRegistered; the lint check still expects the legacy token callback.
@SuppressLint("MissingFirebaseInstanceTokenRefresh")
class CatDoMessagingService : FirebaseMessagingService() {
    override fun onRegistered(installationId: String) {
        Diagnostics.event("notifications_registered")
        if (applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0) {
            Log.d("CatDoFCM", "Test installation ID: $installationId")
        }
    }

    override fun onMessageReceived(message: RemoteMessage) {
        val notification = message.notification ?: return
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) return
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val openApp = PendingIntent.getActivity(
            this, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val alert = NotificationCompat.Builder(this, "catdo_updates")
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(notification.title ?: getString(R.string.app_name))
            .setContentText(notification.body.orEmpty())
            .setStyle(NotificationCompat.BigTextStyle().bigText(notification.body.orEmpty()))
            .setContentIntent(openApp)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(this).notify(message.messageId?.hashCode() ?: System.nanoTime().toInt(), alert)
    }
}
