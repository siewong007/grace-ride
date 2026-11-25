package org.pjstatecm.graceride.ui.screens.login

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.theme.Spacing

@Composable
fun LoginScreen(container: AppContainer) {
    val vm: LoginViewModel = viewModel(factory = LoginViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    var serverField by remember { mutableStateOf(state.serverUrl) }

    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Image(
            painter = painterResource(R.drawable.ic_splash_mark),
            contentDescription = stringResource(R.string.app_name),
            modifier = Modifier.size(96.dp),
        )
        Text(
            stringResource(R.string.app_name),
            style = MaterialTheme.typography.headlineSmall,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(top = Spacing.sm),
        )
        Text(
            stringResource(R.string.login_subtitle),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.xs),
        )
        OutlinedTextField(
            value = state.email,
            onValueChange = vm::setEmail,
            label = { Text(stringResource(R.string.login_email)) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
            modifier = Modifier.fillMaxWidth().padding(top = Spacing.md),
        )
        Button(
            onClick = { vm.signInWithPasskey(context) },
            enabled = !state.signingIn,
            modifier = Modifier.fillMaxWidth().padding(top = Spacing.sm).height(52.dp),
        ) {
            if (state.signingIn) {
                CircularProgressIndicator(Modifier.size(22.dp), strokeWidth = 2.dp, color = MaterialTheme.colorScheme.onPrimary)
            } else {
                Text(stringResource(R.string.login_passkey))
            }
        }
        if (state.noPasskeyHint) {
            Text(
                stringResource(R.string.login_no_passkey),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = Spacing.sm),
            )
        }
        val errText = state.error ?: state.errorRes?.let { stringResource(it) }
        errText?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = Spacing.sm),
            )
        }

        if (state.devAvailable) {
            HorizontalDivider(Modifier.padding(vertical = Spacing.md))
            Text(
                stringResource(R.string.login_dev_title),
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            OutlinedTextField(
                value = serverField,
                onValueChange = { serverField = it },
                label = { Text(stringResource(R.string.login_server_url)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(top = Spacing.xs),
            )
            OutlinedButton(
                onClick = { vm.applyServer(serverField) },
                modifier = Modifier.fillMaxWidth().padding(top = Spacing.xs),
            ) { Text(stringResource(R.string.login_apply_server)) }
            state.devUsers.forEach { user ->
                OutlinedButton(
                    onClick = { vm.devLogin(user.email) },
                    enabled = !state.signingIn,
                    modifier = Modifier.fillMaxWidth().padding(top = Spacing.xs),
                ) {
                    Text(user.name + "  ·  " + user.email)
                }
            }
        }
    }
}
