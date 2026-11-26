package org.pjstatecm.graceride.ui.screens.reimbursements

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Replay
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import coil.compose.AsyncImage
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.theme.LocalStatusColors
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReimbursementDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    id: Long,
) {
    val vm: ReimbursementDetailViewModel = viewModel(
        key = "reimb-$id",
        factory = ReimbursementDetailViewModel.factory(container, id),
    )
    SecureScreenEffect()
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val lifecycleOwner = LocalLifecycleOwner.current
    val view = LocalView.current
    var zoomReceipt by remember { mutableStateOf(false) }

    DisposableEffect(lifecycleOwner) {
        var first = true
        val obs = LifecycleEventObserver { _, ev ->
            if (ev == Lifecycle.Event.ON_RESUME) {
                if (first) first = false else vm.refresh()
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    val claim = state.claim
    val canResubmit = !state.loading &&
        claim != null &&
        claim.status == "returned" &&
        claim.kind != "visitor_special" &&
        me?.id == claim.user.id &&
        me.can("reimbursements", "update")

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.reimb_detail_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        bottomBar = {
            if (canResubmit) {
                Surface(tonalElevation = 3.dp, shadowElevation = 4.dp) {
                    Column(
                        Modifier
                            .fillMaxWidth()
                            .navigationBarsPadding()
                            .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
                    ) {
                        Button(
                            onClick = {
                                UiHaptics.tick(view)
                                nav.navigate(AppRoute.ReimbursementForm(editId = claim.id))
                            },
                            colors = graceRidePrimaryButtonColors(),
                            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 48.dp),
                        ) {
                            Icon(Icons.Outlined.Edit, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(Spacing.xs))
                            Text(stringResource(R.string.reimb_resubmit))
                        }
                    }
                }
            }
        },
    ) { padding ->
        val mode = when {
            state.loading -> "loading"
            state.claim == null -> "error"
            else -> "content"
        }
        Crossfade(
            targetState = mode,
            animationSpec = tween(200),
            label = "reimb_detail_state",
            modifier = Modifier.padding(padding),
        ) { current ->
            when (current) {
                "loading" -> DetailSkeleton()
                "error" -> ErrorView(state.error ?: "", onRetry = vm::load)
                else -> {
                    val r = state.claim ?: return@Crossfade
                    val statusColors = LocalStatusColors.current
                    PullToRefreshBox(
                        isRefreshing = state.refreshing,
                        onRefresh = vm::refresh,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        Column(
                            Modifier
                                .fillMaxSize()
                                .verticalScroll(rememberScrollState())
                                .navigationBarsPadding()
                                .padding(Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
                        ) {
                            ReimbursementStatusTimeline(status = r.status)

                            if (r.reviewNote.isNotBlank()) {
                                val bannerBg = if (r.status == "rejected") statusColors.dangerSoft else statusColors.warningSoft
                                val bannerFg = if (r.status == "rejected") statusColors.danger else statusColors.warning
                                Surface(
                                    color = bannerBg,
                                    shape = MaterialTheme.shapes.small,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Row(
                                        Modifier.padding(Spacing.sm),
                                        horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                        verticalAlignment = Alignment.Top,
                                    ) {
                                        Icon(
                                            Icons.Outlined.Info,
                                            contentDescription = null,
                                            tint = bannerFg,
                                            modifier = Modifier.size(18.dp),
                                        )
                                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                            Text(
                                                stringResource(R.string.reimb_review_note),
                                                style = MaterialTheme.typography.labelMedium,
                                                fontWeight = FontWeight.SemiBold,
                                                color = bannerFg,
                                            )
                                            Text(
                                                r.reviewNote,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = bannerFg,
                                            )
                                        }
                                    }
                                }
                            }

                            OutlinedCard(Modifier.fillMaxWidth()) {
                                Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                    Row(
                                        Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Text(
                                            Money.rm(r.amountCents),
                                            style = MaterialTheme.typography.headlineSmall.tabular(),
                                            fontWeight = FontWeight.Bold,
                                        )
                                        StatusPill(
                                            text = reimbStatusLabel(r.status),
                                            kind = reimbStatusKind(r.status),
                                        )
                                    }
                                    Text(
                                        "${reimbKindLabel(r.kind)} · ${r.user.name}",
                                        style = MaterialTheme.typography.labelLarge,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                    if (r.description.isNotBlank()) {
                                        Text(r.description, style = MaterialTheme.typography.bodyMedium)
                                    }
                                    r.specialReason?.takeIf { it.isNotBlank() }?.let { reason ->
                                        Text(reason, style = MaterialTheme.typography.bodyMedium)
                                    }
                                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                                    r.eventTitle?.takeIf { it.isNotBlank() }?.let { ev ->
                                        DetailLine(stringResource(R.string.reimb_event), ev)
                                    }
                                    r.rideRoute?.takeIf { it.isNotBlank() }?.let { rt ->
                                        DetailLine(stringResource(R.string.reimb_ride_route), rt)
                                    }
                                    r.payeeName?.takeIf { it.isNotBlank() }?.let { pn ->
                                        DetailLine(stringResource(R.string.reimb_payee), "$pn (${r.payeePhone.orEmpty()})")
                                    }
                                    r.reviewer?.takeIf { it.isNotBlank() }?.let { rev ->
                                        DetailLine(stringResource(R.string.reimb_reviewer), rev)
                                    }
                                    r.paidRef?.takeIf { it.isNotBlank() }?.let { pref ->
                                        DetailLine(stringResource(R.string.reimb_paid_ref), pref)
                                    }
                                    DetailLine(stringResource(R.string.reimb_submitted_at), KlTime.display(r.createdAt))
                                }
                            }

                            r.receiptFileId?.let { rfid ->
                                OutlinedCard(
                                    onClick = { zoomReceipt = true },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Column(Modifier.padding(Spacing.xs), horizontalAlignment = Alignment.CenterHorizontally) {
                                        AsyncImage(
                                            model = container.fileUrl(rfid),
                                            contentDescription = stringResource(R.string.reimb_receipt),
                                            contentScale = ContentScale.Fit,
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .height(220.dp),
                                        )
                                        Text(
                                            stringResource(R.string.reimb_tap_zoom_receipt),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (zoomReceipt && state.claim?.receiptFileId != null) {
        val fid = state.claim!!.receiptFileId!!
        ZoomableImageDialog(
            model = container.fileUrl(fid),
            title = stringResource(R.string.reimb_receipt),
            onDismiss = { zoomReceipt = false },
        )
    }
}

@Composable
private fun ReimbursementStatusTimeline(status: String) {
    val c = LocalStatusColors.current
    val step2Label = when (status) {
        "returned" -> stringResource(R.string.reimb_step_returned)
        "rejected" -> stringResource(R.string.reimb_step_rejected)
        else -> stringResource(R.string.reimb_step_reviewed)
    }
    val step1Color = c.success
    val step2Color = when (status) {
        "approved", "paid" -> c.success
        "returned" -> c.warning
        "rejected" -> c.danger
        else -> MaterialTheme.colorScheme.primary
    }
    val step3Color = when (status) {
        "paid" -> c.success
        "approved" -> MaterialTheme.colorScheme.primary
        else -> MaterialTheme.colorScheme.outlineVariant
    }

    OutlinedCard(Modifier.fillMaxWidth()) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TimelineNode(
                label = stringResource(R.string.reimb_step_submitted),
                circleColor = step1Color,
                icon = Icons.Outlined.Check,
                numberText = "1",
                filled = true,
                modifier = Modifier.weight(1f),
            )
            HorizontalDivider(
                modifier = Modifier.width(24.dp),
                thickness = 2.dp,
                color = step2Color,
            )
            TimelineNode(
                label = step2Label,
                circleColor = step2Color,
                icon = when (status) {
                    "approved", "paid" -> Icons.Outlined.Check
                    "returned" -> Icons.Outlined.Replay
                    "rejected" -> Icons.Outlined.Close
                    else -> null
                },
                numberText = "2",
                filled = status != "pending",
                modifier = Modifier.weight(1f),
            )
            HorizontalDivider(
                modifier = Modifier.width(24.dp),
                thickness = 2.dp,
                color = if (status == "paid") c.success else MaterialTheme.colorScheme.outlineVariant,
            )
            TimelineNode(
                label = stringResource(R.string.reimb_step_paid),
                circleColor = step3Color,
                icon = if (status == "paid") Icons.Outlined.Check else null,
                numberText = "3",
                filled = status == "paid",
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun TimelineNode(
    label: String,
    circleColor: Color,
    icon: androidx.compose.ui.graphics.vector.ImageVector?,
    numberText: String,
    filled: Boolean,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier, horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Box(
            Modifier
                .size(28.dp)
                .clip(CircleShape)
                .background(if (filled) circleColor else circleColor.copy(alpha = 0.16f)),
            contentAlignment = Alignment.Center,
        ) {
            if (icon != null) {
                Icon(
                    icon,
                    contentDescription = null,
                    tint = if (filled) Color.White else circleColor,
                    modifier = Modifier.size(16.dp),
                )
            } else {
                Text(
                    numberText,
                    style = MaterialTheme.typography.labelSmall.tabular(),
                    fontWeight = FontWeight.Bold,
                    color = if (filled) Color.White else circleColor,
                )
            }
        }
        Text(
            label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
private fun DetailLine(label: String, value: String) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.width(Spacing.sm))
        Text(value, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.Medium)
    }
}
