package org.pjstatecm.graceride.ui.screens.costs

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.components.OfflineBanner
import org.pjstatecm.graceride.ui.components.ShareCard
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CostsScreen(container: AppContainer, nav: NavHostController) {
    val vm: CostsViewModel = viewModel(factory = CostsViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val snackbar = remember { SnackbarHostState() }
    val doneMsg = stringResource(R.string.costs_submitted)

    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        vm.setProof(uri)
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
        when {
            state.loading -> LoadingView(Modifier.padding(padding))
            state.error != null && state.shares.isEmpty() -> ErrorView(state.error!!, vm::refresh, Modifier.padding(padding))
            else -> PullToRefreshBox(
                isRefreshing = state.refreshing,
                onRefresh = vm::refresh,
                modifier = Modifier.padding(padding),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    contentPadding = PaddingValues(vertical = Spacing.xs),
                ) {
                    if (state.offline) item { OfflineBanner() }
                    if (state.shares.isEmpty()) item { EmptyView(stringResource(R.string.costs_empty)) }
                    items(state.shares, key = { it.id }) { share ->
                        ShareCard(share, onPay = { vm.openPay(share) })
                    }
                }
            }
        }
    }

    val paying = state.paying
    if (paying != null) {
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ModalBottomSheet(onDismissRequest = { vm.closePay() }, sheetState = sheetState) {
            Column(
                Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = Spacing.md),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Text(Money.rm(paying.amountCents), style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
                Text(paying.title, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(
                    stringResource(R.string.costs_payee) + ": " + paying.payeeName +
                        (if (paying.payeeAccount.isNotBlank()) " · " + paying.payeeAccount else ""),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                paying.payeeQrFileId?.let { qrId ->
                    Text(stringResource(R.string.costs_qr), style = MaterialTheme.typography.titleSmall)
                    AsyncImage(
                        model = container.fileUrl(qrId),
                        contentDescription = stringResource(R.string.costs_qr),
                        modifier = Modifier.fillMaxWidth().height(320.dp),
                        contentScale = ContentScale.Fit,
                    )
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
                OutlinedButton(onClick = { pickImage.launch("image/*") }, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        if (state.proofUri == null) stringResource(R.string.costs_upload)
                        else stringResource(R.string.costs_proof_photo) + " ✓"
                    )
                }
                state.sheetError?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                }
                Button(
                    onClick = { vm.submitProof(context) },
                    enabled = !state.submitting,
                    modifier = Modifier.fillMaxWidth().padding(bottom = Spacing.md),
                ) { Text(stringResource(R.string.costs_submit_proof)) }
            }
        }
    }
}
