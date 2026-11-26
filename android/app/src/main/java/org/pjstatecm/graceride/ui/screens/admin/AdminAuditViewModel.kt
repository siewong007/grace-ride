package org.pjstatecm.graceride.ui.screens.admin

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AuditItemDto
import org.pjstatecm.graceride.di.AppContainer

data class AdminAuditUiState(
    val loading: Boolean = true,
    val loadingMore: Boolean = false,
    val exportingStream: Boolean = false,
    val error: String? = null,
    val items: List<AuditItemDto> = emptyList(),
    val nextBefore: Long? = null,
    val retentionDays: Int? = null,
    val q: String = "",
    val entity: String = "",
    val entityId: String = "",
    val action: String = "",
    val actorId: String = "",
    val fromDate: String = "",
    val toDate: String = "",
) {
    val securityCount: Int
        get() = items.count { it.action in SECURITY_ACTIONS }

    val escalationCount: Int
        get() = items.count { it.action == "ride.escalate" }

    val hasFilters: Boolean
        get() = q.isNotBlank() || entity.isNotBlank() || entityId.isNotBlank() ||
            action.isNotBlank() || actorId.isNotBlank() || fromDate.isNotBlank() || toDate.isNotBlank()

    companion object {
        val SECURITY_ACTIONS = setOf("login.failed", "totp.failed", "password.failed", "recovery.reject")
        val WARNING_ACTIONS = setOf(
            "ride.escalate", "ride.no_show", "ride.return", "ride.cancel",
            "group.delete", "role.delete", "invite.revoke",
        )
        val INFO_ACTIONS = setOf(
            "role.create", "role.update", "recovery.approve",
            "recovery.await_second", "reimbursement.pay", "settlement.publish",
        )
        val ENTITIES = listOf(
            "user", "ride", "event", "settlement", "settlement_share", "reimbursement",
            "help_ticket", "file", "group", "area", "role", "invite", "announcement",
            "recovery", "guide_workflow", "visitor_pass", "area_fares", "system_settings",
            "payout_batch", "app_secrets", "family", "finance_report", "users",
        )
        val ACTION_GROUPS = listOf(
            "security" to listOf(
                "login.passkey", "login.reauth", "login.failed", "password.failed",
                "logout", "logout.all", "passkey.register", "passkey.delete",
                "totp.setup", "totp.enable", "totp.failed", "totp.verify",
                "totp.backup_code_used", "totp.backup_codes_regenerated",
                "password.set", "recovery.request", "recovery.use", "recovery.approve",
                "recovery.reject", "recovery.await_second", "invite.accept", "terms.accept",
            ),
            "rides" to listOf(
                "ride.create", "ride.claim", "ride.driver_confirm", "ride.driver_withdraw",
                "ride.passenger_accept", "ride.passenger_decline", "ride.ready",
                "ride.approve", "ride.reassign", "ride.plan_assign", "ride.alt_transport",
                "ride.return", "ride.delayed", "ride.driver_delay", "ride.arrived_gate",
                "ride.unable_to_join", "ride.start", "ride.pickup", "ride.no_show",
                "ride.complete", "ride.cancel", "ride.escalate",
                "location.share", "location.stop", "driver.profile_update",
            ),
            "events" to listOf(
                "event.create", "event.update", "event.cancel", "event.finish",
                "event.signup", "event.withdraw", "event.attendance", "event.waitlist_promote",
            ),
            "finance" to listOf(
                "reimbursement.create", "reimbursement.resubmit", "reimbursement.review",
                "reimbursement.approve", "reimbursement.reject", "reimbursement.pay",
                "settlement.create", "settlement.edit_draft", "settlement.publish",
                "settlement.close", "settlement.delete", "settlement.revise",
                "share.proof", "share.confirm", "share.needs_info",
            ),
            "admin" to listOf(
                "user.update", "role.create", "role.update", "role.delete", "role.bootstrap",
                "group.create", "group.update", "group.delete", "group.members",
                "area.create", "area.update", "settings.rates", "settings.system",
                "settings.alerts", "announcement.create", "announcement.update", "announcement.delete",
                "invite.create", "invite.revoke",
            ),
        )
    }
}

