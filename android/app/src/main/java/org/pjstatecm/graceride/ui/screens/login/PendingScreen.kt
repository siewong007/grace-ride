package org.pjstatecm.graceride.ui.screens.login

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.HourglassTop
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.PendingChecklistCard
import org.pjstatecm.graceride.ui.theme.Spacing

@Composable
fun PendingScreen(container: AppContainer) {
    // Poll gently while staff review the membership.
    LaunchedEffect(Unit) {
        while (true) {
            delay(30_000)
            container.auth.refresh()
        }
    }
    val scope = rememberCoroutineScope()
    var refreshing by remember { mutableStateOf(false) }
    val authState by container.auth.state.collectAsStateWithLifecycle()
    val pendingMe = (authState as? org.pjstatecm.graceride.data.repo.AuthState.Pending)?.me
    val pendingMeId = pendingMe?.id ?: 0L
    val vm: UnlockViewModel = viewModel(
        key = "pending-$pendingMeId",
        factory = UnlockViewModel.factory(container),
    )

    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .statusBarsPadding()
            .navigationBarsPadding()
            .padding(horizontal = Spacing.md, vertical = Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        OutlinedCard(Modifier.fillMaxWidth()) {
            Column(
                Modifier.fillMaxWidth().padding(Spacing.md),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
            ) {
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    shape = CircleShape,
                    modifier = Modifier.size(64.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            Icons.Outlined.HourglassTop,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(30.dp),
                        )
                    }
                }
                Text(
                    stringResource(R.string.pending_title),
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.padding(top = Spacing.xs),
                )
                Text(
                    stringResource(R.string.pending_body),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
                Button(
                    onClick = {
                        if (!refreshing) {
                            refreshing = true
                            scope.launch {
                                try {
                                    container.auth.refresh()
                                } finally {
                                    refreshing = false
                                }
                            }
                        }
                    },
                    enabled = !refreshing,
                    modifier = Modifier.fillMaxWidth().padding(top = Spacing.sm).height(50.dp),
                ) {
                    if (refreshing) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.common_refresh))
                    }
                }
            }
        }
        if (pendingMe != null) {
            PendingChecklistCard(
                me = pendingMe,
                modifier = Modifier.padding(top = Spacing.sm),
            )
        }
        TextButton(onClick = { vm.signOut() }, modifier = Modifier.padding(top = Spacing.sm)) {
            Text(stringResource(R.string.profile_logout))
        }
    }
}
