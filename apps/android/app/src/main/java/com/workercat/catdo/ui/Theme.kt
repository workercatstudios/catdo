package com.workercat.catdo.ui

import android.provider.Settings
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.LifecycleResumeEffect
import com.workercat.catdo.R

/*
 * The Kirakira UI theme (https://kk.workercat.com), mapped onto Material 3 roles.
 * Light is warm paper and cocoa ink with a pink accent; dark is navy with a lighter pink.
 * The values are the `kirakira-theme` registry item's, so Android matches the web and desktop.
 */

/** The Kirakira tokens that have no exact Material 3 role. */
@Immutable
data class KirakiraColors(
    val paper: Color,
    val card: Color,
    val ink: Color,
    val muted: Color,
    val mutedInk: Color,
    val border: Color,
    val input: Color,
    val blush: Color,
    val secondary: Color,
    val primary: Color,
    val onPrimary: Color,
    val destructive: Color,
    /** The page's ink for marks drawn beside a character (the z's), light on navy. */
    val markInk: Color,
    /** The drawer and rail surface, a step off the page. */
    val sidebar: Color,
    val dark: Boolean,
)

private val kirakiraLight = KirakiraColors(
    paper = Color(0xFFFFFCF8), card = Color.White, ink = Color(0xFF4B3832),
    muted = Color(0xFFF7F0EA), mutedInk = Color(0xFF7A655D), border = Color(0xFFF0E4DB), input = Color(0xFFF0E4DB),
    blush = Color(0xFFFDE6EE), secondary = Color(0xFFE4F4F2), primary = Color(0xFFD6336F), onPrimary = Color.White,
    destructive = Color(0xFFE5484D), markInk = Color(0xFF4B3832), sidebar = Color(0xFFFBF6F1), dark = false,
)

private val kirakiraDark = KirakiraColors(
    paper = Color(0xFF1F2238), card = Color(0xFF272B45), ink = Color(0xFFF6ECE4),
    muted = Color(0xFF2A2E48), mutedInk = Color(0xFFB3A8B8), border = Color(0xFF383C5C), input = Color(0xFF383C5C),
    blush = Color(0xFF3B2F4A), secondary = Color(0xFF2F3D4F), primary = Color(0xFFFF7AA5), onPrimary = Color(0xFF1F2238),
    destructive = Color(0xFFFF6B6F), markInk = Color(0xFFF6ECE4), sidebar = Color(0xFF1B1E33), dark = true,
)

val LocalKirakira = staticCompositionLocalOf { kirakiraLight }

/** True when Android's animator duration scale is off: keep fades and instant state, drop movement. */
val LocalReducedMotion = staticCompositionLocalOf { false }

object Kirakira {
    val colors: KirakiraColors @Composable get() = LocalKirakira.current
    val reducedMotion: Boolean @Composable get() = LocalReducedMotion.current
}

private val light = lightColorScheme(
    primary = kirakiraLight.primary, onPrimary = Color.White,
    primaryContainer = kirakiraLight.blush, onPrimaryContainer = kirakiraLight.ink,
    inversePrimary = kirakiraDark.primary,
    secondary = kirakiraLight.mutedInk, onSecondary = Color.White,
    secondaryContainer = kirakiraLight.secondary, onSecondaryContainer = kirakiraLight.ink,
    tertiary = Color(0xFF2D3F63), onTertiary = Color.White,
    tertiaryContainer = Color(0xFFE6EBF5), onTertiaryContainer = Color(0xFF2D3F63),
    background = kirakiraLight.paper, onBackground = kirakiraLight.ink,
    surface = kirakiraLight.paper, onSurface = kirakiraLight.ink,
    surfaceVariant = kirakiraLight.muted, onSurfaceVariant = kirakiraLight.mutedInk,
    surfaceTint = kirakiraLight.primary,
    surfaceBright = Color.White, surfaceDim = kirakiraLight.border,
    surfaceContainerLowest = Color.White, surfaceContainerLow = Color(0xFFFBF6F1),
    surfaceContainer = Color(0xFFFAF4EE), surfaceContainerHigh = kirakiraLight.muted,
    surfaceContainerHighest = kirakiraLight.border,
    outline = Color(0xFFA8948A), outlineVariant = kirakiraLight.border,
    inverseSurface = kirakiraLight.ink, inverseOnSurface = kirakiraLight.paper,
    // Destructive #e5484d is the fill; text on paper needs a deeper red for 4.5:1.
    error = Color(0xFFC2333A), onError = Color.White,
    errorContainer = Color(0xFFFCE4E3), onErrorContainer = Color(0xFF8F1B20),
    scrim = Color.Black,
)

