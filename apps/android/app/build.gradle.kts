plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.google.gms.google-services")
    id("com.google.firebase.crashlytics")
}

android {
    namespace = "com.workercat.catdo"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.workercat.catdo"
        minSdk = 26
        targetSdk = 36
        versionCode = providers.environmentVariable("CATDO_ANDROID_VERSION_CODE").orNull?.toInt() ?: 11
        versionName = providers.environmentVariable("CATDO_ANDROID_VERSION_NAME").orNull ?: "0.2.6"
    }

    signingConfigs {
        providers.environmentVariable("CATDO_ANDROID_KEYSTORE").orNull?.let { keystore ->
            create("release") {
                storeFile = file(keystore)
                storePassword = providers.environmentVariable("CATDO_ANDROID_STORE_PASSWORD").get()
                keyAlias = "catdo-release"
                keyPassword = providers.environmentVariable("CATDO_ANDROID_KEY_PASSWORD").get()
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            if (signingConfigs.findByName("release") != null) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    buildFeatures { compose = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // Compose 1.12 requires API 37, which is not yet present in the public SDK repository.
    val composeBom = platform("androidx.compose:compose-bom:2026.04.01")
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.10.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("com.clerk:clerk-android-ui:1.1.8")
    implementation(platform("com.google.firebase:firebase-bom:34.19.0"))
    implementation("com.google.firebase:firebase-messaging")
    implementation("com.google.firebase:firebase-crashlytics")
    debugImplementation("androidx.compose.ui:ui-tooling")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20260719")
}
