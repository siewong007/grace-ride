package org.pjstatecm.graceride.ui.screens.profile


import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.Logout
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.AutoAwesome
import androidx.compose.material.icons.outlined.BarChart
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.ExpandMore
import androidx.compose.material.icons.outlined.Groups
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material.icons.outlined.Security
import androidx.compose.material.icons.outlined.Shuffle
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Badge
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.applyAppLocale
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.PasskeyInfo
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AnimatedMemberAvatar
import org.pjstatecm.graceride.ui.components.AvatarStyle
import org.pjstatecm.graceride.ui.components.GrokBotCanvas
import org.pjstatecm.graceride.ui.components.GrokBotControlsPanel
import org.pjstatecm.graceride.ui.components.GrokBotSpec
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.MemberAvatar
import org.pjstatecm.graceride.ui.components.SectionHeader
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.ThemeMode
import org.pjstatecm.graceride.ui.theme.tabular

internal enum class AvatarMode { PHOTO, ANIMATED, STUDIO, INITIALS, ICON }


@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
internal fun AvatarEditorSheet(
    me: Me,
    photoUrlFor: (Long) -> String,
    busy: Boolean,
    error: String?,
    onDismiss: () -> Unit,
    onOpenFullStudio: (GrokBotSpec) -> Unit,
    onSavePhoto: (Uri?, String) -> Unit,
    onSaveStyle: (String, String) -> Unit,
) {
    val initParsedColor = AvatarStyle.parseAvatarColor(me.avatarColor)
    val initGrokBot = GrokBotSpec.parse(me.avatarIcon)
    val initParsedIcon = AvatarStyle.parseAvatarIcon(if (initGrokBot != null) null else me.avatarIcon)
    val hasCustom = me.photoFileId != null || !me.avatarIcon.isNullOrBlank() || !me.avatarColor.isNullOrBlank()
    val initialIconBase = when {
        initParsedIcon.icon == "initials" || initParsedIcon.icon.isEmpty() ->
            if (!hasCustom) AvatarStyle.DEFAULT_ANIMATED_ICON else "car"
        else -> initParsedIcon.icon
    }
    val initialMode = when {
        me.photoFileId != null -> AvatarMode.PHOTO
        GrokBotSpec.isGrokBotIcon(me.avatarIcon) -> AvatarMode.STUDIO
        AvatarStyle.isAnimatedAvatar(initParsedIcon.icon) -> AvatarMode.ANIMATED
        AvatarStyle.staticIconVector(initParsedIcon.icon) != null -> AvatarMode.ICON
        !me.avatarColor.isNullOrBlank() || initParsedIcon.fgColor != null -> AvatarMode.INITIALS
        else -> AvatarMode.ANIMATED
    }
    var mode by remember { mutableStateOf(initialMode) }
    var studioSpec by remember { mutableStateOf(initGrokBot ?: GrokBotSpec.DEFAULT) }
    var studioMove by remember { mutableStateOf("idle") }
    var icon by remember { mutableStateOf(initialIconBase) }
    var avatarShape by remember {
        mutableStateOf(initParsedIcon.shape ?: AvatarStyle.defaultAvatarShape(initialIconBase))
    }
    var fgColor by remember { mutableStateOf(initParsedIcon.fgColor) }
    var bgColor by remember {
        mutableStateOf(
            initParsedColor.bgColor.ifBlank {
                if (!hasCustom) AvatarStyle.defaultColorFor(AvatarStyle.DEFAULT_ANIMATED_ICON) else "purple"
            },
        )
    }
    var bgShape by remember { mutableStateOf(initParsedColor.bgShape) }
    var customColorPicked by remember { mutableStateOf(initParsedColor.bgColor.isNotEmpty()) }
    var pickedUri by remember { mutableStateOf<Uri?>(null) }

    val encodedColor = AvatarStyle.encodeAvatarColor(bgColor, bgShape)
    val showFgColor = mode == AvatarMode.INITIALS ||
        (mode == AvatarMode.ANIMATED && AvatarStyle.isShapeableAvatar(icon))
    val encodedIcon = when (mode) {
        AvatarMode.STUDIO -> studioSpec.encode()
        AvatarMode.ANIMATED, AvatarMode.ICON -> AvatarStyle.encodeAvatarIcon(
            icon,
            avatarShape,
            if (showFgColor) fgColor else null,
        )
        AvatarMode.INITIALS -> AvatarStyle.encodeAvatarIcon("initials", null, fgColor)
        AvatarMode.PHOTO -> ""
    }

    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) {
            pickedUri = uri
            mode = AvatarMode.PHOTO
        }
    }

    val modes = listOf(
        AvatarMode.ANIMATED to stringResource(R.string.profile_avatar_animated),
        AvatarMode.STUDIO to stringResource(R.string.profile_avatar_studio),
        AvatarMode.PHOTO to stringResource(R.string.profile_avatar_photo),
        AvatarMode.INITIALS to stringResource(R.string.profile_avatar_initials),
        AvatarMode.ICON to stringResource(R.string.profile_avatar_icon),
    )

    val canSave = when (mode) {
        AvatarMode.PHOTO -> pickedUri != null || me.photoFileId != null
        AvatarMode.STUDIO -> true
        AvatarMode.ANIMATED -> AvatarStyle.isAnimatedAvatar(icon)
        AvatarMode.ICON -> AvatarStyle.staticIconVector(icon) != null
        AvatarMode.INITIALS -> true
    }

    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Text(stringResource(R.string.profile_edit_avatar), style = MaterialTheme.typography.titleLarge)

            // Pinned live preview
            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                if (mode == AvatarMode.STUDIO) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        GrokBotCanvas(
                            spec = studioSpec,
                            move = studioMove,
                            variant = "stage",
                            modifier = Modifier.size(116.dp),
                        )
                        TextButton(onClick = { studioSpec = GrokBotSpec.random(studioSpec) }) {
                            Icon(Icons.Outlined.Shuffle, contentDescription = null, modifier = Modifier.size(15.dp))
                            Text(
                                stringResource(R.string.studio_surprise_me),
                                modifier = Modifier.padding(start = 4.dp),
                            )
                        }
                    }
                } else {
                    val previewUrl = when {
                        mode == AvatarMode.PHOTO && pickedUri != null -> pickedUri.toString()
                        mode == AvatarMode.PHOTO -> me.photoFileId?.let(photoUrlFor)
                        else -> null
                    }
                    val previewIcon = when (mode) {
                        AvatarMode.ANIMATED, AvatarMode.ICON, AvatarMode.INITIALS -> encodedIcon
                        else -> null
                    }
                    MemberAvatar(
                        name = me.name,
                        photoUrl = previewUrl,
                        avatarColor = encodedColor,
                        avatarIcon = previewIcon,
                        size = 88.dp,
                    )
                }
            }

            PrimaryScrollableTabRow(selectedTabIndex = modes.indexOfFirst { it.first == mode }.coerceAtLeast(0)) {
                modes.forEach { (m, label) ->
                    Tab(
                        selected = mode == m,
                        onClick = {
                            mode = m
                            if (m == AvatarMode.ANIMATED && !AvatarStyle.isAnimatedAvatar(icon)) {
                                icon = AvatarStyle.DEFAULT_ANIMATED_ICON
                                avatarShape = AvatarStyle.defaultAvatarShape(icon)
                                if (!customColorPicked) {
                                    bgColor = AvatarStyle.defaultColorFor(AvatarStyle.DEFAULT_ANIMATED_ICON)
                                }
                            } else if (m == AvatarMode.ICON && AvatarStyle.staticIconVector(icon) == null) {
                                icon = "car"
                            }
                        },
                        text = { Text(label, maxLines = 1) },
                    )
                }
            }

            Column(
                Modifier
                    .weight(1f, fill = false)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                when (mode) {
                    AvatarMode.PHOTO -> {
                        OutlinedButton(
                            onClick = {
                                pickImage.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                            },
                            modifier = Modifier.fillMaxWidth().height(48.dp),
                        ) {
                            Text(stringResource(R.string.profile_avatar_choose_photo))
                        }
                    }
                    AvatarMode.ANIMATED -> {
                        val animatedOptions = listOf(
                            "grace_car" to R.string.profile_avatar_grace_car,
                            "peace_dove" to R.string.profile_avatar_peace_dove,
                            "sunny_cloud" to R.string.profile_avatar_sunny_cloud,
                            "happy_bear" to R.string.profile_avatar_happy_bear,
                            "fellowship_cat" to R.string.profile_avatar_fellowship_cat,
                            "joy_puppy" to R.string.profile_avatar_joy_puppy,
                            "star_lantern" to R.string.profile_avatar_star_lantern,
                            "blossom_sprout" to R.string.profile_avatar_blossom_sprout,
                            "grok" to R.string.profile_avatar_grok,
                            "muse" to R.string.profile_avatar_muse,
                        )
                        Column(
                            Modifier.fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            animatedOptions.chunked(2).forEach { pair ->
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    pair.forEach { (key, labelRes) ->
                                        val selected = icon == key
                                        val activeBg = if (selected) bgColor else AvatarStyle.defaultColorFor(key)
                                        val activeFg = if (selected) fgColor else null
                                        val cardShape = if (selected && AvatarStyle.isShapeableAvatar(key)) {
                                            avatarShape
                                        } else {
                                            AvatarStyle.defaultAvatarShape(key)
                                        }
                                        val (cardBg, cardFg) = AvatarStyle.colorPair(activeBg)
                                        OutlinedCard(
                                            onClick = {
                                                icon = key
                                                avatarShape = AvatarStyle.defaultAvatarShape(key)
                                                if (!customColorPicked) {
                                                    bgColor = AvatarStyle.defaultColorFor(key)
                                                }
                                            },
                                            border = if (selected) {
                                                BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
                                            } else {
                                                BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                            },
                                            modifier = Modifier.weight(1f),
                                        ) {
                                            Column(
                                                Modifier.fillMaxWidth().padding(Spacing.xs),
                                                horizontalAlignment = Alignment.CenterHorizontally,
                                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                                            ) {
                                                Surface(
                                                    color = cardBg,
                                                    shape = CircleShape,
                                                    border = if (activeBg == "white") {
                                                        BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                                    } else {
                                                        null
                                                    },
                                                    modifier = Modifier.size(48.dp),
                                                ) {
                                                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                                        AnimatedMemberAvatar(
                                                            icon = key,
                                                            tint = cardFg,
                                                            modifier = Modifier.size(40.dp),
                                                            avatarColor = activeFg ?: activeBg,
                                                            avatarShape = cardShape,
                                                        )
                                                    }
                                                }
                                                Text(
                                                    stringResource(labelRes),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    textAlign = TextAlign.Center,
                                                    maxLines = 1,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AvatarMode.ICON -> {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            AvatarStyle.STATIC_ICONS.forEach { key ->
                                FilterChip(
                                    selected = icon == key,
                                    onClick = { icon = key },
                                    label = {
                                        AnimatedMemberAvatar(
                                            icon = key,
                                            tint = MaterialTheme.colorScheme.onSurface,
                                            modifier = Modifier.size(26.dp),
                                            avatarColor = fgColor ?: bgColor,
                                        )
                                    },
                                )
                            }
                        }
                    }
                    AvatarMode.STUDIO -> {
                        Column(verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            GrokBotControlsPanel(
                                spec = studioSpec,
                                onSpecChange = { studioSpec = it },
                                activeMove = studioMove,
                                onMoveChange = { studioMove = it },
                                tilePreviewSize = 36.dp,
                                cols = 4,
                            )
                            OutlinedButton(
                                onClick = { onOpenFullStudio(studioSpec) },
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Text(stringResource(R.string.studio_open_full))
                            }
                        }
                    }
                    AvatarMode.INITIALS -> Unit
                }

                if (mode == AvatarMode.ANIMATED && AvatarStyle.isShapeableAvatar(icon)) {
                    Text(stringResource(R.string.profile_avatar_shape), style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        AvatarStyle.AVATAR_SHAPES.forEach { sKey ->
                            FilterChip(
                                selected = avatarShape == sKey,
                                onClick = { avatarShape = sKey },
                                label = { Text(stringResource(avatarShapeLabelRes(sKey)), maxLines = 1) },
                            )
                        }
                    }
                }

                val showFgColorPicker = mode == AvatarMode.INITIALS ||
                    (mode == AvatarMode.ANIMATED && AvatarStyle.isShapeableAvatar(icon))
                if (showFgColorPicker) {
                    Text(stringResource(R.string.profile_avatar_fg_color), style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        FilterChip(
                            selected = fgColor == null,
                            onClick = { fgColor = null },
                            label = { Text(stringResource(R.string.profile_avatar_color_auto), maxLines = 1) },
                        )
                        AvatarStyle.COLORS.forEach { cKey ->
                            val swatchBg = AvatarStyle.grokSkinColor(cKey)
                            val swatchFg = AvatarStyle.grokEyeColor(cKey)
                            Box(
                                modifier = Modifier
                                    .size(48.dp)
                                    .clip(CircleShape)
                                    .clickable { fgColor = cKey },
                                contentAlignment = Alignment.Center,
                            ) {
                                Surface(
                                    color = swatchBg,
                                    shape = CircleShape,
                                    border = when {
                                        fgColor == cKey -> BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
                                        cKey == "white" -> BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                        else -> null
                                    },
                                    modifier = Modifier.size(38.dp),
                                ) {
                                    if (fgColor == cKey) {
                                        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                            Icon(Icons.Outlined.Check, contentDescription = cKey, tint = swatchFg, modifier = Modifier.size(18.dp))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if (mode != AvatarMode.STUDIO) {
                    Text(stringResource(R.string.profile_avatar_bg_shape), style = MaterialTheme.typography.labelMedium)
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        AvatarStyle.AVATAR_BG_SHAPES.forEach { bsKey ->
                            val selected = bgShape == bsKey
                            FilterChip(
                                selected = selected,
                                onClick = { bgShape = bsKey },
                                leadingIcon = {
                                    Surface(
                                        color = Color.Transparent,
                                        shape = AvatarStyle.bgComposeShape(bsKey),
                                        border = BorderStroke(
                                            1.5.dp,
                                            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                                        ),
                                        modifier = Modifier.size(14.dp),
                                    ) {}
                                },
                                label = { Text(stringResource(avatarBgShapeLabelRes(bsKey)), maxLines = 1) },
                            )
                        }
                    }

                    if (mode != AvatarMode.PHOTO) {
                        Text(stringResource(R.string.profile_avatar_color), style = MaterialTheme.typography.labelMedium)
                        @Suppress("DEPRECATION")
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xxs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                        ) {
                            AvatarStyle.COLORS.forEach { cKey ->
                                val (bg, fg) = AvatarStyle.colorPair(cKey)
                                Box(
                                    modifier = Modifier
                                        .size(48.dp)
                                        .clip(CircleShape)
                                        .clickable {
                                            bgColor = cKey
                                            customColorPicked = true
                                        },
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Surface(
                                        color = bg,
                                        shape = CircleShape,
                                        border = when {
                                            bgColor == cKey -> BorderStroke(2.dp, fg)
                                            cKey == "white" -> BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                                            else -> null
                                        },
                                        modifier = Modifier.size(38.dp),
                                    ) {
                                        if (bgColor == cKey) {
                                            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                                Icon(Icons.Outlined.Check, contentDescription = cKey, tint = fg, modifier = Modifier.size(18.dp))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                error?.let { err ->
                    Text(
                        err,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }

            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                OutlinedButton(
                    onClick = onDismiss,
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    enabled = !busy && canSave,
                    onClick = {
                        when (mode) {
                            AvatarMode.PHOTO -> onSavePhoto(pickedUri, encodedColor)
                            AvatarMode.STUDIO -> onSaveStyle(studioSpec.encode(), "")
                            AvatarMode.ANIMATED, AvatarMode.ICON, AvatarMode.INITIALS -> onSaveStyle(encodedIcon, encodedColor)
                        }
                    },
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    if (busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.profile_avatar_save))
                    }
                }
            }
        }
    }
}

private fun avatarShapeLabelRes(key: String): Int = when (key) {
    "circle" -> R.string.profile_avatar_shape_circle
    "pebble" -> R.string.profile_avatar_shape_pebble
    "squircle" -> R.string.profile_avatar_shape_squircle
    "capsule" -> R.string.profile_avatar_shape_capsule
    "triangle" -> R.string.profile_avatar_shape_triangle
    "cloud" -> R.string.profile_avatar_shape_cloud
    "droplet" -> R.string.profile_avatar_shape_droplet
    else -> R.string.profile_avatar_shape_hexagon
}

private fun avatarBgShapeLabelRes(key: String): Int = when (key) {
    "squircle" -> R.string.profile_avatar_bg_squircle
    "rounded" -> R.string.profile_avatar_bg_rounded
    "hexagon" -> R.string.profile_avatar_bg_hexagon
    "cloud" -> R.string.profile_avatar_bg_cloud
    "pebble" -> R.string.profile_avatar_bg_pebble
    else -> R.string.profile_avatar_bg_circle
}