private val dark = darkColorScheme(
    primary = kirakiraDark.primary, onPrimary = kirakiraDark.onPrimary,
    primaryContainer = kirakiraDark.blush, onPrimaryContainer = kirakiraDark.ink,
    inversePrimary = kirakiraLight.primary,
    secondary = kirakiraDark.mutedInk, onSecondary = kirakiraDark.paper,
    secondaryContainer = kirakiraDark.secondary, onSecondaryContainer = kirakiraDark.ink,
    tertiary = Color(0xFF9AAAD6), onTertiary = kirakiraDark.paper,
    tertiaryContainer = Color(0xFF2F3A5C), onTertiaryContainer = kirakiraDark.ink,
    background = kirakiraDark.paper, onBackground = kirakiraDark.ink,
    surface = kirakiraDark.paper, onSurface = kirakiraDark.ink,
    surfaceVariant = kirakiraDark.muted, onSurfaceVariant = kirakiraDark.mutedInk,
    surfaceTint = kirakiraDark.primary,
    surfaceBright = Color(0xFF30355A), surfaceDim = Color(0xFF1A1D30),
    surfaceContainerLowest = Color(0xFF1A1D30), surfaceContainerLow = Color(0xFF1B1E33),
    surfaceContainer = kirakiraDark.card, surfaceContainerHigh = kirakiraDark.muted,
    surfaceContainerHighest = kirakiraDark.border,
    outline = Color(0xFF6A6F96), outlineVariant = kirakiraDark.border,
    inverseSurface = kirakiraDark.ink, inverseOnSurface = kirakiraDark.paper,
    error = Color(0xFFFF8A8D), onError = kirakiraDark.paper,
    errorContainer = Color(0xFF4A2533), onErrorContainer = Color(0xFFFFD9DA),
    scrim = Color.Black,
)

// Kirakira's radius is 1rem; sm, md, lg, xl and 2xl step around it.
private val shapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(24.dp),
)

/**
 * M PLUS 1, the rounded Japanese sans Kirakira sets everything in: Latin-subset static weights
 * shared with the desktop app (assets/fonts). Other scripts fall back to the system font, and
 * SemiBold resolves to Bold.
 */
private val mPlus1 = FontFamily(
    Font(R.font.m_plus_1_regular, FontWeight.Normal),
    Font(R.font.m_plus_1_medium, FontWeight.Medium),
    Font(R.font.m_plus_1_bold, FontWeight.Bold),
    Font(R.font.m_plus_1_extrabold, FontWeight.ExtraBold),
)

private fun style(size: Float, line: Float, weight: Int, tracking: Float = 0f) = TextStyle(
    fontFamily = mPlus1, fontSize = size.sp, lineHeight = line.sp, fontWeight = FontWeight(weight), letterSpacing = tracking.sp,
)

// Body text is medium (500), like the web's `font-medium`; headings are heavy and friendly.
private val typography = Typography(
    displaySmall = style(36f, 42f, 800, -0.4f),
    headlineLarge = style(30f, 36f, 800, -0.3f),
    headlineMedium = style(26f, 32f, 800, -0.2f),
    headlineSmall = style(22f, 28f, 800),
    titleLarge = style(20f, 26f, 800),
    titleMedium = style(16f, 22f, 700),
    titleSmall = style(14f, 20f, 700),
    bodyLarge = style(15.5f, 22f, 500),
    bodyMedium = style(14f, 20f, 500),
    bodySmall = style(12.5f, 17f, 500),
    labelLarge = style(14f, 20f, 700),
    labelMedium = style(12f, 16f, 700),
    labelSmall = style(11f, 15f, 700, 0.4f),
)

/** One meaning colour: the pastel fill (tiles, tints), a mark (icons, dots) and a text-safe tone (4.5:1). */
@Immutable
data class Hue(val fill: Color, val mark: Color, val text: Color)

private fun hue(fill: Long, text: Long = fill, mark: Long = text) = Hue(Color(fill), Color(mark), Color(text))

/**
 * Meaningful colour, shared with the web and desktop clients: each view, date kind, and project
 * has its own Kirakira hue. Fills are the published kk hues; text tones keep 4.5:1 on paper and navy.
 */
