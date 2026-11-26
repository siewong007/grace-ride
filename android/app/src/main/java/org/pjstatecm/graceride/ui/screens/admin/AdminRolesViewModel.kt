package org.pjstatecm.graceride.ui.screens.admin

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AdminRoleDefDto
import org.pjstatecm.graceride.data.api.RoleCatalogEntryDto
import org.pjstatecm.graceride.di.AppContainer

data class AdminRolesUiState(
    val loading: Boolean = true,
    val busy: Boolean = false,
    val error: String? = null,
    val message: String? = null,
    val roles: List<AdminRoleDefDto> = emptyList(),
    val catalog: List<RoleCatalogEntryDto> = emptyList(),
    val selectedKey: String = NEW_ROLE_KEY,
    val draftName: String = "",
    val draftDescription: String = "",
    val draftPermissions: Set<String> = emptySet(),
    val dirty: Boolean = false,
) {
    val isNew: Boolean get() = selectedKey == NEW_ROLE_KEY
    val selectedRole: AdminRoleDefDto? get() = if (isNew) null else roles.firstOrNull { it.key == selectedKey }

    companion object {
        const val NEW_ROLE_KEY = "__new__"
    }
}

class AdminRolesViewModel(
    private val container: AppContainer,
) : ViewModel() {

    private val _state = MutableStateFlow(AdminRolesUiState())
    val state: StateFlow<AdminRolesUiState> = _state.asStateFlow()

    init {
        load()
    }

    fun load(selectKey: String? = null) {
        viewModelScope.launch {
            _state.update { it.copy(loading = it.roles.isEmpty(), error = null) }
            container.admin.roles()
                .onSuccess { resp ->
                    val chosenKey = selectKey
                        ?: _state.value.selectedKey.takeIf { key ->
                            key != AdminRolesUiState.NEW_ROLE_KEY && resp.roles.any { it.key == key }
                        }
                        ?: resp.roles.firstOrNull()?.key
                        ?: AdminRolesUiState.NEW_ROLE_KEY
                    applySelection(resp.roles, resp.catalog, chosenKey)
                }
                .onFailure { err ->
                    _state.update { it.copy(loading = false, error = err.message) }
                }
        }
    }

    fun selectRole(key: String) {
        val cur = _state.value
        if (key == AdminRolesUiState.NEW_ROLE_KEY) {
            _state.update {
                it.copy(
                    selectedKey = AdminRolesUiState.NEW_ROLE_KEY,
                    draftName = "",
                    draftDescription = "",
                    draftPermissions = emptySet(),
                    dirty = false,
                    error = null,
                )
            }
            return
        }
        applySelection(cur.roles, cur.catalog, key)
    }

    fun startNewRole() {
        selectRole(AdminRolesUiState.NEW_ROLE_KEY)
    }

    fun setDraftName(name: String) {
        _state.update { cur ->
            val next = cur.copy(draftName = name.take(40))
            next.copy(dirty = computeDirty(next))
        }
    }

    fun setDraftDescription(desc: String) {
        _state.update { cur ->
            val next = cur.copy(draftDescription = desc.take(200))
            next.copy(dirty = computeDirty(next))
        }
    }

    fun togglePermission(permKey: String) {
        val role = _state.value.selectedRole
        if (role != null && permKey in role.locked) return
        _state.update { cur ->
            val nextPerms = cur.draftPermissions.toMutableSet()
            if (!nextPerms.add(permKey)) nextPerms.remove(permKey)
            val next = cur.copy(draftPermissions = nextPerms)
            next.copy(dirty = computeDirty(next))
        }
    }

    fun togglePageRow(entry: RoleCatalogEntryDto) {
        val locked = _state.value.selectedRole?.locked.orEmpty().toSet()
        val keys = entry.actions.map { "${entry.page}.$it" }.filterNot { it in locked }
        if (keys.isEmpty()) return
        _state.update { cur ->
            val allGranted = keys.all { it in cur.draftPermissions }
            val nextPerms = cur.draftPermissions.toMutableSet()
            if (allGranted) nextPerms.removeAll(keys.toSet()) else nextPerms.addAll(keys)
            val next = cur.copy(draftPermissions = nextPerms)
            next.copy(dirty = computeDirty(next))
        }
    }

    fun saveCurrentRole(createdMsg: String, savedMsg: String) {
        val cur = _state.value
        if (cur.busy) return
        val name = cur.draftName.trim()
        val desc = cur.draftDescription.trim()
        val perms = cur.draftPermissions.sorted()
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            if (cur.isNew) {
                container.admin.createRole(name = name, description = desc, permissions = perms)
                    .onSuccess { newKey ->
                        runCatching { container.auth.refresh() }
                        _state.update { it.copy(busy = false, message = createdMsg) }
                        load(selectKey = newKey)
                    }
                    .onFailure { err ->
                        _state.update { it.copy(busy = false, error = err.message) }
                    }
            } else {
                val role = cur.selectedRole ?: return@launch
                container.admin.updateRole(
                    key = role.key,
                    name = if (role.system) role.name else name,
                    description = if (role.system) role.description else desc,
                    permissions = perms,
                )
                    .onSuccess {
                        runCatching { container.auth.refresh() }
                        _state.update { it.copy(busy = false, message = savedMsg) }
                        load(selectKey = role.key)
                    }
                    .onFailure { err ->
                        _state.update { it.copy(busy = false, error = err.message) }
                    }
            }
        }
    }

    fun deleteCurrentRole(deletedMsg: String) {
        val cur = _state.value
        val role = cur.selectedRole ?: return
        if (cur.busy || role.system || cur.isNew) return
        _state.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            container.admin.deleteRole(role.key)
                .onSuccess {
                    runCatching { container.auth.refresh() }
                    _state.update {
                        it.copy(
                            busy = false,
                            selectedKey = AdminRolesUiState.NEW_ROLE_KEY,
                            message = deletedMsg,
                        )
                    }
                    load(selectKey = null)
                }
                .onFailure { err ->
                    _state.update { it.copy(busy = false, error = err.message) }
                }
        }
    }

    fun clearMessage() {
        _state.update { it.copy(message = null) }
    }

    private fun applySelection(
        roles: List<AdminRoleDefDto>,
        catalog: List<RoleCatalogEntryDto>,
        key: String,
    ) {
        val role = roles.firstOrNull { it.key == key }
        _state.update {
            it.copy(
                loading = false,
                error = null,
                roles = roles,
                catalog = catalog,
                selectedKey = role?.key ?: AdminRolesUiState.NEW_ROLE_KEY,
                draftName = role?.name.orEmpty(),
                draftDescription = role?.description.orEmpty(),
                draftPermissions = role?.permissions?.toSet() ?: emptySet(),
                dirty = false,
            )
        }
    }

    private fun computeDirty(cur: AdminRolesUiState): Boolean {
        val role = cur.selectedRole ?: return cur.draftName.isNotBlank() ||
            cur.draftDescription.isNotBlank() ||
            cur.draftPermissions.isNotEmpty()
        return cur.draftName != role.name ||
            cur.draftDescription != role.description ||
            cur.draftPermissions != role.permissions.toSet()
    }

    companion object {
        fun factory(container: AppContainer): ViewModelProvider.Factory = viewModelFactory {
            initializer { AdminRolesViewModel(container) }
        }
    }
}
