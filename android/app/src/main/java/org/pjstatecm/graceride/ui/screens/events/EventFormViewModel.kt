package org.pjstatecm.graceride.ui.screens.events

import android.content.Context
import android.net.Uri
import androidx.annotation.StringRes
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.EventUpsertReq
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer

data class EventFormUiState(
    val loading: Boolean = true,
    val saving: Boolean = false,
    val loadError: String? = null,
    val error: String? = null,
    @param:StringRes @get:StringRes val errorRes: Int? = null,
    val isEdit: Boolean = false,
    val isDuplicate: Boolean = false,
    val isEditingDraft: Boolean = false,
    val groups: List<GroupDto> = emptyList(),
    val areas: List<Area> = emptyList(),
    val title: String = "",
    val kind: String = "badminton",
    val icon: String = "",
    val date: String = "",
    val time: String = "09:30",
    val location: String = "",
    val capacity: String = "12",
    val costNote: String = "",
    val deadline: String = "",
    val description: String = "",
    val areaId: Long? = null,
    val photoFileId: Long? = null,
    val attendanceEnabled: Boolean = true,
    val mileageClaimable: Boolean = false,
    val repeatFrequency: String = "none",
    val repeatCount: String = "4",
    val groupIds: Set<Long> = emptySet(),
    val narrowJoinAs: Boolean = false,
    val asGroupIds: Set<Long> = emptySet(),
    val savedEventId: Long? = null,
)

