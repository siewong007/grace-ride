package org.pjstatecm.graceride.ui.screens.help

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Image
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun HelpDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
) {
    val vm: HelpDetailViewModel = viewModel(
        key = "help-detail-$id",
        factory = HelpDetailViewModel.factory(container, id),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val isStaff = me?.can("admin_help") == true
    val canUpdate = me?.can("help", "update") == true || me?.can("admin_help", "update") == true
    val snackbar = remember { SnackbarHostState() }
    val context = LocalContext.current
    val view = LocalView.current
    var zoomedFileId by remember { mutableStateOf<Long?>(null) }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) vm.uploadAttachment(context, uri)
    }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(state.ticket?.subject ?: stringResource(R.string.help_thread)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading -> DetailSkeleton(Modifier.padding(padding))
            state.ticket == null -> ErrorView(state.error ?: "", onRetry = vm::load, modifier = Modifier.padding(padding))
            else -> {
                val tk = state.ticket!!
                val isResolved = tk.status == "closed"
                Column(
                    Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .imePadding()
                        .navigationBarsPadding(),
                ) {
                    Surface(color = MaterialTheme.colorScheme.surfaceContainerLow, modifier = Modifier.fillMaxWidth()) {
                        Column(
                            Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                        ) {
                            FlowRow(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                            ) {
                                Box(Modifier.defaultMinSize(minHeight = 40.dp), contentAlignment = Alignment.CenterStart) {
                                    StatusPill(text = ticketStatusLabel(tk.status), kind = ticketStatusKind(tk.status))
                                }
                                tk.rideId?.let { rid ->
                                    TextButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            nav.navigate(AppRoute.RideDetail(rid))
                                        },
                                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.DirectionsCar, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text("#$rid")
                                    }
                                }
                                if (isStaff && !isResolved) {
                                    val claimedByMe = tk.assigneeId == me.id
                                    TextButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.claim(!claimedByMe)
                                        },
                                        enabled = !state.busy,
                                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Text(
                                            stringResource(
                                                if (claimedByMe) R.string.help_unclaim else R.string.help_claim,
                                            ),
                                        )
                                    }
                                }
                                if (canUpdate) {
                                    TextButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            if (isResolved) vm.reopen() else vm.close()
                                        },
                                        enabled = !state.busy,
                                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.CheckCircle, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(
                                            stringResource(
                                                if (isResolved) R.string.help_reopen else R.string.help_resolve,
                                            ),
                                        )
                                    }
                                }
                            }
                            tk.assigneeName?.takeIf { it.isNotBlank() }?.let { a ->
                                Text(
                                    stringResource(R.string.help_assigned_to, a),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }

                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.weight(1f),
                    ) {
                        LazyColumn(
                            Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            items(tk.replies, key = { it.id }) { rep ->
                                val isMine = me?.id != null && rep.userId == me.id
                                val bubbleShape = RoundedCornerShape(
                                    topStart = 16.dp,
                                    topEnd = 16.dp,
                                    bottomStart = if (isMine) 16.dp else 4.dp,
                                    bottomEnd = if (isMine) 4.dp else 16.dp,
                                )
                                val containerColor = when {
                                    isMine -> MaterialTheme.colorScheme.primaryContainer
                                    rep.staff -> MaterialTheme.colorScheme.secondaryContainer
                                    else -> MaterialTheme.colorScheme.surfaceContainerHigh
                                }
                                val contentColor = when {
                                    isMine -> MaterialTheme.colorScheme.onPrimaryContainer
                                    rep.staff -> MaterialTheme.colorScheme.onSecondaryContainer
                                    else -> MaterialTheme.colorScheme.onSurface
                                }
                                Box(
                                    modifier = Modifier.fillMaxWidth(),
                                    contentAlignment = if (isMine) Alignment.CenterEnd else Alignment.CenterStart,
                                ) {
                                    Surface(
                                        color = containerColor,
                                        contentColor = contentColor,
                                        shape = bubbleShape,
                                        modifier = Modifier.fillMaxWidth(0.88f),
                                    ) {
                                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                                Text(
                                                    buildString {
                                                        append(rep.name)
                                                        if (rep.staff) append(" · ${stringResource(R.string.help_staff_tag)}")
                                                    },
                                                    style = MaterialTheme.typography.labelLarge,
                                                    fontWeight = FontWeight.SemiBold,
                                                    color = if (rep.staff && !isMine) MaterialTheme.colorScheme.primary else contentColor,
                                                )
                                                Text(
                                                    KlTime.display(rep.createdAt),
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = contentColor.copy(alpha = 0.75f),
                                                )
                                            }
                                            if (rep.body.isNotBlank()) {
                                                Text(rep.body, style = MaterialTheme.typography.bodyMedium)
                                            }
                                            rep.fileId?.let { fid ->
                                                OutlinedCard(
                                                    onClick = { zoomedFileId = fid },
                                                    modifier = Modifier.padding(top = Spacing.xxs),
                                                ) {
                                                    AsyncImage(
                                                        model = container.fileUrl(fid),
                                                        contentDescription = stringResource(R.string.help_attachment),
                                                        contentScale = ContentScale.Crop,
                                                        modifier = Modifier
                                                            .fillMaxWidth()
                                                            .height(160.dp),
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if (!isResolved && canUpdate) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            if (state.attachmentFileId != null) {
                                InputChip(
                                    selected = true,
                                    onClick = {
                                        UiHaptics.tick(view)
                                        vm.clearAttachment()
                                    },
                                    label = { Text(stringResource(R.string.help_photo_attached)) },
                                    leadingIcon = {
                                        Icon(Icons.Outlined.Image, contentDescription = null, modifier = Modifier.size(18.dp))
                                    },
                                    trailingIcon = {
                                        Icon(
                                            Icons.Outlined.Close,
                                            contentDescription = stringResource(R.string.a11y_clear),
                                            modifier = Modifier.size(18.dp),
                                        )
                                    },
                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                )
                            }
                            Row(
                                Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                IconButton(
                                    onClick = { picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) },
                                    enabled = !state.uploading && !state.busy,
                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
                                ) {
                                    if (state.uploading) InlineButtonSpinner()
                                    else Icon(Icons.Outlined.AttachFile, contentDescription = stringResource(R.string.help_attach_photo))
                                }
                                OutlinedTextField(
                                    value = state.replyBody,
                                    onValueChange = vm::setReplyBody,
                                    placeholder = { Text(stringResource(R.string.help_reply_placeholder)) },
                                    modifier = Modifier.weight(1f),
                                )
                                FilledIconButton(
                                    onClick = {
                                        UiHaptics.confirm(view)
                                        vm.sendReply()
                                    },
                                    enabled = !state.busy && !state.uploading && (state.replyBody.isNotBlank() || state.attachmentFileId != null),
                                    modifier = Modifier.defaultMinSize(minHeight = 48.dp, minWidth = 48.dp),
                                ) {
                                    if (state.busy) InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                                    else Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = stringResource(R.string.help_send))
                                }
                            }
                        }
                    } else if (isResolved && canUpdate) {
                        OutlinedButton(
                            onClick = {
                                UiHaptics.tick(view)
                                vm.reopen()
                            },
                            enabled = !state.busy,
                            modifier = Modifier.fillMaxWidth().padding(Spacing.sm).defaultMinSize(minHeight = 48.dp),
                        ) {
                            Text(stringResource(R.string.help_reopen))
                        }
                    }
                }
            }
        }
    }

    zoomedFileId?.let { fid ->
        ZoomableImageDialog(
            model = container.fileUrl(fid),
            title = stringResource(R.string.help_attachment),
            onDismiss = { zoomedFileId = null },
        )
    }
}