object Accents {
    private val light = mapOf(
        "today" to hue(0xFFF7D35C, 0xFFA35A12),
        "inbox" to hue(0xFF5AA9E6, 0xFF2F6FB0),
        "upcoming" to hue(0xFFB79AD1, 0xFF7A52A3),
        "calendar" to hue(0xFF4FB0AA, 0xFF2B7A75),
        "done" to hue(0xFFEC5F8F, 0xFFD6336F),
        "projects" to hue(0xFFF4A35F, 0xFFB5590C),
        "search" to hue(0xFF2D3F63, 0xFF2D3F63),
        "overdue" to hue(0xFFE5484D, 0xFFC2333A),
    )
    private val dark = mapOf(
        "today" to hue(0xFFF7D35C, 0xFFF7D35C),
        "inbox" to hue(0xFF5AA9E6, 0xFF8CC4F0),
        "upcoming" to hue(0xFFB79AD1, 0xFFC9B0E3),
        "calendar" to hue(0xFF4FB0AA, 0xFF7FD0C9),
        "done" to hue(0xFFFF7AA5, 0xFFFF7AA5),
        "projects" to hue(0xFFF4A35F, 0xFFF4A35F),
        "search" to hue(0xFF8FA6D6, 0xFF8FA6D6),
        "overdue" to hue(0xFFFF6B6F, 0xFFFF8A8D),
    )

    @Composable fun hue(name: String): Hue = (if (Kirakira.colors.dark) dark else light).getValue(name)

    @Composable fun today() = hue("today")
    @Composable fun inbox() = hue("inbox")
    @Composable fun upcoming() = hue("upcoming")
    @Composable fun calendar() = hue("calendar")
    @Composable fun projects() = hue("projects")
    @Composable fun search() = hue("search")
    @Composable fun overdue() = hue("overdue")
    /** A finished task fills pink, the one accent. Never green. */
    @Composable fun done() = hue("done")
    @Composable fun repeat() = calendar()
    @Composable fun scheduled() = inbox()

    /** Kirakira's own hues, for decoration (sparkles, bursts, the cat). */
    @Composable fun kk(name: String): Color = when (name) {
        "pink" -> if (Kirakira.colors.dark) Color(0xFFFF7AA5) else Color(0xFFEC5F8F)
        "yellow" -> Color(0xFFF7D35C)
        "orange" -> Color(0xFFF4A35F)
        "sky" -> Color(0xFF5AA9E6)
        "lilac" -> Color(0xFFB79AD1)
        else -> Color(0xFF4FB0AA)
    }

    // The eight project slots shared by every client (p0..p7), on Kirakira's hues.
    private val lightProjects = listOf(0xFFEC5F8F, 0xFF5AA9E6, 0xFF4FB0AA, 0xFFF4A35F, 0xFFB79AD1, 0xFFE0B531, 0xFFE8775F, 0xFF2D3F63)
    private val darkProjects = listOf(0xFFFF7AA5, 0xFF5AA9E6, 0xFF4FB0AA, 0xFFF4A35F, 0xFFB79AD1, 0xFFF7D35C, 0xFFFF9B85, 0xFF8FA6D6)

    /** Stable per-project colour, using the same hash as the web and desktop clients. */
    @Composable fun project(id: String): Color {
        var hash = 0
        for (ch in id) hash = hash * 31 + ch.code
        val index = ((hash.toLong() and 0xFFFFFFFFL) % lightProjects.size).toInt()
        return Color((if (Kirakira.colors.dark) darkProjects else lightProjects)[index])
    }
}

@Composable
private fun rememberReducedMotion(): Boolean {
    val resolver = LocalContext.current.contentResolver
    var reduced by remember { mutableStateOf(false) }
    // Read again on resume, so changing the setting applies without restarting the app.
    LifecycleResumeEffect(resolver) {
        reduced = Settings.Global.getFloat(resolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
        onPauseOrDispose { }
    }
    return reduced
}

@Composable
fun CatDoTheme(content: @Composable () -> Unit) {
    val darkTheme = isSystemInDarkTheme()
    CompositionLocalProvider(
        LocalKirakira provides if (darkTheme) kirakiraDark else kirakiraLight,
        LocalReducedMotion provides rememberReducedMotion(),
    ) {
        MaterialTheme(colorScheme = if (darkTheme) dark else light, shapes = shapes, typography = typography, content = content)
    }
}
