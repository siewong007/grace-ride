package org.pjstatecm.graceride.ui.screens.admin

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.AlertSettingsDto
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.SystemSettingsDto
import org.pjstatecm.graceride.data.api.TestWebhookRespDto
import org.pjstatecm.graceride.data.api.UpdateAlertSettingsReq
import org.pjstatecm.graceride.data.api.UpdateSystemSettingsReq
import org.pjstatecm.graceride.di.AppContainer

data class AdminSettingsUiState(
    val tab: String = "system",
    val loading: Boolean = true,
    val busy: Boolean = false,
    val error: String? = null,
    val message: String? = null,

    // System tab
    val systemSettings: SystemSettingsDto? = null,
    val auditDaysText: String = "",
    val chatDaysText: String = "",
    val maxDriversText: String = "",
    val sessionTimeoutText: String = "",

    // Alerts tab
    val alertSettings: AlertSettingsDto? = null,
    val tgTokenInput: String = "",
    val tgUsernameInput: String = "",
    val clearTgToken: Boolean = false,
    val webhookUrlInput: String = "",
    val clearWebhookUrl: Boolean = false,
    val webhookSecretInput: String = "",
    val clearWebhookSecret: Boolean = false,
    val mobileWebhookUrlInput: String = "",
    val clearMobileWebhookUrl: Boolean = false,
    val vapidPubInput: String = "",
    val vapidPrivInput: String = "",
    val rotateVapidKeys: Boolean = false,
    val testingWebhook: Boolean = false,
    val lastWebhookTest: TestWebhookRespDto? = null,

    // Groups tab
    val groups: List<GroupDto> = emptyList(),
    val activeUsers: List<AdminUserDto> = emptyList(),
    val editingGroupMembersIds: Set<Long> = emptySet(),
    val groupMembersTarget: GroupDto? = null,
    val groupMembersLoading: Boolean = false,
)

