package org.pjstatecm.graceride.ui.screens.profile

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Bitmap
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.Shuffle
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.components.AdaptiveContentFrame
import org.pjstatecm.graceride.ui.components.GrokBotCanvas
import org.pjstatecm.graceride.ui.components.GrokBotControlsPanel
import org.pjstatecm.graceride.ui.components.GrokBotGeometry
import org.pjstatecm.graceride.ui.components.GrokBotSpec
import org.pjstatecm.graceride.ui.components.buildStandaloneGrokBotSvg
import org.pjstatecm.graceride.ui.components.grokColorLabelRes
import org.pjstatecm.graceride.ui.components.grokFaceLabelRes
import org.pjstatecm.graceride.ui.components.grokShapeLabelRes
import org.pjstatecm.graceride.ui.components.renderGrokBotPngBitmap
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GrokBotStudioScreen(
    container: AppContainer,
    nav: NavHostController,
    initialShape: String? = null,
    initialColor: String? = null,
    initialFace: String? = null,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    val isDark = MaterialTheme.colorScheme.background.luminance() < 0.5f

    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
    val savedSpec = GrokBotSpec.parse(me?.avatarIcon) ?: GrokBotSpec.DEFAULT
    var shape by rememberSaveable(initialShape, savedSpec.shape) {
        mutableStateOf(initialShape?.takeIf { it in GrokBotSpec.SHAPES } ?: savedSpec.shape)
    }
    var face by rememberSaveable(initialFace, savedSpec.face) {
        mutableStateOf(initialFace?.takeIf { it in GrokBotSpec.FACES } ?: savedSpec.face)
    }
    var color by rememberSaveable(initialColor, savedSpec.color) {
        mutableStateOf(initialColor?.takeIf { it in GrokBotSpec.COLORS } ?: savedSpec.color)
    }
    val spec = GrokBotSpec(shape = shape, face = face, color = color)
    var activeMove by remember { mutableStateOf("idle") }
    // Bumped on every Move tap so picking the same move again replays it.
    var movePlayKey by remember { mutableIntStateOf(0) }
    var gazeOffset by remember { mutableStateOf(Offset.Zero) }
    var stageSize by remember { mutableStateOf(IntSize(1, 1)) }
    var busy by remember { mutableStateOf(false) }
    var copyMenuOpen by remember { mutableStateOf(false) }

    val savedAvatarMsg = stringResource(R.string.studio_avatar_saved)
    val savedFileMsg = stringResource(R.string.studio_saved_file)
    val exportFailedMsg = stringResource(R.string.studio_export_failed)
    val copiedImageMsg = stringResource(R.string.studio_copied_image)
    val copiedSvgMsg = stringResource(R.string.studio_copied_svg)
    val copiedLinkMsg = stringResource(R.string.studio_copied_link)

    val createPngLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("image/png"),
    ) { uri ->
        if (uri != null) {
            scope.launch {
                val ok = withContext(Dispatchers.IO) {
                    runCatching {
                        val bmp = renderGrokBotPngBitmap(spec, 512)
                        try {
                            context.contentResolver.openOutputStream(uri)?.use { out ->
                                if (!bmp.compress(Bitmap.CompressFormat.PNG, 100, out)) error("Compress failed")
                            } ?: error("No stream")
                        } finally {
                            bmp.recycle()
                        }
                    }.isSuccess
                }
                snackbar.showSnackbar(if (ok) savedFileMsg else exportFailedMsg)
            }
        }
    }

    val createSvgLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("image/svg+xml"),
    ) { uri ->
        if (uri != null) {
            scope.launch {
                val ok = withContext(Dispatchers.IO) {
                    runCatching {
                        val svg = buildStandaloneGrokBotSvg(spec, 512)
                        context.contentResolver.openOutputStream(uri)?.use { out ->
                            out.write(svg.toByteArray(Charsets.UTF_8))
                        } ?: error("No stream")
                    }.isSuccess
                }
                snackbar.showSnackbar(if (ok) savedFileMsg else exportFailedMsg)
            }
        }
    }

    fun copyPlainText(label: String, text: String, notice: String) {
        if (copyToClipboard(context, label, text)) {
            scope.launch { snackbar.showSnackbar(notice) }
        }
    }

    fun copyImageToClipboard() {
        scope.launch {
            val clipUri = withContext(Dispatchers.IO) {
                runCatching {
                    val bmp = renderGrokBotPngBitmap(spec, 512)
                    try {
                        val dir = java.io.File(context.cacheDir, "shared_avatars").apply { mkdirs() }
                        dir.listFiles()?.forEach { old -> if (old.isFile) old.delete() }
                        val outFile = java.io.File(dir, spec.filename("png"))
                        outFile.outputStream().use { stream ->
                            if (!bmp.compress(Bitmap.CompressFormat.PNG, 100, stream)) {
                                error("PNG compress failed")
                            }
                        }
                        androidx.core.content.FileProvider.getUriForFile(
                            context,
                            "${context.packageName}.fileprovider",
                            outFile,
                        )
                    } finally {
                        bmp.recycle()
                    }
                }.getOrNull()
            }
            if (clipUri != null) {
                val cm = context.getSystemService(ClipboardManager::class.java)
                cm?.setPrimaryClip(ClipData.newUri(context.contentResolver, "Grok Bot", clipUri))
                snackbar.showSnackbar(copiedImageMsg)
            } else {
                snackbar.showSnackbar(exportFailedMsg)
            }
        }
    }

    fun saveAsMyAvatar() {
        if (busy) return
        busy = true
        scope.launch {
            try {
                container.community.updateProfile(
                    ProfileUpdateReq(
                        clear_photo = true,
                        avatar_icon = spec.encode(),
                        avatar_color = "",
                    ),
                )
                container.auth.refresh()
                snackbar.showSnackbar(savedAvatarMsg)
            } catch (e: ApiException) {
                snackbar.showSnackbar(e.message ?: exportFailedMsg)
            } finally {
                busy = false
            }
        }
    }

    val statusReadout = (
        "${stringResource(grokColorLabelRes(spec.color))} " +
            "${stringResource(grokShapeLabelRes(spec.shape))} · " +
            stringResource(grokFaceLabelRes(spec.face))
        ).uppercase()
    val palette = GrokBotGeometry.COLOR_PALETTES[spec.color] ?: GrokBotGeometry.COLOR_PALETTES.getValue("blue")
    val stageBg = if (isDark) Color(0xFF111215) else Color(0xFFFAF9F6)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.studio_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(
                            Icons.AutoMirrored.Outlined.ArrowBack,
                            contentDescription = stringResource(R.string.a11y_back),
                        )
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        AdaptiveContentFrame(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            maxWidth = 760.dp,
        ) {
            LazyColumn(
                Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                contentPadding = PaddingValues(vertical = Spacing.sm),
            ) {
            item {
                Text(
                    stringResource(R.string.studio_subtitle),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            // Stage Card
            item {
                OutlinedCard(
                    shape = RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(
                        Modifier.fillMaxWidth().padding(Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                statusReadout,
                                style = MaterialTheme.typography.labelMedium,
                                fontFamily = FontFamily.Monospace,
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.weight(1f),
                            )
                            AssistChip(
                                onClick = {
                                    val next = GrokBotSpec.random(spec)
                                    shape = next.shape
                                    face = next.face
                                    color = next.color
                                },
                                label = { Text(stringResource(R.string.studio_surprise_me)) },
                                leadingIcon = {
                                    Icon(Icons.Outlined.Shuffle, contentDescription = null, modifier = Modifier.size(16.dp))
                                },
                            )
                        }

                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(270.dp)
                                .clip(RoundedCornerShape(14.dp))
                                .background(stageBg)
                                .background(
                                    Brush.radialGradient(
                                        0f to Color(palette.glowColor),
                                        0.7f to Color.Transparent,
                                    ),
                                )
                                .onSizeChanged { stageSize = it }
                                .pointerInput(Unit) {
                                    detectTapGestures { pos ->
                                        if (stageSize.width <= 0 || stageSize.height <= 0) return@detectTapGestures
                                        val nx = ((pos.x - stageSize.width * 0.5f) / (stageSize.width * 0.45f)).coerceIn(-1f, 1f)
                                        val ny = ((pos.y - stageSize.height * 0.46f) / (stageSize.height * 0.45f)).coerceIn(-1f, 1f)
                                        gazeOffset = Offset(nx, ny)
                                    }
                                }
                                .pointerInput(Unit) {
                                    detectDragGestures(
                                        onDragEnd = { gazeOffset = Offset.Zero },
                                        onDragCancel = { gazeOffset = Offset.Zero },
                                    ) { change, _ ->
                                        change.consume()
                                        if (stageSize.width <= 0 || stageSize.height <= 0) return@detectDragGestures
                                        val pos = change.position
                                        val nx = ((pos.x - stageSize.width * 0.5f) / (stageSize.width * 0.45f)).coerceIn(-1f, 1f)
                                        val ny = ((pos.y - stageSize.height * 0.46f) / (stageSize.height * 0.45f)).coerceIn(-1f, 1f)
                                        gazeOffset = Offset(nx, ny)
                                    }
                                },
                            contentAlignment = Alignment.Center,
                        ) {
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                GrokBotCanvas(
                                    spec = spec,
                                    move = activeMove,
                                    playKey = movePlayKey,
                                    variant = "stage",
                                    gazeOffset = gazeOffset,
                                    modifier = Modifier.size(200.dp),
                                )
                                Text(
                                    stringResource(R.string.studio_cursor_hint),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }

                        Button(
                            onClick = { saveAsMyAvatar() },
                            enabled = !busy,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Icon(Icons.Outlined.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                            Text(
                                stringResource(R.string.studio_save_avatar),
                                modifier = Modifier.padding(start = Spacing.xxs),
                            )
                        }

                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            OutlinedButton(
                                onClick = { createPngLauncher.launch(spec.filename("png")) },
                                modifier = Modifier.weight(1f),
                            ) {
                                Icon(Icons.Outlined.Download, contentDescription = null, modifier = Modifier.size(16.dp))
                                Text(stringResource(R.string.studio_download_png), modifier = Modifier.padding(start = 4.dp))
                            }
                            OutlinedButton(
                                onClick = { createSvgLauncher.launch(spec.filename("svg")) },
                                modifier = Modifier.weight(1f),
                            ) {
                                Icon(Icons.Outlined.Download, contentDescription = null, modifier = Modifier.size(16.dp))
                                Text(stringResource(R.string.studio_download_svg), modifier = Modifier.padding(start = 4.dp))
                            }
                            Box(Modifier.weight(1f)) {
                                OutlinedButton(
                                    onClick = { copyMenuOpen = true },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Text(stringResource(R.string.studio_copy), modifier = Modifier.padding(start = 4.dp))
                                }
                                DropdownMenu(
                                    expanded = copyMenuOpen,
                                    onDismissRequest = { copyMenuOpen = false },
                                ) {
                                    DropdownMenuItem(
                                        text = { Text(stringResource(R.string.studio_copy_image)) },
                                        onClick = {
                                            copyMenuOpen = false
                                            copyImageToClipboard()
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text(stringResource(R.string.studio_copy_svg)) },
                                        onClick = {
                                            copyMenuOpen = false
                                            copyPlainText("Grok Bot SVG", buildStandaloneGrokBotSvg(spec, 512), copiedSvgMsg)
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text(stringResource(R.string.studio_copy_link)) },
                                        onClick = {
                                            copyMenuOpen = false
                                            val url = container.webUrl("studio?shape=${spec.shape}&color=${spec.color}&face=${spec.face}")
                                            copyPlainText("Grok Bot Studio Link", url, copiedLinkMsg)
                                        },
                                    )
                                }
                            }
                        }
                    }
                }
            }

            // Controls Card (Look & Move)
            item {
                OutlinedCard(
                    shape = RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Box(Modifier.padding(Spacing.sm)) {
                        GrokBotControlsPanel(
                            spec = spec,
                            onSpecChange = { next ->
                                shape = next.shape
                                face = next.face
                                color = next.color
                            },
                            activeMove = activeMove,
                            onMoveChange = {
                                activeMove = it
                                movePlayKey++
                            },
                            tilePreviewSize = 48.dp,
                            cols = 4,
                        )
                    }
                }
            }

            // How to make a Grok bot avatar
            item {
                Column(
                    Modifier.fillMaxWidth().padding(top = Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Text(
                        stringResource(R.string.studio_how_title),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                    StepGuideCard(
                        title = stringResource(R.string.studio_step1_title),
                        body = stringResource(R.string.studio_step1_body),
                    )
                    StepGuideCard(
                        title = stringResource(R.string.studio_step2_title),
                        body = stringResource(R.string.studio_step2_body),
                    )
                    StepGuideCard(
                        title = stringResource(R.string.studio_step3_title),
                        body = stringResource(R.string.studio_step3_body),
                    )
                }
            }
            }
        }
    }
}

@Composable
private fun StepGuideCard(title: String, body: String) {
    OutlinedCard(
        shape = RoundedCornerShape(12.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            Modifier.padding(Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Text(title, style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.SemiBold)
            Text(body, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
