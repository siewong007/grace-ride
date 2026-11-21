package org.pjstatecm.graceride.ui.screens.login

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
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
    Column(
        Modifier.fillMaxSize().padding(horizontal = Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(stringResource(R.string.pending_title), style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
        Text(
            stringResource(R.string.pending_body),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.xs),
        )
        Button(
            onClick = { scope.launch { container.auth.refresh() } },
            modifier = Modifier.fillMaxWidth().padding(top = Spacing.md),
        ) { Text(stringResource(R.string.common_refresh)) }
        val vm: UnlockViewModel = viewModel(factory = UnlockViewModel.factory(container))
        TextButton(onClick = { vm.signOut() }, modifier = Modifier.padding(top = Spacing.xs)) {
            Text(stringResource(R.string.profile_logout))
        }
    }
}
