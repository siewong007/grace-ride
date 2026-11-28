package org.pjstatecm.graceride.ui.screens.traffic

import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.ArrowDownward
import androidx.compose.material.icons.outlined.ArrowUpward
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material.icons.outlined.Download
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.PhotoCamera
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
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
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.selection.toggleable
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AreaFareDto
import org.pjstatecm.graceride.data.api.AreaPhotoDto
import org.pjstatecm.graceride.data.api.BulkFareItemReq
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.media.PreparedUpload
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.domain.FareImportRowStatus
import org.pjstatecm.graceride.domain.FareIssueKind
import org.pjstatecm.graceride.domain.FareSheetCodec
import org.pjstatecm.graceride.domain.FareSheetLayout
import org.pjstatecm.graceride.domain.FareWorkbookParseResult
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.domain.XlsxErrorCode
import org.pjstatecm.graceride.domain.XlsxException
import org.pjstatecm.graceride.platform.ExportShare
import java.nio.charset.StandardCharsets
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular
import java.math.BigDecimal

/** Traffic Hub "Areas" tab combining Area Photos, Pairwise Fares, and Area Management sub-tabs. */
@Composable
internal fun TrafficAreasSection(
    state: TrafficHubUiState,
    me: Me?,
    vm: TrafficHubViewModel,
    fileUrlFor: (Long) -> String,
    bottomClearance: Dp,
) {
    val subTabs = remember(me) {
        buildList {
            if (me?.can("admin_area_photos") == true) add("photos" to R.string.traffic_areas_sub_photos)
            if (me?.can("admin_fares") == true) add("fares" to R.string.traffic_areas_sub_fares)
            if (me?.can("admin_areas") == true) add("areas" to R.string.traffic_areas_sub_manage)
        }
    }

    if (state.areasLoading && state.areas.isEmpty()) {
        LoadingView()
        return
    }
    if (state.areasError != null && state.areas.isEmpty()) {
        ErrorView(state.areasError, onRetry = { vm.refreshActive(silent = false) })
        return
    }

    Column(Modifier.fillMaxSize()) {
        if (subTabs.size > 1) {
            SingleChoiceSegmentedButtonRow(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            ) {
                subTabs.forEachIndexed { index, (key, labelRes) ->
                    SegmentedButton(
                        selected = state.areaSubTab == key,
                        onClick = { vm.setAreaSubTab(key) },
                        shape = SegmentedButtonDefaults.itemShape(index = index, count = subTabs.size),
                    ) {
                        Text(stringResource(labelRes), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }
        }

        AnimatedContent(
            targetState = state.areaSubTab,
            transitionSpec = {
                if (animatorsEnabled()) {
                    val fromIdx = subTabs.indexOfFirst { it.first == initialState }
                    val toIdx = subTabs.indexOfFirst { it.first == targetState }
                    val dir = if (fromIdx != -1 && toIdx != -1 && toIdx != fromIdx) {
                        if (toIdx > fromIdx) 1 else -1
                    } else {
                        1
                    }
                    (slideInHorizontally(tween(240, easing = GraceRideSpringEasing)) { it * dir } +
                        fadeIn(tween(180)))
                        .togetherWith(
                            slideOutHorizontally(tween(220, easing = GraceRideSpringEasing)) { (-it / 3) * dir } +
                                fadeOut(tween(140)),
                        )
                } else {
                    fadeIn(tween(120)).togetherWith(fadeOut(tween(100)))
                }
            },
            label = "traffic_areas_subtab",
            modifier = Modifier.weight(1f),
        ) { subTab ->
            when (subTab) {
                "photos" -> AreaPhotosPane(state, vm, fileUrlFor, bottomClearance)
                "fares" -> AreaFaresPane(state, me, vm, bottomClearance)
                else -> AreasManagePane(state, vm, bottomClearance)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AreaPhotosPane(
    state: TrafficHubUiState,
    vm: TrafficHubViewModel,
    fileUrlFor: (Long) -> String,
    bottomClearance: Dp,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var pendingUpload by remember { mutableStateOf<Pair<Uri, PreparedUpload>?>(null) }
    var captionInput by remember { mutableStateOf("") }
    var editingPhoto by remember { mutableStateOf<AreaPhotoDto?>(null) }
    var deletingPhoto by remember { mutableStateOf<AreaPhotoDto?>(null) }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri: Uri? ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            val res = withContext(Dispatchers.IO) {
                runCatching {
                    prepareImageUpload(
                        context = context,
                        uri = uri,
                        prefix = "area-photo",
                        maxDimensionPx = 1600,
                        quality = 85,
                    )
                }
            }
            res.onSuccess { prepared ->
                captionInput = ""
                pendingUpload = uri to prepared
            }.onFailure { err ->
                err.message?.let(vm::reportMessage)
            }
        }
    }

    val photosDto = state.areaPhotos
    val selectedAreaId = state.selectedAreaId ?: state.areas.firstOrNull()?.id

    PullToRefreshBox(
        isRefreshing = state.refreshing,
        onRefresh = { vm.refreshActive(silent = false) },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                start = Spacing.sm,
                end = Spacing.sm,
                top = Spacing.xs,
                bottom = Spacing.lg + bottomClearance,
            ),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            if (state.areas.isNotEmpty()) {
                item {
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        items(state.areas, key = { "ap-area-${it.id}" }) { area ->
                            FilterChip(
                                selected = area.id == selectedAreaId,
                                onClick = { vm.selectArea(area.id) },
                                label = {
                                    Text("${area.name}${if (area.photoCount > 0) " (${area.photoCount})" else ""}")
                                },
                            )
                        }
                    }
                }
            }

            if (selectedAreaId != null && photosDto != null) {
                item {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            "${photosDto.area.name} · ${photosDto.photos.size}/${photosDto.max}",
                            style = MaterialTheme.typography.titleSmall,
                        )
                        if (photosDto.canAdd) {
                            Button(
                                onClick = {
                                    picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                                },
                                enabled = !state.areasBusy && photosDto.photos.size < photosDto.max,
                            ) {
                                if (state.areasBusy) {
                                    InlineButtonSpinner()
                                } else {
                                    Icon(Icons.Outlined.PhotoCamera, contentDescription = null, modifier = Modifier.size(18.dp))
                                }
                                Spacer(Modifier.width(Spacing.xxs))
                                Text(stringResource(R.string.traffic_area_photo_add))
                            }
                        }
                    }
                    if (photosDto.photos.size >= photosDto.max) {
                        Text(
                            stringResource(R.string.traffic_area_photos_max_reached, photosDto.max),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            val photos = photosDto?.photos.orEmpty()
            when {
                state.areaPhotosLoading && photosDto == null -> {
                    item { LoadingView(Modifier.height(220.dp)) }
                }
                state.areaPhotosError != null && photosDto == null -> {
                    item {
                        ErrorView(
                            message = state.areaPhotosError,
                            modifier = Modifier.height(260.dp),
                            onRetry = { selectedAreaId?.let(vm::selectArea) ?: vm.refreshActive(silent = false) },
                        )
                    }
                }
                photos.isEmpty() -> {
                    item {
                        EmptyView(
                            text = stringResource(R.string.traffic_area_photos_empty),
                            icon = Icons.Outlined.PhotoCamera,
                        )
                    }
                }
                else -> {
                    itemsIndexed(photos, key = { _, p -> "area-photo-${p.id}" }) { index, photo ->
                        OutlinedCard(
                            Modifier
                                .fillMaxWidth()
                                .animateItem(),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        ) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                AsyncImage(
                                    model = ImageRequest.Builder(context)
                                        .data(fileUrlFor(photo.fileId))
                                        .crossfade(true)
                                        .build(),
                                    contentDescription = photo.caption.ifBlank { photosDto?.area?.name },
                                    contentScale = ContentScale.Crop,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(180.dp)
                                        .clip(RoundedCornerShape(8.dp)),
                                )
                                if (photo.caption.isNotBlank()) {
                                    Text(photo.caption, style = MaterialTheme.typography.bodyMedium)
                                }
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        photo.createdBy?.name.orEmpty(),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    Row {
                                        if (photosDto?.canReorder == true && photos.size > 1) {
                                            IconButton(
                                                onClick = { if (selectedAreaId != null) vm.movePhoto(selectedAreaId, photo.id, -1) },
                                                enabled = index > 0 && !state.areasBusy,
                                            ) {
                                                Icon(Icons.Outlined.ArrowUpward, contentDescription = stringResource(R.string.traffic_area_photo_move_up))
                                            }
                                            IconButton(
                                                onClick = { if (selectedAreaId != null) vm.movePhoto(selectedAreaId, photo.id, 1) },
                                                enabled = index < photos.lastIndex && !state.areasBusy,
                                            ) {
                                                Icon(Icons.Outlined.ArrowDownward, contentDescription = stringResource(R.string.traffic_area_photo_move_down))
                                            }
                                        }
                                        if (photo.canEdit) {
                                            IconButton(
                                                onClick = {
                                                    captionInput = photo.caption
                                                    editingPhoto = photo
                                                },
                                                enabled = !state.areasBusy,
                                            ) {
                                                Icon(Icons.Outlined.Edit, contentDescription = stringResource(R.string.traffic_area_photo_edit_caption))
                                            }
                                        }
                                        if (photo.canDelete) {
                                            IconButton(
                                                onClick = { deletingPhoto = photo },
                                                enabled = !state.areasBusy,
                                            ) {
                                                Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete))
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pendingUpload?.let { (uri, prepared) ->
        AlertDialog(
            onDismissRequest = { pendingUpload = null },
            title = { Text(stringResource(R.string.traffic_area_photo_add)) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    AsyncImage(
                        model = ImageRequest.Builder(context)
                            .data(uri)
                            .crossfade(true)
                            .build(),
                        contentDescription = null,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(140.dp)
                            .clip(RoundedCornerShape(8.dp)),
                    )
                    OutlinedTextField(
                        value = captionInput,
                        onValueChange = { captionInput = it },
                        label = { Text(stringResource(R.string.traffic_area_photo_caption_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val areaId = selectedAreaId
                        if (areaId != null) {
                            vm.uploadAndAddPhoto(
                                areaId = areaId,
                                bytes = prepared.bytes,
                                fileName = prepared.filename,
                                mimeType = prepared.mime,
                                caption = captionInput,
                            )
                        }
                        pendingUpload = null
                    },
                ) { Text(stringResource(R.string.common_save)) }
            },
            dismissButton = {
                TextButton(onClick = { pendingUpload = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    editingPhoto?.let { photo ->
        AlertDialog(
            onDismissRequest = { editingPhoto = null },
            title = { Text(stringResource(R.string.traffic_area_photo_edit_caption)) },
            text = {
                OutlinedTextField(
                    value = captionInput,
                    onValueChange = { captionInput = it },
                    label = { Text(stringResource(R.string.traffic_area_photo_caption_hint)) },
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (selectedAreaId != null) {
                            vm.updatePhotoCaption(selectedAreaId, photo.id, captionInput)
                        }
                        editingPhoto = null
                    },
                ) { Text(stringResource(R.string.common_save)) }
            },
            dismissButton = {
                TextButton(onClick = { editingPhoto = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    deletingPhoto?.let { photo ->
        AlertDialog(
            onDismissRequest = { deletingPhoto = null },
            title = { Text(stringResource(R.string.common_delete)) },
            text = { Text(stringResource(R.string.traffic_area_photo_delete_confirm)) },
            confirmButton = {
                Button(
                    onClick = {
                        if (selectedAreaId != null) vm.deletePhoto(selectedAreaId, photo.id)
                        deletingPhoto = null
                    },
                ) { Text(stringResource(R.string.common_delete)) }
            },
            dismissButton = {
                TextButton(onClick = { deletingPhoto = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun AreaFaresPane(
    state: TrafficHubUiState,
    me: Me?,
    vm: TrafficHubViewModel,
    bottomClearance: Dp,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var showEditDialog by remember { mutableStateOf(false) }
    var isEditingFare by remember { mutableStateOf(false) }
    var areaAId by remember { mutableStateOf<Long?>(null) }
    var areaBId by remember { mutableStateOf<Long?>(null) }
    var amountText by remember { mutableStateOf("") }
    var deleteTarget by remember { mutableStateOf<AreaFareDto?>(null) }

    var showImportDialog by remember { mutableStateOf(false) }
    var importWorkbook by remember { mutableStateOf<FareWorkbookParseResult?>(null) }
    var importSheetIndex by remember { mutableStateOf(0) }
    var createMissingAreas by remember { mutableStateOf(false) }
    var parseError by remember { mutableStateOf<String?>(null) }
    var importStatusBanner by remember { mutableStateOf<String?>(null) }

    val canUpdateFares = me?.can("admin_fares", "update") == true
    val canCreateAreas = me?.can("admin_areas", "create") == true
    val originLabel = stringResource(R.string.traffic_fare_sheet_origin)
    val sheetTitle = stringResource(R.string.traffic_fare_sheet_name)

    val fareSheetPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri: Uri? ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            parseError = null
            val res = withContext(Dispatchers.IO) {
                runCatching {
                    val cr = context.contentResolver
                    var fileName = "fares.xlsx"
                    cr.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                        if (c.moveToFirst()) {
                            c.getString(0)?.takeIf { it.isNotBlank() }?.let { fileName = it }
                        }
                    }
                    val bytes = cr.openInputStream(uri)?.use { it.readBytes() }
                        ?: throw IllegalStateException("Unable to read selected file")
                    FareSheetCodec.readFareBytes(
                        bytes = bytes,
                        fileName = fileName,
                        allAreas = state.areas,
                        existingFares = state.areaFares,
                    )
                }
            }
            res.onSuccess { wb ->
                val firstPopulated = wb.sheets.indexOfFirst {
                    it.parsed.rows.isNotEmpty() || it.parsed.issues.isNotEmpty()
                }.coerceAtLeast(0)
                importWorkbook = wb
                importSheetIndex = firstPopulated
                createMissingAreas = false
                parseError = null
                showImportDialog = true
            }.onFailure { err ->
                importWorkbook = null
                parseError = when ((err as? XlsxException)?.code) {
                    XlsxErrorCode.LEGACY_XLS -> context.getString(R.string.traffic_fare_import_err_xls)
                    XlsxErrorCode.TOO_LARGE -> context.getString(R.string.traffic_fare_import_err_large)
                    XlsxErrorCode.INVALID_XLSX -> context.getString(R.string.traffic_fare_import_err_invalid)
                    null -> err.message ?: context.getString(R.string.traffic_fare_import_err_invalid)
                }
                showImportDialog = true
            }
        }
    }

    val filteredFares = remember(state.areaFares, state.fareSearchQuery) {
        val q = state.fareSearchQuery.trim()
        if (q.isEmpty()) state.areaFares
        else state.areaFares.filter {
            it.areaA.name.contains(q, ignoreCase = true) || it.areaB.name.contains(q, ignoreCase = true)
        }
    }

    PullToRefreshBox(
        isRefreshing = state.refreshing,
        onRefresh = { vm.refreshActive(silent = false) },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                start = Spacing.sm,
                end = Spacing.sm,
                top = Spacing.xs,
                bottom = Spacing.lg + bottomClearance,
            ),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            item {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = state.fareSearchQuery,
                        onValueChange = { vm.setFareSearchQuery(it) },
                        placeholder = {
                            Text(
                                stringResource(R.string.traffic_fare_search_hint),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        },
                        leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                        trailingIcon = {
                            if (state.fareSearchQuery.isNotEmpty()) {
                                IconButton(onClick = { vm.setFareSearchQuery("") }) {
                                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                }
                            }
                        },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )
                    Button(
                        onClick = {
                            isEditingFare = false
                            areaAId = state.areas.firstOrNull()?.id
                            areaBId = state.areas.getOrNull(1)?.id ?: areaAId
                            amountText = ""
                            showEditDialog = true
                        },
                        enabled = state.areas.size >= 2 && !state.areasBusy,
                    ) {
                        Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(stringResource(R.string.traffic_fare_add))
                    }
                }
            }

            item {
                FlowRow(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                ) {
                    if (state.areas.any { it.active }) {
                        OutlinedButton(
                            onClick = {
                                scope.launch {
                                    val bytes = withContext(Dispatchers.Default) {
                                        FareSheetCodec.buildFaresXlsx(
                                            areas = state.areas,
                                            fares = state.areaFares,
                                            sheetName = sheetTitle,
                                            originLabel = originLabel,
                                        )
                                    }
                                    ExportShare.writeAndShareFile(
                                        context = context,
                                        filename = "grace-ride-area-fares.xlsx",
                                        mimeType = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                                        bytes = bytes,
                                    )
                                }
                            },
                            enabled = !state.areasBusy,
                        ) {
                            Icon(Icons.Outlined.Description, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xxs))
                            Text(stringResource(R.string.traffic_fare_export_xlsx))
                        }
                        OutlinedButton(
                            onClick = {
                                scope.launch {
                                    val csv = withContext(Dispatchers.Default) {
                                        FareSheetCodec.buildFaresCsv(
                                            areas = state.areas,
                                            fares = state.areaFares,
                                            originLabel = originLabel,
                                        )
                                    }
                                    ExportShare.writeAndShareFile(
                                        context = context,
                                        filename = "grace-ride-area-fares.csv",
                                        mimeType = "text/csv",
                                        bytes = csv.toByteArray(StandardCharsets.UTF_8),
                                    )
                                }
                            },
                            enabled = !state.areasBusy,
                        ) {
                            Icon(Icons.Outlined.Download, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xxs))
                            Text(stringResource(R.string.traffic_fare_export_csv))
                        }
                    }
                    if (canUpdateFares) {
                        OutlinedButton(
                            onClick = {
                                parseError = null
                                fareSheetPicker.launch(
                                    arrayOf(
                                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                                        "text/csv",
                                        "text/comma-separated-values",
                                        "text/tab-separated-values",
                                        "application/octet-stream",
                                        "*/*",
                                    ),
                                )
                            },
                            enabled = !state.areasBusy,
                        ) {
                            Icon(Icons.Outlined.UploadFile, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xxs))
                            Text(stringResource(R.string.traffic_fare_import_action))
                        }
                    }
                }
            }

            importStatusBanner?.let { banner ->
                item {
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary),
                    ) {
                        Row(
                            Modifier.fillMaxWidth().padding(Spacing.sm),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(banner, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
                            IconButton(onClick = { importStatusBanner = null }) {
                                Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.common_close))
                            }
                        }
                    }
                }
            }

            if (filteredFares.isEmpty()) {
                item {
                    EmptyView(text = stringResource(R.string.traffic_fare_empty))
                }
            } else {
                items(filteredFares, key = { "fare-${it.id}" }) { fare ->
                    OutlinedCard(
                        onClick = {
                            isEditingFare = true
                            areaAId = fare.areaA.id
                            areaBId = fare.areaB.id
                            amountText = BigDecimal(fare.amountCents).movePointLeft(2).stripTrailingZeros().toPlainString()
                            showEditDialog = true
                        },
                        modifier = Modifier
                            .fillMaxWidth()
                            .animateItem(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Row(
                            Modifier.fillMaxWidth().padding(Spacing.sm),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Column(Modifier.weight(1f)) {
                                Text("${fare.areaA.name} ↔ ${fare.areaB.name}", style = MaterialTheme.typography.titleMedium)
                                Text(
                                    Money.rm(fare.amountCents),
                                    style = MaterialTheme.typography.bodyMedium.tabular(),
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
                            IconButton(onClick = { deleteTarget = fare }, enabled = !state.areasBusy) {
                                Icon(Icons.Outlined.DeleteOutline, contentDescription = stringResource(R.string.common_delete))
                            }
                        }
                    }
                }
            }
        }
    }

    if (showEditDialog) {
        val areas = state.areas
        val parsedCents = parseRmToCents(amountText)
        val sameArea = areaAId != null && areaAId == areaBId
        AlertDialog(
            onDismissRequest = { showEditDialog = false },
            title = { Text(stringResource(if (isEditingFare) R.string.traffic_fare_edit else R.string.traffic_fare_add)) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                    AreaDropdownField(
                        label = stringResource(R.string.traffic_fare_area_a),
                        areas = areas,
                        selectedId = areaAId,
                        onSelect = { areaAId = it },
                    )
                    AreaDropdownField(
                        label = stringResource(R.string.traffic_fare_area_b),
                        areas = areas,
                        selectedId = areaBId,
                        onSelect = { areaBId = it },
                    )
                    if (sameArea) {
                        Text(
                            stringResource(R.string.traffic_fare_same_area_error),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                    OutlinedTextField(
                        value = amountText,
                        onValueChange = { amountText = it },
                        label = { Text(stringResource(R.string.traffic_fare_amount_rm)) },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val a = areaAId
                        val b = areaBId
                        if (a != null && b != null && a != b && parsedCents != null && parsedCents >= 0L) {
                            vm.putFare(a, b, parsedCents)
                            showEditDialog = false
                        }
                    },
                    enabled = areaAId != null && areaBId != null && !sameArea && parsedCents != null && parsedCents >= 0L,
                ) { Text(stringResource(R.string.common_save)) }
            },
            dismissButton = {
                TextButton(onClick = { showEditDialog = false }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    deleteTarget?.let { fare ->
        AlertDialog(
            onDismissRequest = { deleteTarget = null },
            title = { Text(stringResource(R.string.common_delete)) },
            text = { Text(stringResource(R.string.traffic_fare_delete_confirm, fare.areaA.name, fare.areaB.name)) },
            confirmButton = {
                Button(
                    onClick = {
                        vm.deleteFare(fare.id)
                        deleteTarget = null
                    },
                ) { Text(stringResource(R.string.common_delete)) }
            },
            dismissButton = {
                TextButton(onClick = { deleteTarget = null }) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }

    if (showImportDialog) {
        val wb = importWorkbook
        val sheet = wb?.sheets?.getOrNull(importSheetIndex) ?: wb?.sheets?.firstOrNull()
        val parsed = sheet?.parsed
        val applicable = remember(parsed, canCreateAreas, createMissingAreas) {
            parsed?.rows.orEmpty().mapNotNull { r ->
                val effectiveNew = r.status == FareImportRowStatus.UNKNOWN_AREA && canCreateAreas && createMissingAreas
                if (r.status == FareImportRowStatus.NEW || r.status == FareImportRowStatus.UPDATE || effectiveNew) {
                    BulkFareItemReq(areaA = r.areaAName, areaB = r.areaBName, amountCents = r.amountCents)
                } else {
                    null
                }
            }
        }
        val newCount = parsed?.rows?.count {
            it.status == FareImportRowStatus.NEW ||
                (it.status == FareImportRowStatus.UNKNOWN_AREA && canCreateAreas && createMissingAreas)
        } ?: 0
        val updateCount = parsed?.rows?.count { it.status == FareImportRowStatus.UPDATE } ?: 0
        val unchangedCount = parsed?.rows?.count { it.status == FareImportRowStatus.UNCHANGED } ?: 0
        val skippedCount = parsed?.rows?.count {
            it.status == FareImportRowStatus.UNKNOWN_AREA && !(canCreateAreas && createMissingAreas)
        } ?: 0
        val canApply = !state.areasBusy && parsed != null && parsed.issues.isEmpty() && applicable.isNotEmpty()
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        val view = LocalView.current
        val statusColors = LocalStatusColors.current
        var importFilter by remember(importSheetIndex, showImportDialog) {
            mutableStateOf(if (parsed?.issues?.isNotEmpty() == true) "issues" else "changes")
        }

        ModalBottomSheet(
            onDismissRequest = { showImportDialog = false },
            sheetState = sheetState,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = Spacing.sm)
                    .padding(bottom = Spacing.md),
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Text(
                    stringResource(R.string.traffic_fare_import_title),
                    style = MaterialTheme.typography.titleMedium,
                )
                parseError?.let { err ->
                    Surface(
                        color = statusColors.dangerSoft,
                        contentColor = statusColors.danger,
                        shape = MaterialTheme.shapes.small,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(err, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(Spacing.xs))
                    }
                }
                if (wb != null && parsed != null) {
                    Text(
                        "${wb.fileName} (${stringResource(if (parsed.layout == FareSheetLayout.MATRIX) R.string.traffic_fare_layout_matrix else R.string.traffic_fare_layout_list)})",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (wb.sheets.size > 1) {
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            itemsIndexed(wb.sheets) { idx, s ->
                                FilterChip(
                                    selected = idx == importSheetIndex,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        importSheetIndex = idx
                                        createMissingAreas = false
                                    },
                                    label = { Text(s.name) },
                                )
                            }
                        }
                    }
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        StatusPill(stringResource(R.string.traffic_fare_pill_new, newCount), RideStatusKind.ACTIVE)
                        StatusPill(stringResource(R.string.traffic_fare_pill_update, updateCount), RideStatusKind.PENDING)
                        StatusPill(stringResource(R.string.traffic_fare_pill_same, unchangedCount), RideStatusKind.DONE)
                        if (skippedCount > 0) {
                            StatusPill(stringResource(R.string.traffic_fare_pill_skipped, skippedCount), RideStatusKind.CANCELLED)
                        }
                    }
                    if (parsed.unknownAreas.isNotEmpty() && canCreateAreas) {
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 48.dp)
                                .toggleable(
                                    value = createMissingAreas,
                                    role = Role.Switch,
                                    onValueChange = {
                                        UiHaptics.tick(view)
                                        createMissingAreas = it
                                    },
                                ),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(
                                stringResource(R.string.traffic_fare_import_create_areas, parsed.unknownAreas.joinToString(", ")),
                                style = MaterialTheme.typography.bodySmall,
                                modifier = Modifier.weight(1f).padding(end = Spacing.xs),
                            )
                            Switch(checked = createMissingAreas, onCheckedChange = null)
                        }
                    }

                    val changeRows = remember(parsed, canCreateAreas, createMissingAreas) {
                        parsed.rows.filter { r ->
                            r.status == FareImportRowStatus.NEW ||
                                r.status == FareImportRowStatus.UPDATE ||
                                (r.status == FareImportRowStatus.UNKNOWN_AREA && canCreateAreas && createMissingAreas)
                        }
                    }
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        item {
                            FilterChip(
                                selected = importFilter == "changes",
                                onClick = { UiHaptics.tick(view); importFilter = "changes" },
                                label = { Text("${stringResource(R.string.traffic_fare_filter_changes)} (${newCount + updateCount})") },
                            )
                        }
                        if (parsed.issues.isNotEmpty()) {
                            item {
                                FilterChip(
                                    selected = importFilter == "issues",
                                    onClick = { UiHaptics.tick(view); importFilter = "issues" },
                                    label = { Text("${stringResource(R.string.traffic_fare_filter_issues)} (${parsed.issues.size})") },
                                )
                            }
                        }
                        item {
                            FilterChip(
                                selected = importFilter == "all",
                                onClick = { UiHaptics.tick(view); importFilter = "all" },
                                label = { Text("${stringResource(R.string.traffic_fare_filter_all)} (${parsed.rows.size})") },
                            )
                        }
                    }

                    val visibleRows = if (importFilter == "all") parsed.rows else changeRows
                    LazyColumn(
                        Modifier
                            .fillMaxWidth()
                            .weight(1f, fill = false)
                            .heightIn(max = 320.dp),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                    ) {
                        if (importFilter == "issues") {
                            items(parsed.issues.take(25)) { issue ->
                                val msg = when (issue.kind) {
                                    FareIssueKind.CONFLICT -> stringResource(R.string.traffic_fare_issue_conflict, issue.row, issue.col, issue.route.orEmpty())
                                    FareIssueKind.INVALID_AMOUNT -> stringResource(R.string.traffic_fare_issue_amount, issue.row, issue.col, issue.route.orEmpty(), issue.raw.orEmpty())
                                    FareIssueKind.INVALID_AREA_NAME -> stringResource(R.string.traffic_fare_issue_area, issue.row, issue.col, issue.raw.orEmpty())
                                    FareIssueKind.MISSING_AREA -> stringResource(R.string.traffic_fare_issue_missing_area, issue.row, issue.col)
                                }
                                Surface(
                                    color = statusColors.dangerSoft,
                                    contentColor = statusColors.danger,
                                    shape = MaterialTheme.shapes.small,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(msg, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(Spacing.xs))
                                }
                            }
                        }
                        if (importFilter != "issues") {
                            items(visibleRows.take(80)) { row ->
                                val effectiveStatus = if (row.status == FareImportRowStatus.UNKNOWN_AREA && canCreateAreas && createMissingAreas) {
                                    FareImportRowStatus.NEW
                                } else {
                                    row.status
                                }
                                val (statusText, statusKind) = when (effectiveStatus) {
                                    FareImportRowStatus.NEW -> stringResource(R.string.traffic_fare_row_new) to RideStatusKind.ACTIVE
                                    FareImportRowStatus.UPDATE -> stringResource(R.string.traffic_fare_row_update) to RideStatusKind.PENDING
                                    FareImportRowStatus.UNCHANGED -> stringResource(R.string.traffic_fare_row_same) to RideStatusKind.DONE
                                    FareImportRowStatus.UNKNOWN_AREA -> stringResource(R.string.traffic_fare_row_skipped) to RideStatusKind.CANCELLED
                                }
                                OutlinedCard(
                                    modifier = Modifier.fillMaxWidth(),
                                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                ) {
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(Modifier.weight(1f).padding(end = Spacing.xs)) {
                                            Text(
                                                "${row.areaAName} ↔ ${row.areaBName}",
                                                style = MaterialTheme.typography.bodyMedium,
                                                fontWeight = FontWeight.Medium,
                                            )
                                            Row(
                                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                                verticalAlignment = Alignment.CenterVertically,
                                            ) {
                                                row.previousCents?.takeIf { effectiveStatus == FareImportRowStatus.UPDATE }?.let { old ->
                                                    Text(
                                                        Money.rm(old),
                                                        style = MaterialTheme.typography.bodySmall.tabular().copy(
                                                            textDecoration = TextDecoration.LineThrough,
                                                        ),
                                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                    )
                                                }
                                                Text(
                                                    Money.rm(row.amountCents),
                                                    style = MaterialTheme.typography.bodySmall.tabular().copy(fontWeight = FontWeight.SemiBold),
                                                    color = when (effectiveStatus) {
                                                        FareImportRowStatus.NEW -> statusColors.success
                                                        FareImportRowStatus.UPDATE -> statusColors.warning
                                                        else -> MaterialTheme.colorScheme.onSurfaceVariant
                                                    },
                                                )
                                            }
                                        }
                                        StatusPill(text = statusText, kind = statusKind)
                                    }
                                }
                            }
                        }
                    }
                }

                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    OutlinedButton(
                        onClick = { showImportDialog = false },
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.common_cancel))
                    }
                    Button(
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.bulkImportFares(applicable, canCreateAreas && createMissingAreas) { res ->
                                res.onSuccess { done ->
                                    importStatusBanner = context.getString(
                                        R.string.traffic_fare_import_done,
                                        done.created,
                                        done.updated,
                                    )
                                    showImportDialog = false
                                }
                            }
                        },
                        enabled = canApply,
                        colors = graceRidePrimaryButtonColors(),
                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                    ) {
                        Text(stringResource(R.string.traffic_fare_import_apply, applicable.size))
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun AreasManagePane(
    state: TrafficHubUiState,
    vm: TrafficHubViewModel,
    bottomClearance: Dp,
) {
    var editingArea by remember { mutableStateOf<Area?>(null) }
    var showCreate by remember { mutableStateOf(false) }
    var areaName by remember { mutableStateOf("") }
    var areaActive by remember { mutableStateOf(true) }

    PullToRefreshBox(
        isRefreshing = state.refreshing,
        onRefresh = { vm.refreshActive(silent = false) },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                start = Spacing.sm,
                end = Spacing.sm,
                top = Spacing.xs,
                bottom = Spacing.lg + bottomClearance,
            ),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            item {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    Button(
                        onClick = {
                            areaName = ""
                            areaActive = true
                            showCreate = true
                        },
                        enabled = !state.areasBusy,
                    ) {
                        Icon(Icons.Outlined.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(Spacing.xxs))
                        Text(stringResource(R.string.traffic_area_add))
                    }
                }
            }
            items(state.areas, key = { "area-m-${it.id}" }) { area ->
                OutlinedCard(
                    onClick = {
                        areaName = area.name
                        areaActive = area.active
                        editingArea = area
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .animateItem(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Row(
                        Modifier.fillMaxWidth().padding(Spacing.sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            modifier = Modifier.weight(1f),
                        ) {
                            Icon(Icons.Outlined.LocationOn, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                            Column {
                                Text(area.name, style = MaterialTheme.typography.titleMedium)
                                Text(
                                    stringResource(R.string.traffic_area_photos_count, area.photoCount),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        StatusPill(
                            text = stringResource(if (area.active) R.string.traffic_area_active else R.string.traffic_area_inactive),
                            kind = if (area.active) RideStatusKind.ACTIVE else RideStatusKind.CANCELLED,
                        )
                    }
                }
            }
        }
    }

    if (showCreate || editingArea != null) {
        val current = editingArea
        AlertDialog(
            onDismissRequest = {
                showCreate = false
                editingArea = null
            },
            title = {
                Text(stringResource(if (current == null) R.string.traffic_area_add else R.string.traffic_area_edit))
            },
            text = {
                Column(
                    Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    OutlinedTextField(
                        value = areaName,
                        onValueChange = { areaName = it },
                        label = { Text(stringResource(R.string.traffic_area_name_hint)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(stringResource(R.string.traffic_area_active), style = MaterialTheme.typography.bodyMedium)
                        Switch(checked = areaActive, onCheckedChange = { areaActive = it })
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (areaName.isNotBlank()) {
                            if (current == null) vm.createArea(areaName, areaActive)
                            else vm.updateArea(current.id, areaName, areaActive)
                            showCreate = false
                            editingArea = null
                        }
                    },
                    enabled = areaName.isNotBlank(),
                ) { Text(stringResource(R.string.common_save)) }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        showCreate = false
                        editingArea = null
                    },
                ) { Text(stringResource(R.string.common_cancel)) }
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun AreaDropdownField(
    label: String,
    areas: List<Area>,
    selectedId: Long?,
    onSelect: (Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    val selectedName = areas.firstOrNull { it.id == selectedId }?.name.orEmpty()
    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = !expanded },
        modifier = modifier.fillMaxWidth(),
    ) {
        OutlinedTextField(
            value = selectedName,
            onValueChange = {},
            readOnly = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                .fillMaxWidth(),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            areas.forEach { area ->
                DropdownMenuItem(
                    text = { Text(area.name) },
                    onClick = {
                        onSelect(area.id)
                        expanded = false
                    },
                )
            }
        }
    }
}

internal fun parseRmToCents(raw: String): Long? {
    val clean = raw.trim()
    if (clean.isEmpty()) return null
    return runCatching {
        BigDecimal(clean).movePointRight(2).longValueExact()
    }.getOrNull() ?: runCatching {
        BigDecimal(clean).movePointRight(2).toLong()
    }.getOrNull()
}
