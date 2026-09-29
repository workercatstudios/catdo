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
    primary = Color(0xFF262626), onPrimary = Color.White,
    primaryContainer = Color(0xFFECECE8), onPrimaryContainer = Color(0xFF232323),
    secondary = Color(0xFF6B6B67), onSecondary = Color.White,
    secondaryContainer = Color(0xFFECECE8), onSecondaryContainer = Color(0xFF232323),
    tertiary = Color(0xFF6B6B67), onTertiary = Color.White,
    tertiaryContainer = Color(0xFFF6F6F4), onTertiaryContainer = Color(0xFF232323),
    background = Color(0xFFFFFFFF), onBackground = Color(0xFF232323),
    surface = Color(0xFFFFFFFF), onSurface = Color(0xFF232323),
    surfaceVariant = Color(0xFFF6F6F4), onSurfaceVariant = Color(0xFF6B6B67),
    surfaceContainerLowest = Color.White, surfaceContainerLow = Color(0xFFF6F6F4),
    surfaceContainer = Color(0xFFF6F6F4), surfaceContainerHigh = Color(0xFFEFEFEC),
    surfaceContainerHighest = Color(0xFFE7E7E3),
    outline = Color(0xFFE7E7E3), outlineVariant = Color(0xFFE7E7E3),
    inverseSurface = Color(0xFF232323), inverseOnSurface = Color(0xFFFFFFFF),
    inversePrimary = Color(0xFFEDEDEA),
    error = Color(0xFFAD3F3C), onError = Color.White,
    errorContainer = Color(0xFFF8E7E4), onErrorContainer = Color(0xFF7F2928),
)

private val dark = darkColorScheme(
    primary = Color(0xFFEDEDEA), onPrimary = Color(0xFF1B1B1A),
    primaryContainer = Color(0xFF2C2C2A), onPrimaryContainer = Color(0xFFEDEDEA),
    secondary = Color(0xFFA3A39F), onSecondary = Color(0xFF171716),
    secondaryContainer = Color(0xFF2C2C2A), onSecondaryContainer = Color(0xFFEDEDEA),
    tertiary = Color(0xFFA3A39F), onTertiary = Color(0xFF171716),
    tertiaryContainer = Color(0xFF333331), onTertiaryContainer = Color(0xFFEDEDEA),
    background = Color(0xFF1B1B1A), onBackground = Color(0xFFEDEDEA),
    surface = Color(0xFF1B1B1A), onSurface = Color(0xFFEDEDEA),
    surfaceVariant = Color(0xFF232322), onSurfaceVariant = Color(0xFFA3A39F),
    surfaceContainerLowest = Color(0xFF131312), surfaceContainerLow = Color(0xFF171716),
    surfaceContainer = Color(0xFF232322), surfaceContainerHigh = Color(0xFF2A2A29),
    surfaceContainerHighest = Color(0xFF333331),
    outline = Color(0xFF333331), outlineVariant = Color(0xFF333331),
    inverseSurface = Color(0xFFEDEDEA), inverseOnSurface = Color(0xFF1B1B1A),
    inversePrimary = Color(0xFF262626),
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

/** Meaningful colour: each view, date kind, and project has its own hue. */
object Accents {
    @Composable fun today() = pick(Color(0xFF946A2A), Color(0xFFD9A860))
    @Composable fun inbox() = pick(Color(0xFF3B6FB6), Color(0xFF8FB4E8))
    @Composable fun upcoming() = pick(Color(0xFF6B5BB5), Color(0xFFB3A6E8))
    @Composable fun calendar() = pick(Color(0xFF25736A), Color(0xFF86CDBF))
    @Composable fun done() = pick(Color(0xFF3E7A4F), Color(0xFF9ACB9F))
    @Composable fun repeat() = calendar()
    @Composable fun scheduled() = inbox()

    private val lightProjects = listOf(
        Color(0xFF3B6FB6), Color(0xFF6B5BB5), Color(0xFF2F8A7D), Color(0xFFB1843D),
        Color(0xFFB5486A), Color(0xFFC2622F), Color(0xFF3E7A4F), Color(0xFF5A6B7A),
    )
    private val darkProjects = listOf(
        Color(0xFF8FB4E8), Color(0xFFB3A6E8), Color(0xFF86CDBF), Color(0xFFD9A860),
        Color(0xFFE7A0B9), Color(0xFFEBA37A), Color(0xFF9ACB9F), Color(0xFF9FB0BF),
    )

    /** Stable per-project colour, using the same hash as the web and desktop clients. */
    @Composable fun project(id: String): Color {
        var hash = 0
        for (ch in id) hash = hash * 31 + ch.code
        val index = ((hash.toLong() and 0xFFFFFFFFL) % lightProjects.size).toInt()
        return if (isSystemInDarkTheme()) darkProjects[index] else lightProjects[index]
    }

    @Composable private fun pick(light: Color, dark: Color) = if (isSystemInDarkTheme()) dark else light
}

@Composable
fun CatDoTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) dark else light, shapes = shapes, typography = typography, content = content)
}