class AdminSettingsViewModel(
    private val container: AppContainer,
    initialTab: String? = null,
) : ViewModel() {

    private val _state = MutableStateFlow(
        AdminSettingsUiState(tab = resolveInitialTab(initialTab, container.auth.me)),
    )
    val state: StateFlow<AdminSettingsUiState> = _state.asStateFlow()

    init {
        refresh()
    }

    fun setTab(tab: String) {
        _state.update { it.copy(tab = tab, error = null) }
    }

    fun refresh() {
        val me = container.auth.me
        viewModelScope.launch {
            _state.update { it.copy(loading = true, error = null) }
            coroutineScope {
                val sysDef = if (me?.can("admin_settings") == true) async { container.admin.systemSettings() } else null
                val alertDef = if (me?.can("admin_settings") == true) async { container.admin.alertSettings() } else null
                val grpDef = if (me?.can("admin_groups") == true) async { container.admin.groups() } else null

                val sys = sysDef?.await()?.getOrNull()
                val alerts = alertDef?.await()?.getOrNull()
                val groups = grpDef?.await()?.getOrNull()

                _state.update { cur ->
                    cur.copy(
                        loading = false,
                        systemSettings = sys ?: cur.systemSettings,
                        auditDaysText = sys?.auditRetentionDays?.toString() ?: cur.auditDaysText,
                        chatDaysText = sys?.chatRetentionDays?.toString() ?: cur.chatDaysText,
                        maxDriversText = sys?.maxLiveDrivers?.toString() ?: cur.maxDriversText,
                        sessionTimeoutText = sys?.sessionTimeoutMins?.toString() ?: cur.sessionTimeoutText,
                        alertSettings = alerts ?: cur.alertSettings,
                        tgUsernameInput = alerts?.telegramBotUsername ?: cur.tgUsernameInput,
                        vapidPubInput = alerts?.vapidPublicKey ?: cur.vapidPubInput,
                        groups = groups ?: cur.groups,
                    )
                }
            }
        }
    }

    fun setAuditDays(v: String) = _state.update { it.copy(auditDaysText = v) }
    fun setChatDays(v: String) = _state.update { it.copy(chatDaysText = v) }
    fun setMaxDrivers(v: String) = _state.update { it.copy(maxDriversText = v) }
    fun setSessionTimeout(v: String) = _state.update { it.copy(sessionTimeoutText = v) }

    fun saveSystemSettings(savedMsg: String) {
        val cur = _state.value
        val audit = cur.auditDaysText.trim().toIntOrNull() ?: return
        val chat = cur.chatDaysText.trim().toIntOrNull() ?: return
        val drivers = cur.maxDriversText.trim().toIntOrNull() ?: return
        val timeout = cur.sessionTimeoutText.trim().toIntOrNull() ?: return
        if (audit !in 30..3650 || chat !in 7..3650 || drivers !in 1..100 || timeout !in 0..1440) return
        if (cur.busy) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            container.admin.updateSystemSettings(
                UpdateSystemSettingsReq(
                    auditRetentionDays = audit,
                    chatRetentionDays = chat,
                    maxLiveDrivers = drivers,
                    sessionTimeoutMins = timeout,
                ),
            ).onSuccess {
                _state.update { it.copy(busy = false, message = savedMsg) }
                refresh()
            }.onFailure { err ->
                _state.update { it.copy(busy = false, error = err.message) }
            }
        }
    }

    fun setTgToken(v: String) = _state.update { it.copy(tgTokenInput = v, clearTgToken = false) }
    fun setClearTgToken(v: Boolean) = _state.update { it.copy(clearTgToken = v, tgTokenInput = if (v) "" else it.tgTokenInput) }
    fun setTgUsername(v: String) = _state.update { it.copy(tgUsernameInput = v) }
    fun setWebhookUrl(v: String) = _state.update { it.copy(webhookUrlInput = v, clearWebhookUrl = false) }
    fun setClearWebhookUrl(v: Boolean) = _state.update { it.copy(clearWebhookUrl = v, webhookUrlInput = if (v) "" else it.webhookUrlInput) }
    fun setWebhookSecret(v: String) = _state.update { it.copy(webhookSecretInput = v, clearWebhookSecret = false) }
    fun setClearWebhookSecret(v: Boolean) = _state.update { it.copy(clearWebhookSecret = v, webhookSecretInput = if (v) "" else it.webhookSecretInput) }
    fun setMobileWebhookUrl(v: String) = _state.update { it.copy(mobileWebhookUrlInput = v, clearMobileWebhookUrl = false) }
    fun setClearMobileWebhookUrl(v: Boolean) = _state.update { it.copy(clearMobileWebhookUrl = v, mobileWebhookUrlInput = if (v) "" else it.mobileWebhookUrlInput) }
    fun setVapidPub(v: String) = _state.update { it.copy(vapidPubInput = v) }
    fun setVapidPriv(v: String) = _state.update { it.copy(vapidPrivInput = v) }
    fun setRotateVapidKeys(v: Boolean) = _state.update { it.copy(rotateVapidKeys = v) }

    fun saveAlertSettings(savedMsg: String) {
        val cur = _state.value
        if (cur.busy) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            val req = UpdateAlertSettingsReq(
                telegramBotToken = cur.tgTokenInput.trim().takeIf { it.isNotEmpty() },
                clearTelegramBotToken = cur.clearTgToken.takeIf { it },
                telegramBotUsername = cur.tgUsernameInput.trim().takeIf { it != (cur.alertSettings?.telegramBotUsername ?: "") },
                alertWebhookUrl = cur.webhookUrlInput.trim().takeIf { it.isNotEmpty() },
                clearAlertWebhookUrl = cur.clearWebhookUrl.takeIf { it },
                alertWebhookSecret = cur.webhookSecretInput.trim().takeIf { it.isNotEmpty() },
                clearAlertWebhookSecret = cur.clearWebhookSecret.takeIf { it },
                mobilePushWebhookUrl = cur.mobileWebhookUrlInput.trim().takeIf { it.isNotEmpty() },
                clearMobilePushWebhookUrl = cur.clearMobileWebhookUrl.takeIf { it },
                vapidPublicKey = cur.vapidPubInput.trim().takeIf { it.isNotEmpty() && !cur.rotateVapidKeys },
                vapidPrivateKey = cur.vapidPrivInput.trim().takeIf { it.isNotEmpty() && !cur.rotateVapidKeys },
                rotateVapidKeys = cur.rotateVapidKeys.takeIf { it },
            )
            container.admin.updateAlertSettings(req)
                .onSuccess {
                    _state.update {
                        it.copy(
                            busy = false,
                            tgTokenInput = "",
                            clearTgToken = false,
                            webhookUrlInput = "",
                            clearWebhookUrl = false,
                            webhookSecretInput = "",
                            clearWebhookSecret = false,
                            mobileWebhookUrlInput = "",
                            clearMobileWebhookUrl = false,
                            vapidPrivInput = "",
                            rotateVapidKeys = false,
                            message = savedMsg,
                        )
                    }
                    refresh()
                }
                .onFailure { err ->
                    _state.update { it.copy(busy = false, error = err.message) }
                }
        }
    }

    fun testWebhook() {
        if (_state.value.testingWebhook) return
        _state.update { it.copy(testingWebhook = true, lastWebhookTest = null, error = null) }
        viewModelScope.launch {
            container.admin.testAlertWebhook()
                .onSuccess { resp -> _state.update { it.copy(testingWebhook = false, lastWebhookTest = resp) } }
                .onFailure { err -> _state.update { it.copy(testingWebhook = false, error = err.message) } }
        }
    }

    fun createGroup(name: String, description: String, parentId: Long?, addedMsg: String) {
        if (_state.value.busy || name.isBlank()) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            container.admin.createGroup(name.trim(), description.trim(), parentId)
                .onSuccess {
                    _state.update { it.copy(busy = false, message = addedMsg) }
                    refresh()
                }
                .onFailure { err -> _state.update { it.copy(busy = false, error = err.message) } }
        }
    }

    fun updateGroup(id: Long, name: String, description: String, parentId: Long?, savedMsg: String) {
        if (_state.value.busy || name.isBlank()) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            container.admin.updateGroup(id, name.trim(), description.trim(), parentId)
                .onSuccess {
                    _state.update { it.copy(busy = false, message = savedMsg) }
                    refresh()
                }
                .onFailure { err -> _state.update { it.copy(busy = false, error = err.message) } }
        }
    }

    fun deleteGroup(id: Long, deletedMsg: String) {
        if (_state.value.busy) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            container.admin.deleteGroup(id)
                .onSuccess {
                    _state.update { it.copy(busy = false, message = deletedMsg) }
                    refresh()
                }
                .onFailure { err -> _state.update { it.copy(busy = false, error = err.message) } }
        }
    }

    fun openGroupMembers(group: GroupDto) {
        _state.update { it.copy(groupMembersTarget = group, groupMembersLoading = true, error = null) }
        viewModelScope.launch {
            coroutineScope {
                val membersDef = async { container.admin.groupMembers(group.id) }
                val usersDef = if (_state.value.activeUsers.isEmpty()) {
                    async { container.admin.users(status = "active", limit = 200) }
                } else null
                val members = membersDef.await().getOrNull() ?: emptyList()
                val users = usersDef?.await()?.getOrNull()?.items ?: _state.value.activeUsers
                _state.update {
                    it.copy(
                        groupMembersLoading = false,
                        editingGroupMembersIds = members.map { m -> m.id }.toSet(),
                        activeUsers = users,
                    )
                }
            }
        }
    }

    fun toggleGroupMember(userId: Long) {
        _state.update { cur ->
            val next = cur.editingGroupMembersIds.toMutableSet()
            if (!next.add(userId)) next.remove(userId)
            cur.copy(editingGroupMembersIds = next)
        }
    }

    fun setAllGroupMembers(ids: Set<Long>) {
        _state.update { cur -> cur.copy(editingGroupMembersIds = ids) }
    }

    fun closeGroupMembers() {
        _state.update { it.copy(groupMembersTarget = null, groupMembersLoading = false) }
    }

    fun saveGroupMembers(savedMsg: String) {
        val target = _state.value.groupMembersTarget ?: return
        if (_state.value.busy) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            container.admin.setGroupMembers(target.id, _state.value.editingGroupMembersIds.toList())
                .onSuccess {
                    _state.update { it.copy(busy = false, groupMembersTarget = null, message = savedMsg) }
                    refresh()
                }
                .onFailure { err -> _state.update { it.copy(busy = false, error = err.message) } }
        }
    }

    fun clearMessage() {
        _state.update { it.copy(message = null, error = null) }
    }

    companion object {
        private fun resolveInitialTab(requested: String?, me: org.pjstatecm.graceride.data.api.Me?): String =
            when {
                requested == "alerts" && me?.can("admin_settings") == true -> "alerts"
                requested == "groups" && me?.can("admin_groups") == true -> "groups"
                me?.can("admin_settings") == true -> "system"
                me?.can("admin_groups") == true -> "groups"
                else -> "system"
            }

        fun factory(container: AppContainer, initialTab: String?): ViewModelProvider.Factory = viewModelFactory {
            initializer { AdminSettingsViewModel(container, initialTab) }
        }
    }
}
