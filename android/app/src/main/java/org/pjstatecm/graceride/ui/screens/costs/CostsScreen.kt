package org.pjstatecm.graceride.ui.screens.costs

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.FilterChip
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.CheckCircleOutline
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.PhotoCamera
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.ShareCard
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.copyToClipboard
import org.pjstatecm.graceride.ui.theme.GrPrimary
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CostsScreen(container: AppContainer, nav: NavHostController) {
    val vm: CostsViewModel = viewModel(factory = CostsViewModel.factory(container))
    SecureScreenEffect()
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? org.pjstatecm.graceride.data.repo.AuthState.SignedIn)?.me ?: container.auth.me
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val doneMsg = stringResource(R.string.costs_submitted)
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }

    androidx.compose.runtime.DisposableEffect(lifecycleOwner) {
        val obs = androidx.lifecycle.LifecycleEventObserver { _, event ->
            if (event == androidx.lifecycle.Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.refresh() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        vm.onProofPicked(context, uri)
    }

    LaunchedEffect(state.doneMessage) {
        if (state.doneMessage != null) {
            snackbar.showSnackbar(doneMsg)
            vm.clearDone()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.costs_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        val viewMode = when {
            state.loading -> "loading"
            state.error != null && state.shares.isEmpty() -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = viewMode,
            animationSpec = tween(200),
            label = "costs_state",
            modifier = Modifier.padding(padding),
        ) { mode ->
            when (mode) {
                "loading" -> SkeletonLoadingList(count = 3)
                "error" -> ErrorView(state.error ?: "", onRetry = vm::refresh)
                else -> Column(Modifier.fillMaxSize()) {
                    AnimatedOfflineBanner(
                        visible = state.offline,
                        modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
                    )
                    val dueShares = state.shares.filter { it.status == "unpaid" || it.status == "needs_info" }
                    val totalDueCents = dueShares.sumOf { it.amountCents }
                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.weight(1f),
                    ) {
                        LazyColumn(
                            Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            contentPadding = PaddingValues(vertical = Spacing.xs),
                        ) {
                            if (dueShares.isNotEmpty()) {
                                item(key = "due-summary") {
                                    OutlinedCard(
                                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Row(
                                            Modifier
                                                .fillMaxWidth()
                                                .height(IntrinsicSize.Min),
                                        ) {
                                            Box(
                                                Modifier
                                                    .width(3.dp)
                                                    .fillMaxHeight()
                                                    .background(GrPrimary),
                                            )
                                            Column(
                                                Modifier.weight(1f).padding(Spacing.sm),
                                                verticalArrangement = Arrangement.spacedBy(Spacing.xxs),
                                            ) {
                                                Text(
                                                    stringResource(R.string.costs_total_due),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                                Text(
                                                    Money.rm(totalDueCents),
                                                    style = MaterialTheme.typography.headlineSmall.tabular(),
                                                    color = MaterialTheme.colorScheme.onSurface,
                                                )
                                                Text(
                                                    stringResource(R.string.costs_items_due, dueShares.size),
                                                    style = MaterialTheme.typography.bodyMedium.tabular(),
                                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                            if (state.shares.isEmpty()) {
                                item {
                                    EmptyView(
                                        text = stringResource(R.string.costs_empty),
                                        icon = Icons.Outlined.CheckCircleOutline,
                                    )
                                }
                            }
                            items(state.shares, key = { it.id }) { share ->
                                ShareCard(
                                    share,
                                    canPay = me?.can("costs", "update") == true,
                                    onPay = { vm.openPay(share) },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    val paying = state.paying
    if (paying != null) {
        val canDismissSheet by androidx.compose.runtime.rememberUpdatedState(!state.submitting)
        val sheetState = rememberModalBottomSheetState(
            skipPartiallyExpanded = true,
            confirmValueChange = { canDismissSheet },
        )
        var accountCopied by remember(paying.id) { mutableStateOf(false) }
        ModalBottomSheet(onDismissRequest = { if (!state.submitting) vm.closePay() }, sheetState = sheetState) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .imePadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = Spacing.md)
                    .padding(bottom = Spacing.md),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Text(
                    Money.rm(paying.amountCents),
                    style = MaterialTheme.typography.headlineMedium.tabular(),
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Text(
                    paying.title,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                OutlinedCard(Modifier.fillMaxWidth()) {
                    Row(
                        Modifier.fillMaxWidth().padding(Spacing.sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(
                                stringResource(R.string.costs_payee),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Text(
                                paying.payeeName,
                                style = MaterialTheme.typography.titleSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                            if (paying.payeeAccount.isNotBlank()) {
                                Text(
                                    paying.payeeAccount,
                                    style = MaterialTheme.typography.bodyMedium.tabular(),
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        if (paying.payeeAccount.isNotBlank()) {
                            TextButton(
                                onClick = {
                                    UiHaptics.tick(view)
                                    if (copyToClipboard(context, "Payee account", paying.payeeAccount, sensitive = true)) {
                                        accountCopied = true
                                    }
                                },
                            ) {
                                Icon(Icons.Outlined.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text(
                                    stringResource(
                                        if (accountCopied) R.string.common_copied else R.string.costs_copy_account,
                                    ),
                                )
                            }
                        }
                    }
                }

                paying.payeeQrFileId?.let { qrId ->
                    Text(stringResource(R.string.costs_qr), style = MaterialTheme.typography.titleSmall)
                    Surface(
                        color = Color.White,
                        shape = MaterialTheme.shapes.medium,
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        AsyncImage(
                            model = container.fileUrl(qrId),
                            contentDescription = stringResource(R.string.costs_qr),
                            modifier = Modifier.fillMaxWidth().height(280.dp).padding(12.dp),
                            contentScale = ContentScale.Fit,
                        )
                    }
                }

                Text(
                    stringResource(R.string.costs_proof_hint),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                OutlinedTextField(
                    value = state.txnRef,
                    onValueChange = vm::setTxnRef,
                    label = { Text(stringResource(R.string.costs_txn_ref)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                if (state.ocrExtracting) {
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        LinearProgressIndicator(Modifier.weight(1f))
                        Text(
                            stringResource(R.string.costs_ocr_detecting),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else if (state.ocrCandidates.isNotEmpty()) {
                    Text(
                        stringResource(R.string.costs_ocr_suggestions),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    @OptIn(ExperimentalLayoutApi::class)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                        state.ocrCandidates.forEach { cand ->
                            FilterChip(
                                selected = state.txnRef.trim() == cand,
                                onClick = { vm.setTxnRef(cand) },
                                label = { Text(cand) },
                            )
                        }
                    }
                }

                state.proofUri?.let { uri ->
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceContainerLow,
                        shape = MaterialTheme.shapes.medium,
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        AsyncImage(
                            model = uri,
                            contentDescription = stringResource(R.string.costs_proof_photo),
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(140.dp)
                                .clip(MaterialTheme.shapes.medium),
                            contentScale = ContentScale.Crop,
                        )
                    }
                }

                OutlinedButton(
                    onClick = {
                        pickImage.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                    },
                    modifier = Modifier.fillMaxWidth().height(48.dp),
                ) {
                    Icon(Icons.Outlined.PhotoCamera, contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(Spacing.xs))
                    Text(
                        stringResource(
                            if (state.proofUri == null) R.string.costs_upload else R.string.costs_change_photo,
                        ),
                    )
                }

                val sheetErrText = state.sheetError ?: state.sheetErrorRes?.let { stringResource(it) }
                sheetErrText?.let {
                    CalloutBanner(
                        icon = Icons.Outlined.WarningAmber,
                        text = it,
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                        contentColor = MaterialTheme.colorScheme.onErrorContainer,
                    )
                }

                Button(
                    onClick = {
                        UiHaptics.confirm(view)
                        vm.submitProof(context)
                    },
                    enabled = !state.submitting,
                    modifier = Modifier.fillMaxWidth().height(52.dp),
                ) {
                    if (state.submitting) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.costs_submit_proof))
                    }
                }
                TextButton(
                    onClick = { vm.closePay() },
                    enabled = !state.submitting,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(stringResource(R.string.common_cancel))
                }
            }
        }
    }
}
