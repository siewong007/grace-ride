package org.pjstatecm.graceride.ui.screens.login

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.BuildConfig
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.theme.Spacing

data class UnlockUiState(val busy: Boolean = false, val error: String? = null, val useBackup: Boolean = false)

class UnlockViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(UnlockUiState())
    val state: StateFlow<UnlockUiState> = _state.asStateFlow()

    fun toggleBackup() { _state.value = _state.value.copy(useBackup = !_state.value.useBackup, error = null) }

    fun unlock(code: String) {
        if (_state.value.busy || code.isBlank()) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.auth.unlock(code.trim(), _state.value.useBackup)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun devUnlock() {
        viewModelScope.launch {
            try { container.auth.devUnlock() } catch (_: ApiException) {}
        }
    }

    fun signOut() { viewModelScope.launch { container.auth.logout() } }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { UnlockViewModel(container) } }
    }
}

@Composable
fun UnlockScreen(container: AppContainer) {
    val vm: UnlockViewModel = viewModel(factory = UnlockViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    var code by remember { mutableStateOf("") }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = Spacing.md),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
    ) {
        Text(stringResource(R.string.unlock_title), style = MaterialTheme.typography.headlineSmall, color = MaterialTheme.colorScheme.onSurface)
        Text(
            stringResource(R.string.unlock_subtitle),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = Spacing.xs),
        )
        OutlinedTextField(
            value = code,
            onValueChange = { code = it },
            label = { Text(stringResource(if (state.useBackup) R.string.unlock_backup else R.string.unlock_code)) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
            modifier = Modifier.fillMaxWidth().padding(top = Spacing.md),
        )
        state.error?.let {
            Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(top = Spacing.xs))
        }
        Button(
            onClick = { vm.unlock(code) },
            enabled = !state.busy && code.isNotBlank(),
            modifier = Modifier.fillMaxWidth().padding(top = Spacing.sm).height(52.dp),
        ) { Text(stringResource(R.string.unlock_submit)) }
        TextButton(onClick = { vm.toggleBackup() }) {
            Text(stringResource(if (state.useBackup) R.string.unlock_code else R.string.unlock_backup))
        }
        if (BuildConfig.DEBUG) {
            OutlinedButton(onClick = { vm.devUnlock() }, modifier = Modifier.fillMaxWidth().padding(top = Spacing.xs)) {
                Text("Dev unlock")
            }
        }
        TextButton(onClick = { vm.signOut() }, modifier = Modifier.padding(top = Spacing.sm)) {
            Text(stringResource(R.string.profile_logout))
        }
    }
}