class EventFormViewModel(
    private val container: AppContainer,
    private val eventId: Long?,
    private val duplicateId: Long?,
) : ViewModel() {
    private val _state = MutableStateFlow(
        EventFormUiState(
            isEdit = eventId != null,
            isDuplicate = eventId == null && duplicateId != null,
        ),
    )
    val state: StateFlow<EventFormUiState> = _state.asStateFlow()

    init {
        load()
    }

    fun load() {
        _state.value = _state.value.copy(loading = true, loadError = null)
        viewModelScope.launch {
            try {
                val sourceId = eventId ?: duplicateId
                val canMileage = container.auth.me?.canAuthoriseMileage() == true
                val (groups, areas, existing) = coroutineScope {
                    val gDef = async { runCatching { container.community.groups() }.getOrDefault(emptyList()) }
                    val aDef = async { runCatching { container.rides.areas() }.getOrDefault(emptyList()) }
                    val eDef = async { sourceId?.let { container.events.detail(it) } }
                    Triple(gDef.await(), aDef.await(), eDef.await())
                }
                val isEditMode = eventId != null
                val myId = container.auth.me?.id
                _state.value = if (existing != null) {
                    val parts = existing.startsAt.split('T', ' ')
                    val keepPhoto = isEditMode || (myId != null && existing.createdBy?.id == myId)
                    EventFormUiState(
                        loading = false,
                        isEdit = isEditMode,
                        isDuplicate = !isEditMode,
                        isEditingDraft = isEditMode && existing.status == "draft",
                        groups = groups,
                        areas = areas,
                        title = existing.title,
                        kind = existing.kind.ifBlank { "badminton" },
                        icon = existing.icon.orEmpty(),
                        date = if (isEditMode) parts.getOrNull(0).orEmpty() else "",
                        time = parts.getOrNull(1).orEmpty().take(5).ifBlank { "09:30" },
                        location = existing.location,
                        capacity = existing.capacity.toString(),
                        costNote = existing.costNote,
                        deadline = if (isEditMode) existing.signupDeadline.orEmpty() else "",
                        description = existing.description,
                        areaId = existing.area?.id,
                        photoFileId = if (keepPhoto) existing.photoFileId else null,
                        attendanceEnabled = existing.attendanceEnabled,
                        mileageClaimable = if (isEditMode || canMileage) existing.mileageClaimable else false,
                        groupIds = existing.groupIds.toSet(),
                        narrowJoinAs = existing.asGroupIds.isNotEmpty(),
                        asGroupIds = existing.asGroupIds.toSet(),
                    )
                } else {
                    EventFormUiState(
                        loading = false,
                        isEdit = false,
                        isDuplicate = false,
                        groups = groups,
                        areas = areas,
                    )
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, loadError = e.message)
            } catch (e: Exception) {
                _state.value = _state.value.copy(loading = false, loadError = e.message)
            }
        }
    }

    fun update(transform: (EventFormUiState) -> EventFormUiState) {
        _state.value = transform(_state.value).copy(error = null, errorRes = null)
    }

    fun toggleGroup(id: Long) {
        val cur = _state.value.groupIds
        _state.value = _state.value.copy(
            groupIds = if (id in cur) cur - id else cur + id,
            error = null,
            errorRes = null,
        )
    }

    fun toggleAsGroup(id: Long) {
        val cur = _state.value.asGroupIds
        _state.value = _state.value.copy(
            asGroupIds = if (id in cur) cur - id else cur + id,
        )
    }

    fun uploadCoverPhoto(context: Context, uri: Uri) {
        if (_state.value.saving) return
        _state.value = _state.value.copy(saving = true, error = null, errorRes = null)
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(
                        context = context,
                        uri = uri,
                        prefix = "event-cover",
                        maxDimensionPx = 1600,
                        quality = 85,
                    )
                }
                val fileRef = container.community.upload(
                    bytes = prepared.bytes,
                    filename = prepared.filename,
                    mime = prepared.mime,
                )
                _state.value = _state.value.copy(saving = false, photoFileId = fileRef.id)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(saving = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(saving = false, error = e.message)
            }
        }
    }

    fun submit(
        targetStatus: String?,
        publishAfterUpdate: Boolean = false,
        joinableGroupIds: Set<Long>,
    ) {
        val s = _state.value
        if (s.saving) return
        val cleanTitle = s.title.trim()
        val cleanDate = s.date.trim()
        val cleanTime = s.time.trim()
        val dateRe = Regex("^\\d{4}-\\d{2}-\\d{2}$")
        val timeRe = Regex("^([01]\\d|2[0-3]):[0-5]\\d$")
        when {
            cleanTitle.isEmpty() -> {
                _state.value = s.copy(errorRes = R.string.event_form_name)
                return
            }
            !dateRe.matches(cleanDate) || !timeRe.matches(cleanTime) -> {
                _state.value = s.copy(errorRes = R.string.event_form_datetime_invalid)
                return
            }
            s.groupIds.isEmpty() -> {
                _state.value = s.copy(errorRes = R.string.event_form_group_required)
                return
            }
        }
        val canMileage = container.auth.me?.canAuthoriseMileage() == true
        val req = EventUpsertReq(
            title = cleanTitle,
            kind = s.kind,
            icon = s.icon.trim().takeIf { it.isNotEmpty() },
            description = s.description.trim(),
            starts_at = "${cleanDate}T$cleanTime",
            location = s.location.trim(),
            capacity = (s.capacity.trim().toLongOrNull() ?: 0L).coerceAtLeast(0L),
            cost_note = s.costNote.trim(),
            signup_deadline = s.deadline.trim().takeIf { it.isNotEmpty() }?.replace(' ', 'T'),
            group_ids = s.groupIds.toList(),
            as_group_ids = if (s.narrowJoinAs) {
                s.asGroupIds.filter { it in joinableGroupIds }
            } else {
                emptyList()
            },
            photo_file_id = s.photoFileId,
            attendance_enabled = s.attendanceEnabled,
            mileage_claimable = if (eventId == null && !canMileage) false else s.mileageClaimable,
            area_id = s.areaId,
            repeat_frequency = if (eventId == null && s.repeatFrequency != "none") s.repeatFrequency else null,
            repeat_count = if (eventId == null && s.repeatFrequency != "none") {
                (s.repeatCount.toLongOrNull() ?: 2L).coerceIn(2L, 12L)
            } else {
                null
            },
            status = if (eventId == null) targetStatus else null,
        )

        _state.value = s.copy(saving = true, error = null, errorRes = null)
        viewModelScope.launch {
            try {
                val outId = if (eventId != null) {
                    container.events.update(eventId, req)
                    if (publishAfterUpdate) {
                        container.events.publish(eventId)
                    }
                    eventId
                } else {
                    container.events.create(req).id
                }
                _state.value = _state.value.copy(saving = false, savedEventId = outId)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(saving = false, error = e.message)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.value = _state.value.copy(saving = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, eventId: Long?, duplicateId: Long?) = viewModelFactory {
            initializer { EventFormViewModel(container, eventId, duplicateId) }
        }
    }
}
