package org.pjstatecm.graceride.ui.screens.public

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ExpandLess
import androidx.compose.material.icons.outlined.ExpandMore
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.LicensePackageDto
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.LoadingView
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LicensesScreen(
    container: AppContainer,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    var packages by remember { mutableStateOf<List<LicensePackageDto>?>(null) }
    var loadError by remember { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    var expandedKey by rememberSaveable { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        val bundled = withContext(Dispatchers.IO) {
            runCatching {
                context.assets.open("third-party-licenses.json").bufferedReader().use { reader ->
                    apiJson.decodeFromString<List<LicensePackageDto>>(reader.readText())
                }
            }.getOrNull()
        }
        if (bundled != null) {
            packages = bundled
        } else {
            val remote = runCatching { container.publicFlow.thirdPartyLicenses() }.getOrNull()
            if (remote != null) {
                packages = remote
            } else {
                loadError = true
            }
        }
    }

    val filtered = remember(packages, query) {
        val list = packages.orEmpty()
        val q = query.trim().lowercase()
        if (q.isEmpty()) list
        else list.filter {
            it.name.lowercase().contains(q) ||
                it.version.lowercase().contains(q) ||
                it.license.lowercase().contains(q)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.licenses_title)) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.common_back),
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        containerColor = MaterialTheme.colorScheme.background,
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = Spacing.sm),
            contentPadding = PaddingValues(vertical = Spacing.sm),
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            item {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Column(
                        Modifier.padding(Spacing.sm),
                        verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                    ) {
                        Text(
                            stringResource(R.string.licenses_intro),
                            style = MaterialTheme.typography.bodyMedium,
                        )
                        Text(
                            stringResource(R.string.licenses_osm_notice),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        packages?.let { list ->
                            Text(
                                stringResource(R.string.licenses_count, list.size),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                }
            }

            if (packages != null) {
                item {
                    OutlinedTextField(
                        value = query,
                        onValueChange = { query = it },
                        label = { Text(stringResource(R.string.licenses_filter_label)) },
                        placeholder = { Text(stringResource(R.string.licenses_filter_hint)) },
                        leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                        trailingIcon = if (query.isNotEmpty()) {
                            {
                                IconButton(onClick = { query = "" }) {
                                    Icon(Icons.Outlined.Close, contentDescription = stringResource(R.string.a11y_clear))
                                }
                            }
                        } else {
                            null
                        },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }

            when {
                loadError -> item {
                    ErrorView(message = stringResource(R.string.licenses_error))
                }
                packages == null -> item {
                    LoadingView()
                }
                filtered.isEmpty() -> item {
                    EmptyView(
                        text = stringResource(R.string.licenses_empty),
                        subtitle = query.takeIf { it.isNotBlank() },
                    )
                }
                else -> items(filtered, key = { "${it.name}@${it.version}" }) { pkg ->
                    val key = "${pkg.name}@${pkg.version}"
                    val isExpanded = expandedKey == key
                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                    ) {
                        Surface(
                            onClick = {
                                if (pkg.text.isNotBlank()) {
                                    expandedKey = if (isExpanded) null else key
                                }
                            },
                            color = MaterialTheme.colorScheme.surface,
                        ) {
                            Column(
                                Modifier
                                    .fillMaxWidth()
                                    .padding(Spacing.sm),
                                verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            ) {
                                Row(
                                    Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(Modifier.weight(1f)) {
                                        Text(
                                            text = if (pkg.version.isNotBlank()) "${pkg.name} ${pkg.version}" else pkg.name,
                                            style = MaterialTheme.typography.titleSmall,
                                        )
                                        if (pkg.license.isNotBlank()) {
                                            Text(
                                                text = pkg.license,
                                                style = MaterialTheme.typography.labelMedium,
                                                color = MaterialTheme.colorScheme.primary,
                                            )
                                        }
                                    }
                                    if (pkg.text.isNotBlank()) {
                                        Icon(
                                            if (isExpanded) Icons.Outlined.ExpandLess else Icons.Outlined.ExpandMore,
                                            contentDescription = null,
                                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                                AnimatedVisibility(visible = isExpanded && pkg.text.isNotBlank()) {
                                    Text(
                                        text = pkg.text,
                                        style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
