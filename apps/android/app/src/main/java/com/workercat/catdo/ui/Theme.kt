package com.workercat.catdo.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val light = lightColorScheme(
    primary = Color(0xFF36332F), onPrimary = Color.White,
    primaryContainer = Color(0xFFEAE6E0), onPrimaryContainer = Color(0xFF302E2B),
    secondary = Color(0xFF706B64), onSecondary = Color.White,
    secondaryContainer = Color(0xFFF4F2EF), onSecondaryContainer = Color(0xFF302E2B),
    tertiary = Color(0xFF706B64), onTertiary = Color.White,
    tertiaryContainer = Color(0xFFF4F2EF), onTertiaryContainer = Color(0xFF302E2B),
    background = Color(0xFFFDFCFB), onBackground = Color(0xFF302E2B),
    surface = Color(0xFFFDFCFB), onSurface = Color(0xFF302E2B),
    surfaceVariant = Color(0xFFF4F2EF), onSurfaceVariant = Color(0xFF706B64),
    surfaceContainerLowest = Color.White, surfaceContainerLow = Color(0xFFF8F6F3),
    surfaceContainer = Color(0xFFF4F2EF), surfaceContainerHigh = Color(0xFFEEEAE5),
    surfaceContainerHighest = Color(0xFFEAE6E0),
    outline = Color(0xFFE3DFD9), outlineVariant = Color(0xFFE3DFD9),
    inverseSurface = Color(0xFF302E2B), inverseOnSurface = Color(0xFFFDFCFB),
    inversePrimary = Color(0xFFE1D9CD),
    error = Color(0xFFAD3F3C), onError = Color.White,
    errorContainer = Color(0xFFF8E7E4), onErrorContainer = Color(0xFF7F2928),
)

private val dark = darkColorScheme(
    primary = Color(0xFFE1D9CD), onPrimary = Color(0xFF252320),
    primaryContainer = Color(0xFF3C3832), onPrimaryContainer = Color(0xFFEEEAE4),
    secondary = Color(0xFFB6AFA5), onSecondary = Color(0xFF252320),
    secondaryContainer = Color(0xFF3C3832), onSecondaryContainer = Color(0xFFEEEAE4),
    tertiary = Color(0xFFB6AFA5), onTertiary = Color(0xFF252320),
    tertiaryContainer = Color(0xFF3C3832), onTertiaryContainer = Color(0xFFEEEAE4),
    background = Color(0xFF201F1D), onBackground = Color(0xFFEEEAE4),
    surface = Color(0xFF201F1D), onSurface = Color(0xFFEEEAE4),
    surfaceVariant = Color(0xFF292724), onSurfaceVariant = Color(0xFFB6AFA5),
    surfaceContainerLowest = Color(0xFF1B1A18), surfaceContainerLow = Color(0xFF252320),
    surfaceContainer = Color(0xFF292724), surfaceContainerHigh = Color(0xFF332F2B),
    surfaceContainerHighest = Color(0xFF3C3832),
    outline = Color(0xFF403C36), outlineVariant = Color(0xFF403C36),
    inverseSurface = Color(0xFFEEEAE4), inverseOnSurface = Color(0xFF201F1D),
    inversePrimary = Color(0xFF36332F),
    error = Color(0xFFE7988B), onError = Color(0xFF47201D),
    errorContainer = Color(0xFF5F302B), onErrorContainer = Color(0xFFFFDAD4),
)

@Composable
fun CatDoTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) dark else light, content = content)
}