class AdminAuditViewModel(
    private val container: AppContainer,
) : ViewModel() {

    private val _state = MutableStateFlow(AdminAuditUiState())
    val state: StateFlow<AdminAuditUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private var debounceJob: Job? = null

    init {
        load(before = null)
    }

    fun setQuery(q: String) {
        _state.update { it.copy(q = q) }
        debounceJob?.cancel()
        debounceJob = viewModelScope.launch {
            delay(300L)
            load(before = null)
        }
    }

    fun setFilter(
        entity: String = _state.value.entity,
        entityId: String = _state.value.entityId,
        action: String = _state.value.action,
        actorId: String = _state.value.actorId,
        fromDate: String = _state.value.fromDate,
        toDate: String = _state.value.toDate,
    ) {
        _state.update {
            it.copy(
                entity = entity,
                entityId = entityId,
                action = action,
                actorId = actorId,
                fromDate = fromDate,
                toDate = toDate,
            )
        }
        load(before = null)
    }

    fun filterByActor(actorName: String) {
        _state.update { it.copy(q = actorName) }
        load(before = null)
    }

    fun toggleSecurityOnly() {
        val isSec = _state.value.q == "failed"
        _state.update { it.copy(q = if (isSec) "" else "failed") }
        load(before = null)
    }

    fun clearFilters() {
        debounceJob?.cancel()
        _state.update {
            it.copy(q = "", entity = "", entityId = "", action = "", actorId = "", fromDate = "", toDate = "")
        }
        load(before = null)
    }

    fun loadMore() {
        val next = _state.value.nextBefore ?: return
        if (_state.value.loading || _state.value.loadingMore) return
        load(before = next)
    }

    fun refresh() {
        load(before = null)
    }

    fun downloadStreamCsv(onResult: (Result<ByteArray>) -> Unit) {
        if (_state.value.exportingStream) return
        val cur = _state.value
        _state.update { it.copy(exportingStream = true) }
        viewModelScope.launch {
            val res = container.admin.exportAuditCsvBytes(
                q = cur.q.trim().ifBlank { null },
                entity = cur.entity.ifBlank { null },
                entityId = cur.entityId.toLongOrNull(),
                action = cur.action.ifBlank { null },
                actorId = cur.actorId.toLongOrNull(),
                from = cur.fromDate.ifBlank { null },
                to = cur.toDate.ifBlank { null },
            )
            _state.update { it.copy(exportingStream = false) }
            onResult(res)
        }
    }

    private fun load(before: Long?) {
        loadJob?.cancel()
        val cur = _state.value
        loadJob = viewModelScope.launch {
            _state.update {
                if (before == null) it.copy(loading = true, error = null)
                else it.copy(loadingMore = true, error = null)
            }
            container.admin.auditLog(
                q = cur.q.trim().ifBlank { null },
                entity = cur.entity.ifBlank { null },
                entityId = cur.entityId.toLongOrNull(),
                action = cur.action.ifBlank { null },
                actorId = cur.actorId.toLongOrNull(),
                from = cur.fromDate.ifBlank { null },
                to = cur.toDate.ifBlank { null },
                before = before,
                limit = 50,
            ).onSuccess { page ->
                _state.update { state ->
                    state.copy(
                        loading = false,
                        loadingMore = false,
                        error = null,
                        items = if (before == null) page.items else (state.items + page.items).distinctBy { it.id },
                        nextBefore = page.nextBefore,
                        retentionDays = page.retentionDays ?: state.retentionDays,
                    )
                }
            }.onFailure { err ->
                _state.update {
                    it.copy(loading = false, loadingMore = false, error = err.message)
                }
            }
        }
    }

    companion object {
        fun factory(container: AppContainer): ViewModelProvider.Factory = viewModelFactory {
            initializer { AdminAuditViewModel(container) }
        }
    }
}
