package com.workercat.catdo.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

val Sage = Color(0xFF78916E)
val DeepSage = Color(0xFF486542)
val WarmPaper = Color(0xFFF7F7F3)
val Ink = Color(0xFF252923)
val MutedInk = Color(0xFF71766E)
val Deadline = Color(0xFFAE634D)

private val light = lightColorScheme(
    primary = DeepSage, onPrimary = Color.White,
    primaryContainer = Color(0xFFE1EADD), onPrimaryContainer = Color(0xFF244420),
    secondary = Color(0xFF6D766A), onSecondary = Color.White,
    background = WarmPaper, onBackground = Ink,
    surface = Color.White, onSurface = Ink,
    surfaceVariant = Color(0xFFF0F1EC), onSurfaceVariant = MutedInk,
    outline = Color(0xFFD7DCD2), error = Deadline,
)
private val dark = darkColorScheme(
    primary = Color(0xFFA9C89E), onPrimary = Color(0xFF20351E),
    primaryContainer = Color(0xFF344D31), onPrimaryContainer = Color(0xFFD7ECD0),
    secondary = Color(0xFFB9C6B3), onSecondary = Color(0xFF293327),
    background = Color(0xFF171B18), onBackground = Color(0xFFE7EAE3),
    surface = Color(0xFF212721), onSurface = Color(0xFFE7EAE3),
    surfaceVariant = Color(0xFF2C342D), onSurfaceVariant = Color(0xFFADB8AA),
    outline = Color(0xFF4A554B), error = Color(0xFFEAA78F),
)

@Composable
fun CatDoTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) dark else light, content = content)
}
