# CatDo for Android

The native Android app uses Kotlin, Jetpack Compose, Material 3, and the same JSON task and sync contract as the web and desktop clients. Tasks are saved locally first, so Today, Inbox, Upcoming, Calendar, Projects, search, editing, and completion work offline. WorkerCat sign-in uses Clerk's Android SDK and its native authentication view. Local task data remains on the device after sign-out.

## Build

Install JDK 17 or newer, Android SDK Platform 36, and Build Tools 36.0.0. From this directory:

```sh
./gradlew :app:assembleDebug
./gradlew :app:testDebugUnitTest
```

The APK is at `app/build/outputs/apk/debug/app-debug.apk`. Open this directory in Android Studio to run it on a device. No account is required for local use.

## Design and storage

- A single Compose activity draws edge to edge and follows the system light or dark appearance.
- The repository is the only writer of `catdo.json`, using Android's `AtomicFile`; UI reads it as a `StateFlow`.
- Sync freezes each upload with an operation ID before sending it. Interrupted requests retry the same ID. Whole-record three-way merge follows the existing desktop and web behavior; unresolved conflicts ask the user which side to keep.
- Clerk manages session persistence and token refresh. Local task data and app credentials are excluded from Android backup and device transfer.

Current boundaries: the app can receive FCM alerts after the user enables **Notifications** in Settings, but task reminder scheduling and a server sender are not implemented yet. Workspace/project archive controls are not exposed, and sync runs while the app is in use or when the user selects **Sync now**. The production API accepts Clerk session tokens from Android and web, and OAuth tokens from CatDo Desktop. Tagged releases include a signed APK on GitHub Releases; Google Play distribution is not configured.

Firebase project `catdo-9b516` under `coffeechopdev@gmail.com` contains the Android app `com.workercat.catdo`. Its `app/google-services.json` contains public project identifiers and is committed so local and CI builds use the same FCM sender. Firebase Messaging registration starts only after the user enables notifications and grants the Android permission; turning it off unregisters the device. A device needs Google Play services or a Google APIs emulator for FCM delivery. No FCM server credentials belong in the app or repository.

Firebase Crashlytics reports fatal crashes and selected notification and sync failures. The release build uploads its R8 mapping file through the Crashlytics Gradle plugin. Events use fixed names; the app does not set a Crashlytics user ID or add task content, account IDs, or tokens. The Firebase console does not show the first report until a device sends one. The web API logs Clerk rejection reason codes without credentials or task data.

The production WorkerCat Clerk instance needs Native API enabled and `com.workercat.catdo` registered as an Android application with the release certificate SHA-256 fingerprint. Email authentication uses Clerk's native UI. Social sign-in also needs its redirect URL allowlisted in Clerk.

Release CI reads the signing keystore and passwords from repository secrets `ANDROID_RELEASE_KEYSTORE_BASE64`, `ANDROID_RELEASE_STORE_PASSWORD`, and `ANDROID_RELEASE_KEY_PASSWORD`. The keystore must contain alias `catdo-release`. Keep an independent, secure backup of the keystore and passwords: Android updates must use the same signing key. The release APK is named `catdo-<version>-android.apk`; its version name follows the release tag, and its version code increases with each Release workflow run.

The current build uses `compileSdk 36` and Compose BOM `2026.04.01` because SDK Platform 37, required by Compose 1.12, is absent from the public Android SDK repository available in this environment. The app targets Android 16 (API 36) and supports Android 8.0 and newer.

Architecture choices follow the [Android architecture recommendations](https://developer.android.com/topic/architecture/recommendations), [offline-first guide](https://developer.android.com/topic/architecture/data-layer/offline-first), [Compose BOM guidance](https://developer.android.com/develop/ui/compose/bom), and [edge-to-edge setup](https://developer.android.com/develop/ui/compose/system/setup-e2e).
