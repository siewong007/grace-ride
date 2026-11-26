package org.pjstatecm.graceride.ui.screens.help

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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.ThumbDown
import androidx.compose.material.icons.outlined.ThumbUp
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
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
import java.util.Locale
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.RideStatusKind
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.NotifDestination
import org.pjstatecm.graceride.ui.NotifLinkRouter
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.DetailSkeleton
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.StatusPill
import org.pjstatecm.graceride.ui.components.ZoomableImageDialog
import org.pjstatecm.graceride.ui.navigateRoute
import org.pjstatecm.graceride.ui.openInWebApp
import org.pjstatecm.graceride.ui.theme.Spacing
import org.pjstatecm.graceride.ui.theme.graceRidePrimaryButtonColors
import org.pjstatecm.graceride.ui.theme.tabular

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HelpGuideDetailScreen(
    container: AppContainer,
    nav: NavHostController,
    key: String,
) {
    val vm: HelpGuideDetailViewModel = viewModel(
        key = "guide-$key",
        factory = HelpGuideDetailViewModel.factory(container, key),
    )
    val state by vm.state.collectAsStateWithLifecycle()
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val me = (authState as? AuthState.SignedIn)?.me ?: container.auth.me
    val canEdit = me?.can("admin_help", "update") == true
    val isZh = Locale.getDefault().language == "zh"
    val context = LocalContext.current
    val view = LocalView.current
    val snackbar = remember { SnackbarHostState() }
    val thanksMsg = stringResource(R.string.help_feedback_thanks)

    var zoomedStepFileId by remember { mutableStateOf<Long?>(null) }
    var zoomedStepTitle by remember { mutableStateOf("") }

    LaunchedEffect(state.message) {
        state.message?.let {
            snackbar.showSnackbar(it)
            vm.clearMessage()
        }
    }

    val title = state.guide?.let { g ->
        if (isZh) g.titleZh.ifBlank { g.titleEn } else g.titleEn.ifBlank { g.titleZh }
    } ?: stringResource(R.string.help_tab_guides)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
                actions = {
                    if (canEdit && state.guide != null) {
                        IconButton(
                            onClick = {
                                UiHaptics.tick(view)
                                nav.navigate(AppRoute.HelpGuideEditor(key = state.guide!!.slug.ifBlank { state.guide!!.id.toString() }))
                            },
                        ) {
                            Icon(Icons.Outlined.Edit, contentDescription = stringResource(R.string.help_guide_edit))
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            state.loading -> DetailSkeleton(Modifier.padding(padding))
            state.guide == null -> ErrorView(state.error ?: "", onRetry = vm::load, modifier = Modifier.padding(padding))
            else -> {
                val g = state.guide!!
                val summary = if (isZh) g.summaryZh.ifBlank { g.summaryEn } else g.summaryEn.ifBlank { g.summaryZh }
                Column(
                    Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .verticalScroll(rememberScrollState())
                        .navigationBarsPadding()
                        .padding(Spacing.sm),
                    verticalArrangement = Arrangement.spacedBy(0.dp),
                ) {
                    if (!g.published) {
                        StatusPill(text = stringResource(R.string.help_guide_draft), kind = RideStatusKind.PENDING)
                        Spacer(Modifier.height(Spacing.xs))
                    }
                    if (summary.isNotBlank()) {
                        Text(summary, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Spacer(Modifier.height(Spacing.sm))
                    }

                    g.steps.forEachIndexed { idx, step ->
                        val stepTitle = if (isZh) step.titleZh.ifBlank { step.titleEn } else step.titleEn.ifBlank { step.titleZh }
                        val stepBody = if (isZh) step.bodyZh.ifBlank { step.bodyEn } else step.bodyEn.ifBlank { step.bodyZh }
                        val noteText = if (isZh) step.noteZh.orEmpty().ifBlank { step.noteEn.orEmpty() } else step.noteEn.orEmpty().ifBlank { step.noteZh.orEmpty() }
                        val dest = remember(step.link, me) {
                            if (step.link.isBlank()) NotifDestination.None
                            else if (me != null) NotifLinkRouter.resolveFor(step.link, me)
                            else NotifLinkRouter.resolve(step.link)
                        }
                        val isNativeDest = dest is NotifDestination.Native

                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                                ) {
                                    Surface(
                                        color = MaterialTheme.colorScheme.primaryContainer,
                                        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                                        shape = CircleShape,
                                        modifier = Modifier.size(30.dp),
                                    ) {
                                        Box(contentAlignment = Alignment.Center) {
                                            Text(
                                                "${idx + 1}",
                                                style = MaterialTheme.typography.labelLarge.tabular(),
                                                fontWeight = FontWeight.Bold,
                                            )
                                        }
                                    }
                                    Text(
                                        stepTitle.ifBlank { stringResource(R.string.help_guide_step_number, idx + 1) },
                                        style = MaterialTheme.typography.titleMedium,
                                        modifier = Modifier.weight(1f),
                                    )
                                }
                                if (stepBody.isNotBlank()) {
                                    Text(stepBody, style = MaterialTheme.typography.bodyMedium)
                                }
                                step.imageFileId?.let { fid ->
                                    OutlinedCard(
                                        onClick = {
                                            zoomedStepTitle = stepTitle
                                            zoomedStepFileId = fid
                                        },
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        AsyncImage(
                                            model = container.fileUrl(fid),
                                            contentDescription = stepTitle,
                                            contentScale = ContentScale.Fit,
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .height(180.dp)
                                                .clip(RoundedCornerShape(8.dp)),
                                        )
                                    }
                                }
                                if (noteText.isNotBlank()) {
                                    Surface(
                                        color = MaterialTheme.colorScheme.secondaryContainer,
                                        shape = MaterialTheme.shapes.small,
                                        modifier = Modifier.fillMaxWidth(),
                                    ) {
                                        Text(
                                            noteText,
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSecondaryContainer,
                                            modifier = Modifier.padding(Spacing.xs),
                                        )
                                    }
                                }
                                if (step.link.isNotBlank() && dest !is NotifDestination.None) {
                                    FilledTonalButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            when (dest) {
                                                is NotifDestination.Native -> nav.navigateRoute(dest.route)
                                                is NotifDestination.Web -> openInWebApp(context, container.webUrl(dest.path))
                                                NotifDestination.None -> {}
                                            }
                                        },
                                        modifier = Modifier.defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(
                                            imageVector = if (isNativeDest) Icons.AutoMirrored.Filled.ArrowForward else Icons.AutoMirrored.Outlined.OpenInNew,
                                            contentDescription = null,
                                            modifier = Modifier.size(16.dp),
                                        )
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(
                                            stringResource(
                                                if (isNativeDest) R.string.help_guide_open_in_app else R.string.help_guide_open_link,
                                            ),
                                        )
                                    }
                                }
                            }
                        }

                        if (idx < g.steps.lastIndex) {
                            Box(
                                Modifier
                                    .padding(start = 28.dp)
                                    .width(2.dp)
                                    .height(14.dp)
                                    .background(MaterialTheme.colorScheme.outlineVariant),
                            )
                        }
                    }

                    Spacer(Modifier.height(Spacing.sm))

                    OutlinedCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(Spacing.sm), verticalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                            Text(stringResource(R.string.help_feedback_prompt), style = MaterialTheme.typography.titleSmall)
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                                val yesSelected = state.myFeedback == true
                                val noSelected = state.myFeedback == false
                                if (yesSelected) {
                                    Button(
                                        onClick = {
                                            UiHaptics.confirm(view)
                                            vm.sendFeedback(true, thanksMsg)
                                        },
                                        enabled = !state.feedbackBusy,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.ThumbUp, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.help_feedback_yes))
                                    }
                                } else {
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.confirm(view)
                                            vm.sendFeedback(true, thanksMsg)
                                        },
                                        enabled = !state.feedbackBusy,
                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.ThumbUp, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.help_feedback_yes))
                                    }
                                }
                                if (noSelected) {
                                    Button(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.sendFeedback(false, thanksMsg)
                                        },
                                        enabled = !state.feedbackBusy,
                                        colors = graceRidePrimaryButtonColors(),
                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.ThumbDown, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.help_feedback_no))
                                    }
                                } else {
                                    OutlinedButton(
                                        onClick = {
                                            UiHaptics.tick(view)
                                            vm.sendFeedback(false, thanksMsg)
                                        },
                                        enabled = !state.feedbackBusy,
                                        modifier = Modifier.weight(1f).defaultMinSize(minHeight = 48.dp),
                                    ) {
                                        Icon(Icons.Outlined.ThumbDown, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(Spacing.xxs))
                                        Text(stringResource(R.string.help_feedback_no))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    zoomedStepFileId?.let { fid ->
        ZoomableImageDialog(
            model = container.fileUrl(fid),
            title = zoomedStepTitle.ifBlank { title },
            onDismiss = { zoomedStepFileId = null },
        )
    }
}
