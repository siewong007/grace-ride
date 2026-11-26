package org.pjstatecm.graceride.ui.screens.help

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.GuideStepInReq
import org.pjstatecm.graceride.data.api.SaveGuideReq
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer

data class GuideStepDraft(
    val localKey: Long,
    val id: String? = null,
    val titleEn: String = "",
    val titleZh: String = "",
    val bodyEn: String = "",
    val bodyZh: String = "",
    val link: String = "",
    val imageFileId: Long? = null,
    val noteKind: String = "tip",
    val noteEn: String = "",
    val noteZh: String = "",
)

data class HelpGuideEditorUiState(
    val loading: Boolean = true,
    val saving: Boolean = false,
    val editKey: String? = null,
    val guideId: Long? = null,
    val slug: String = "",
    val titleEn: String = "",
    val titleZh: String = "",
    val summaryEn: String = "",
    val summaryZh: String = "",
    val audience: String = "member",
    val published: Boolean = true,
    val steps: List<GuideStepDraft> = listOf(GuideStepDraft(localKey = 1L)),
    val error: String? = null,
    val savedKey: String? = null,
    val deleted: Boolean = false,
) {
    val enGaps: Int
        get() = listOf(
            titleEn.isBlank(),
            summaryEn.isBlank(),
            steps.any { it.titleEn.isBlank() },
        ).count { it }
    val zhGaps: Int
        get() = listOf(
            titleZh.isBlank(),
            summaryZh.isBlank(),
            steps.any { it.titleZh.isBlank() },
        ).count { it }
    val completenessGaps: Int
        get() = listOf(
            titleEn.isBlank(),
            titleZh.isBlank(),
            summaryEn.isBlank(),
            summaryZh.isBlank(),
            steps.any { it.titleEn.isBlank() || it.titleZh.isBlank() },
        ).count { it }
}

class HelpGuideEditorViewModel(
    private val container: AppContainer,
    private val editKey: String?,
) : ViewModel() {
    private val _state = MutableStateFlow(HelpGuideEditorUiState(loading = editKey != null, editKey = editKey))
    val state: StateFlow<HelpGuideEditorUiState> = _state.asStateFlow()
    private var nextKey = 2L

    init {
        if (editKey != null) {
            load(editKey)
        }
    }

    private fun load(key: String) {
        viewModelScope.launch {
            try {
                val g = container.help.guideDetail(key)
                val drafts = g.steps.mapIndexed { idx, s ->
                    GuideStepDraft(
                        localKey = (idx + 1).toLong(),
                        id = s.id,
                        titleEn = s.titleEn,
                        titleZh = s.titleZh,
                        bodyEn = s.bodyEn,
                        bodyZh = s.bodyZh,
                        link = s.link,
                        imageFileId = s.imageFileId,
                        noteKind = s.noteKind ?: "tip",
                        noteEn = s.noteEn.orEmpty(),
                        noteZh = s.noteZh.orEmpty(),
                    )
                }.ifEmpty { listOf(GuideStepDraft(localKey = 1L)) }
                nextKey = (drafts.maxOfOrNull { it.localKey } ?: 1L) + 1L
                _state.value = HelpGuideEditorUiState(
                    loading = false,
                    editKey = g.id.toString(),
                    guideId = g.id,
                    slug = g.slug,
                    titleEn = g.titleEn,
                    titleZh = g.titleZh,
                    summaryEn = g.summaryEn,
                    summaryZh = g.summaryZh,
                    audience = g.audience,
                    published = g.published,
                    steps = drafts,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun update(block: (HelpGuideEditorUiState) -> HelpGuideEditorUiState) {
        _state.value = block(_state.value).copy(error = null)
    }

    fun addStep() {
        val k = nextKey++
        _state.value = _state.value.copy(steps = _state.value.steps + GuideStepDraft(localKey = k))
    }

    fun removeStep(localKey: Long) {
        if (_state.value.steps.size <= 1) return
        _state.value = _state.value.copy(steps = _state.value.steps.filterNot { it.localKey == localKey })
    }

    fun moveStep(localKey: Long, delta: Int) {
        val list = _state.value.steps.toMutableList()
        val idx = list.indexOfFirst { it.localKey == localKey }
        val next = idx + delta
        if (idx < 0 || next !in list.indices) return
        val item = list.removeAt(idx)
        list.add(next, item)
        _state.value = _state.value.copy(steps = list)
    }

    fun updateStep(localKey: Long, block: (GuideStepDraft) -> GuideStepDraft) {
        _state.value = _state.value.copy(
            steps = _state.value.steps.map { if (it.localKey == localKey) block(it) else it },
        )
    }

    fun uploadStepImage(context: Context, localKey: Long, uri: Uri) {
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(context, uri, prefix = "guide-step", maxDimensionPx = 1600, quality = 85)
                }
                val fileRef = container.help.uploadFile(prepared.bytes, prepared.filename, prepared.mime)
                updateStep(localKey) { it.copy(imageFileId = fileRef.id) }
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message)
            }
        }
    }

    fun save(fallbackErr: String) {
        val s = _state.value
        if (s.saving) return
        if (s.slug.trim().isEmpty() || (s.titleEn.isBlank() && s.titleZh.isBlank())) {
            _state.value = s.copy(error = fallbackErr)
            return
        }
        val req = SaveGuideReq(
            slug = s.slug.trim().lowercase(),
            title_en = s.titleEn.trim(),
            title_zh = s.titleZh.trim(),
            summary_en = s.summaryEn.trim(),
            summary_zh = s.summaryZh.trim(),
            published = s.published,
            audience = s.audience,
            steps = s.steps.map { st ->
                GuideStepInReq(
                    id = st.id,
                    title_en = st.titleEn.trim(),
                    title_zh = st.titleZh.trim(),
                    body_en = st.bodyEn.trim(),
                    body_zh = st.bodyZh.trim(),
                    link = st.link.trim().ifBlank { null },
                    image_file_id = st.imageFileId,
                    note_kind = st.noteKind.ifBlank { null },
                    note_en = st.noteEn.trim().ifBlank { null },
                    note_zh = st.noteZh.trim().ifBlank { null },
                )
            },
        )
        _state.value = s.copy(saving = true, error = null)
        viewModelScope.launch {
            try {
                val targetKey = if (s.guideId != null) {
                    container.help.updateGuide(s.guideId.toString(), req)
                    req.slug
                } else {
                    container.help.createGuide(req)
                    req.slug
                }
                _state.value = _state.value.copy(saving = false, savedKey = targetKey)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(saving = false, error = e.message)
            }
        }
    }

    fun deleteGuide() {
        val target = _state.value.guideId?.toString() ?: _state.value.editKey ?: return
        if (_state.value.saving) return
        _state.value = _state.value.copy(saving = true, error = null)
        viewModelScope.launch {
            try {
                container.help.deleteGuide(target)
                _state.value = _state.value.copy(saving = false, deleted = true)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(saving = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, key: String?) = viewModelFactory {
            initializer { HelpGuideEditorViewModel(container, key) }
        }
    }
}
