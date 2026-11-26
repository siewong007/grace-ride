package org.pjstatecm.graceride.ui.screens.events

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.platform.CheckinParseResult
import org.pjstatecm.graceride.platform.QrCodeCanvas
import org.pjstatecm.graceride.platform.launchNativeQrScanner
import org.pjstatecm.graceride.platform.parseCheckinInput
import org.pjstatecm.graceride.platform.parseMemberPassInput
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.CalloutBanner
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventPassQrSheet(
    eventId: Long,
    passCode: String,
    eventTitle: String,
    onDismiss: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.lg),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Text(eventTitle, style = MaterialTheme.typography.titleLarge)
            Text(stringResource(R.string.event_pass_label), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
            QrCodeCanvas(
                content = "GRPASS:$eventId:$passCode",
                modifier = Modifier.size(220.dp),
            )
            Text(passCode, style = MaterialTheme.typography.headlineSmall)
            Text(
                stringResource(R.string.event_pass_hint),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            OutlinedButton(
                onClick = onDismiss,
                modifier = Modifier.fillMaxWidth().height(48.dp),
            ) {
                Text(stringResource(R.string.common_close))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventCheckInSheet(
    eventId: Long,
    busy: Boolean,
    error: String? = null,
    scannerModeForOrganiser: Boolean,
    initialCode: String = "",
    onSelfCheckin: (String?) -> Unit,
    onScanMemberPass: (String) -> Unit,
    onClearError: () -> Unit = {},
    onDismiss: () -> Unit,
) {
    org.pjstatecm.graceride.ui.SecureScreenEffect()
    val context = LocalContext.current
    val view = LocalView.current
    var code by remember(initialCode) { mutableStateOf(initialCode) }
    var scannerUnavailable by remember { mutableStateOf(false) }
    var localError by remember { mutableStateOf<String?>(null) }
    val wrongEventMsg = stringResource(R.string.event_checkin_wrong_event)
    val invalidCodeMsg = stringResource(R.string.event_scanner_unavailable)
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    fun submitRaw(raw: String) {
        val parsed = if (scannerModeForOrganiser) {
            parseMemberPassInput(raw, eventId)
        } else {
            parseCheckinInput(raw, eventId)
        }
        when (parsed) {
            is CheckinParseResult.Code -> {
                localError = null
                if (scannerModeForOrganiser) {
                    onScanMemberPass(parsed.value)
                } else {
                    onSelfCheckin(parsed.value)
                }
            }
            CheckinParseResult.WrongEvent -> localError = wrongEventMsg
            CheckinParseResult.Invalid -> localError = invalidCodeMsg
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = Spacing.md)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Text(
                stringResource(if (scannerModeForOrganiser) R.string.event_scan_member_btn else R.string.event_checkin_btn),
                style = MaterialTheme.typography.titleLarge,
            )
            (localError ?: error)?.let { errMsg ->
                CalloutBanner(
                    icon = Icons.Outlined.QrCodeScanner,
                    text = errMsg,
                    containerColor = MaterialTheme.colorScheme.errorContainer,
                    contentColor = MaterialTheme.colorScheme.onErrorContainer,
                )
            }
            Button(
                enabled = !busy,
                onClick = {
                    UiHaptics.tick(view)
                    localError = null
                    onClearError()
                    launchNativeQrScanner(
                        context = context,
                        onResult = ::submitRaw,
                        onUnavailable = { scannerUnavailable = true },
                    )
                },
                modifier = Modifier.fillMaxWidth().height(52.dp),
            ) {
                Icon(Icons.Outlined.QrCodeScanner, contentDescription = null, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(Spacing.xs))
                Text(stringResource(R.string.event_scan_qr))
            }
            if (scannerUnavailable) {
                Text(
                    stringResource(R.string.event_scanner_unavailable),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            OutlinedTextField(
                value = code,
                onValueChange = {
                    code = it
                    localError = null
                    onClearError()
                },
                label = {
                    Text(
                        stringResource(
                            if (scannerModeForOrganiser) R.string.event_member_code_hint else R.string.event_code_hint,
                        ),
                    )
                },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                OutlinedButton(onClick = onDismiss, modifier = Modifier.weight(1f).height(48.dp)) {
                    Text(stringResource(R.string.common_cancel))
                }
                Button(
                    enabled = !busy && code.isNotBlank(),
                    onClick = {
                        UiHaptics.confirm(view)
                        submitRaw(code)
                    },
                    modifier = Modifier.weight(1f).height(48.dp),
                ) {
                    if (busy) {
                        InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                    } else {
                        Text(stringResource(R.string.common_confirm))
                    }
                }
            }
        }
    }
}
