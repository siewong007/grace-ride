package org.pjstatecm.graceride.ui.screens.rides

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.outlined.ChatBubbleOutline
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material3.AssistChip
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ChatMessageDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.SecureScreenEffect
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.InlineButtonSpinner
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TripChatSheet(
    container: AppContainer,
    rideId: Long,
    onDismiss: () -> Unit,
) {
    val vm: TripChatViewModel = viewModel(
        key = "trip-chat-$rideId",
        factory = TripChatViewModel.factory(container, rideId),
    )
    SecureScreenEffect()
    val state by vm.state.collectAsStateWithLifecycle()
    val view = LocalView.current
    var draft by remember { mutableStateOf("") }
    var reactingMessageId by remember { mutableStateOf<Long?>(null) }
    val listState = rememberLazyListState()
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    LaunchedEffect(state.messages.size) {
        if (state.messages.isNotEmpty()) {
            listState.animateScrollToItem(state.messages.lastIndex)
        }
    }
    androidx.compose.runtime.DisposableEffect(vm) {
        vm.setSheetVisible(true)
        onDispose { vm.setSheetVisible(false) }
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = Spacing.sm)
                .padding(bottom = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.chat_title), style = MaterialTheme.typography.titleLarge)
                    if (state.members.isNotEmpty()) {
                        Text(
                            stringResource(R.string.chat_members_label, state.members.joinToString(", ")),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.common_cancel))
                }
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

            when {
                state.loading -> SkeletonLoadingList(count = 3, modifier = Modifier.heightIn(min = 240.dp))
                state.messages.isEmpty() -> Box(Modifier.fillMaxWidth().heightIn(min = 200.dp)) {
                    EmptyView(
                        text = stringResource(R.string.chat_empty),
                        icon = Icons.Outlined.ChatBubbleOutline,
                    )
                }
                else -> LazyColumn(
                    state = listState,
                    modifier = Modifier.fillMaxWidth().weight(1f, fill = false).heightIn(min = 120.dp, max = 420.dp),
                    contentPadding = PaddingValues(vertical = Spacing.xs),
                    verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    items(state.messages, key = { it.id }) { msg ->
                        ChatBubble(
                            msg = msg,
                            allowedReactions = state.reactionsAllowed,
                            canReact = state.open && state.canPost,
                            pickerOpen = reactingMessageId == msg.id,
                            onTogglePicker = {
                                if (state.open && state.canPost && msg.userId != null) {
                                    reactingMessageId = if (reactingMessageId == msg.id) null else msg.id
                                }
                            },
                            onReact = { emoji ->
                                if (state.open && state.canPost) {
                                    UiHaptics.tick(view)
                                    reactingMessageId = null
                                    vm.react(msg.id, emoji)
                                }
                            },
                        )
                    }
                }
            }

            state.error?.let { err ->
                Text(err, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            }

            if (!state.open) {
                Text(
                    stringResource(R.string.chat_closed),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(vertical = Spacing.xxs),
                )
            } else if (state.canPost) {
                val presets = listOf(
                    stringResource(R.string.chat_preset_at_gate),
                    stringResource(R.string.chat_preset_5_mins),
                    stringResource(R.string.chat_preset_thanks),
                )
                LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xxs)) {
                    items(presets) { text ->
                        AssistChip(
                            onClick = {
                                UiHaptics.tick(view)
                                vm.send(text)
                            },
                            label = { Text(text) },
                        )
                    }
                }
                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    OutlinedTextField(
                        value = draft,
                        onValueChange = { if (it.length <= 1000) draft = it },
                        placeholder = { Text(stringResource(R.string.chat_placeholder)) },
                        maxLines = 3,
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                        keyboardActions = KeyboardActions(
                            onSend = {
                                if (draft.isNotBlank()) {
                                    UiHaptics.confirm(view)
                                    vm.send(draft) { draft = "" }
                                }
                            },
                        ),
                        modifier = Modifier.weight(1f),
                    )
                    FilledIconButton(
                        enabled = draft.isNotBlank() && !state.sending,
                        onClick = {
                            UiHaptics.confirm(view)
                            vm.send(draft) { draft = "" }
                        },
                        modifier = Modifier.size(48.dp),
                    ) {
                        if (state.sending) {
                            InlineButtonSpinner(color = MaterialTheme.colorScheme.onPrimary)
                        } else {
                            Icon(Icons.AutoMirrored.Filled.Send, contentDescription = stringResource(R.string.chat_send))
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ChatBubble(
    msg: ChatMessageDto,
    allowedReactions: List<String>,
    canReact: Boolean,
    pickerOpen: Boolean,
    onTogglePicker: () -> Unit,
    onReact: (String) -> Unit,
) {
    val isSystem = msg.userId == null
    if (isSystem) {
        Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant,
                shape = CircleShape,
            ) {
                Text(
                    msg.body,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = Spacing.sm, vertical = 4.dp),
                )
            }
        }
        return
    }

    val align = if (msg.mine) Alignment.End else Alignment.Start
    Column(Modifier.fillMaxWidth(), horizontalAlignment = align) {
        if (!msg.mine && !msg.name.isNullOrBlank()) {
            Text(
                msg.name,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(bottom = 2.dp),
            )
        }
        Surface(
            color = if (msg.mine) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant,
            contentColor = if (msg.mine) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
            shape = RoundedCornerShape(14.dp),
            modifier = Modifier
                .widthIn(max = 300.dp)
                .clip(RoundedCornerShape(14.dp))
                .clickable(onClick = onTogglePicker),
        ) {
            Column(Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xs)) {
                Text(msg.body, style = MaterialTheme.typography.bodyMedium)
                Row(
                    Modifier.padding(top = 4.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        KlTime.time(msg.createdAt),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (msg.mine && msg.status != null) {
                        val statusLabel = when (msg.status) {
                            "read" -> stringResource(R.string.chat_status_read)
                            "delivered" -> stringResource(R.string.chat_status_delivered)
                            else -> stringResource(R.string.chat_status_sent)
                        }
                        Text(
                            statusLabel,
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Medium,
                            color = if (msg.status == "read") MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
        if (msg.reactions.isNotEmpty()) {
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                modifier = Modifier.padding(top = 4.dp),
            ) {
                msg.reactions.forEach { reaction ->
                    Surface(
                        enabled = canReact,
                        onClick = { onReact(reaction.emoji) },
                        color = if (reaction.mine) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surface,
                        shape = CircleShape,
                        border = BorderStroke(
                            1.dp,
                            if (reaction.mine) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
                        ),
                    ) {
                        Text(
                            "${reaction.emoji} ${reaction.count}",
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                        )
                    }
                }
            }
        }
        if (pickerOpen) {
            Row(
                Modifier
                    .padding(top = 4.dp)
                    .background(MaterialTheme.colorScheme.surfaceContainerHigh, CircleShape)
                    .padding(horizontal = Spacing.xs, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                allowedReactions.forEach { emoji ->
                    Text(
                        emoji,
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier
                            .clip(CircleShape)
                            .clickable { onReact(emoji) }
                            .padding(horizontal = 4.dp, vertical = 2.dp),
                    )
                }
            }
        }
    }
}
