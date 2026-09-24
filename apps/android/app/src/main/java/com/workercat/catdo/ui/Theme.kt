package com.workercat.catdo.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val light = lightColorScheme(
    primary = Color(0xFF3E624E), onPrimary = Color.White,
    primaryContainer = Color(0xFFE9EFE9), onPrimaryContainer = Color(0xFF242824),
    secondary = Color(0xFF696F66), onSecondary = Color.White,
    secondaryContainer = Color(0xFFE9EFE9), onSecondaryContainer = Color(0xFF242824),
    tertiary = Color(0xFF696F66), onTertiary = Color.White,
    tertiaryContainer = Color(0xFFF7F7F5), onTertiaryContainer = Color(0xFF242824),
    background = Color(0xFFFFFFFF), onBackground = Color(0xFF242824),
    surface = Color(0xFFFFFFFF), onSurface = Color(0xFF242824),
    surfaceVariant = Color(0xFFF7F7F5), onSurfaceVariant = Color(0xFF696F66),
    surfaceContainerLowest = Color.White, surfaceContainerLow = Color(0xFFF7F7F5),
    surfaceContainer = Color(0xFFF7F7F5), surfaceContainerHigh = Color(0xFFF0F2ED),
    surfaceContainerHighest = Color(0xFFE8EAE5),
    outline = Color(0xFFE8EAE5), outlineVariant = Color(0xFFE8EAE5),
    inverseSurface = Color(0xFF242824), inverseOnSurface = Color(0xFFFFFFFF),
    inversePrimary = Color(0xFFACCCB1),
    error = Color(0xFFAD3F3C), onError = Color.White,
    errorContainer = Color(0xFFF8E7E4), onErrorContainer = Color(0xFF7F2928),
)

private val dark = darkColorScheme(
    primary = Color(0xFFACCCB1), onPrimary = Color(0xFF1C201D),
    primaryContainer = Color(0xFF2C3E30), onPrimaryContainer = Color(0xFFEDF0EA),
    secondary = Color(0xFFA2AAA0), onSecondary = Color(0xFF181B19),
    secondaryContainer = Color(0xFF2C3E30), onSecondaryContainer = Color(0xFFEDF0EA),
    tertiary = Color(0xFFA2AAA0), onTertiary = Color(0xFF181B19),
    tertiaryContainer = Color(0xFF333B34), onTertiaryContainer = Color(0xFFEDF0EA),
    background = Color(0xFF1C201D), onBackground = Color(0xFFEDF0EA),
    surface = Color(0xFF1C201D), onSurface = Color(0xFFEDF0EA),
    surfaceVariant = Color(0xFF242925), onSurfaceVariant = Color(0xFFA2AAA0),
    surfaceContainerLowest = Color(0xFF141815), surfaceContainerLow = Color(0xFF181B19),
    surfaceContainer = Color(0xFF242925), surfaceContainerHigh = Color(0xFF2B322C),
    surfaceContainerHighest = Color(0xFF333B34),
    outline = Color(0xFF333B34), outlineVariant = Color(0xFF333B34),
    inverseSurface = Color(0xFFEDF0EA), inverseOnSurface = Color(0xFF1C201D),
    inversePrimary = Color(0xFF3E624E),
    error = Color(0xFFE7988B), onError = Color(0xFF47201D),
    errorContainer = Color(0xFF5F302B), onErrorContainer = Color(0xFFFFDAD4),
)

private val shapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(12.dp),
    extraLarge = RoundedCornerShape(16.dp),
)

private val typography = Typography(
    headlineLarge = TextStyle(fontSize = 28.sp, lineHeight = 34.sp, fontWeight = FontWeight.SemiBold, letterSpacing = (-0.6).sp),
    headlineSmall = TextStyle(fontSize = 24.sp, lineHeight = 30.sp, fontWeight = FontWeight.SemiBold),
    titleLarge = TextStyle(fontSize = 20.sp, lineHeight = 26.sp, fontWeight = FontWeight.SemiBold),
    titleMedium = TextStyle(fontSize = 16.sp, lineHeight = 24.sp, fontWeight = FontWeight.Medium),
    bodyLarge = TextStyle(fontSize = 15.sp, lineHeight = 21.sp),
    bodyMedium = TextStyle(fontSize = 14.sp, lineHeight = 21.sp),
    bodySmall = TextStyle(fontSize = 12.sp, lineHeight = 17.sp),
    labelLarge = TextStyle(fontSize = 14.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium),
    labelMedium = TextStyle(fontSize = 12.sp, lineHeight = 18.sp, fontWeight = FontWeight.Medium),
    labelSmall = TextStyle(fontSize = 11.sp, lineHeight = 16.sp, fontWeight = FontWeight.Medium, letterSpacing = 0.5.sp),
)

@Composable
fun CatDoTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) dark else light, shapes = shapes, typography = typography, content = content)
}
