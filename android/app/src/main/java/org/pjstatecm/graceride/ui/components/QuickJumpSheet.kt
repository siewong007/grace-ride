package org.pjstatecm.graceride.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.AltRoute
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.automirrored.outlined.ReceiptLong
import androidx.compose.material.icons.outlined.AddCircleOutline
import androidx.compose.material.icons.outlined.AdminPanelSettings
import androidx.compose.material.icons.outlined.Assessment
import androidx.compose.material.icons.outlined.BarChart
import androidx.compose.material.icons.outlined.BookmarkBorder
import androidx.compose.material.icons.outlined.Campaign
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Event
import androidx.compose.material.icons.outlined.FamilyRestroom
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.Payments
import androidx.compose.material.icons.outlined.PeopleOutline
import androidx.compose.material.icons.outlined.PersonOutline
import androidx.compose.material.icons.outlined.RequestQuote
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Security
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.theme.Spacing

private enum class QuickJumpGroup { ACTIONS, MEMBER, ADMIN }

private class QuickJumpEntry(
    val route: AppRoute,
    val labelRes: Int,
    val subtitleRes: Int?,
    val icon: ImageVector,
    val isAction: Boolean,
    val group: QuickJumpGroup,
)

/**
 * Command-palette quick jump sheet matching web `QuickJump.tsx`,
 * upgraded for mobile discoverability with All / Actions / Admin filter chips and grouped headers.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QuickJumpSheet(
    me: Me?,
    onNavigate: (AppRoute) -> Unit,
    onDismiss: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val view = LocalView.current
    var query by remember { mutableStateOf("") }
    var scopeFilter by remember { mutableStateOf("all") }

    val rawEntries = buildList {
        fun memberPage(route: AppRoute, labelRes: Int, icon: ImageVector) =
            add(QuickJumpEntry(route, labelRes, null, icon, false, QuickJumpGroup.MEMBER))

        fun adminPage(route: AppRoute, labelRes: Int, icon: ImageVector) =
            add(QuickJumpEntry(route, labelRes, null, icon, false, QuickJumpGroup.ADMIN))

        fun quickAction(route: AppRoute, labelRes: Int, icon: ImageVector) =
            add(QuickJumpEntry(route, labelRes, R.string.quick_jump_action_tag, icon, true, QuickJumpGroup.ACTIONS))

        // Quick actions first when filtered or highlighted
        quickAction(AppRoute.RideNew(), R.string.quick_jump_action_book_ride, Icons.Outlined.AddCircleOutline)
        quickAction(AppRoute.EventNew(), R.string.quick_jump_action_new_event, Icons.Outlined.AddCircleOutline)
        quickAction(AppRoute.ReimbursementForm(), R.string.reimb_new, Icons.Outlined.AddCircleOutline)
        quickAction(AppRoute.TrafficHub(tab = "rides"), R.string.quick_jump_action_traffic_rides, Icons.Outlined.AdminPanelSettings)
        quickAction(AppRoute.TrafficHub(tab = "live"), R.string.quick_jump_action_live_map, Icons.Outlined.Map)

        memberPage(AppRoute.Home, R.string.nav_home, Icons.Outlined.Home)
        memberPage(AppRoute.Rides, R.string.nav_rides, Icons.Outlined.DirectionsCar)
        memberPage(AppRoute.Events, R.string.nav_events, Icons.Outlined.Event)
        memberPage(AppRoute.Costs, R.string.costs_title, Icons.Outlined.Payments)
        memberPage(AppRoute.History, R.string.history_title, Icons.Outlined.History)
        memberPage(AppRoute.Notifications, R.string.notif_title, Icons.Outlined.Notifications)
        memberPage(AppRoute.Profile, R.string.nav_profile, Icons.Outlined.PersonOutline)
        memberPage(AppRoute.SavedAddresses, R.string.saved_addresses_title, Icons.Outlined.BookmarkBorder)
        memberPage(AppRoute.Family, R.string.profile_family, Icons.Outlined.FamilyRestroom)
        memberPage(AppRoute.SecuritySettings, R.string.profile_security, Icons.Outlined.Security)
        memberPage(AppRoute.Reimbursements, R.string.reimb_title, Icons.Outlined.RequestQuote)
        memberPage(AppRoute.Help(), R.string.help_title, Icons.AutoMirrored.Outlined.HelpOutline)
        memberPage(AppRoute.Report, R.string.report_title, Icons.Outlined.Assessment)

        adminPage(AppRoute.DriveHub, R.string.drive_hub_title, Icons.AutoMirrored.Outlined.AltRoute)
        adminPage(AppRoute.TrafficHub(tab = "plans"), R.string.nav_traffic, Icons.Outlined.Map)
        adminPage(AppRoute.AdminConsole, R.string.admin_console_title, Icons.Outlined.AdminPanelSettings)
        adminPage(AppRoute.AdminMembers(tab = "members"), R.string.profile_admin_members, Icons.Outlined.PeopleOutline)
        adminPage(AppRoute.Announcements(manage = true), R.string.home_admin_announcements_btn, Icons.Outlined.Campaign)
        adminPage(AppRoute.Settlements, R.string.settlements_title, Icons.AutoMirrored.Outlined.ReceiptLong)
        adminPage(AppRoute.AdminReimbursements(), R.string.reimb_admin_title, Icons.Outlined.AdminPanelSettings)
        adminPage(AppRoute.AdminReports(), R.string.admin_reports_title, Icons.Outlined.BarChart)
        adminPage(AppRoute.AdminRoles, R.string.roles_title, Icons.Outlined.Shield)
        adminPage(AppRoute.AdminAudit, R.string.audit_title, Icons.Outlined.History)
        adminPage(AppRoute.AdminSettings(), R.string.settings_title, Icons.Outlined.Settings)
    }.filter { entry ->
        me == null || Routes.isPermitted(entry.route, me)
    }

    val hasAdminEntries = rawEntries.any { it.group == QuickJumpGroup.ADMIN }
    val labeled = rawEntries.map { it to stringResource(it.labelRes) }
    val trimmed = query.trim()
    val prefixAction = trimmed.startsWith(">")
    val effectiveScope = if (prefixAction) "actions" else scopeFilter
    val term = trimmed.removePrefix(">").trim().lowercase()
    val filtered = labeled.filter { (entry, label) ->
        when (effectiveScope) {
            "actions" -> if (!entry.isAction) return@filter false
            "admin" -> if (entry.group != QuickJumpGroup.ADMIN) return@filter false
        }
        if (term.isEmpty()) true else label.lowercase().contains(term)
    }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = Spacing.sm)
                .padding(bottom = Spacing.md),
            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.quick_jump_title),
                    style = MaterialTheme.typography.titleMedium,
                )
                IconButton(onClick = onDismiss) {
                    Icon(
                        Icons.Outlined.Close,
                        contentDescription = stringResource(R.string.common_close),
                    )
                }
            }
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text(stringResource(R.string.quick_jump_placeholder)) },
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
            LazyRow(horizontalArrangement = Arrangement.spacedBy(Spacing.xs)) {
                item {
                    FilterChip(
                        selected = effectiveScope == "all",
                        onClick = {
                            UiHaptics.tick(view)
                            scopeFilter = "all"
                            if (prefixAction) query = trimmed.removePrefix(">").trim()
                        },
                        label = { Text(stringResource(R.string.quick_jump_filter_all)) },
                    )
                }
                item {
                    FilterChip(
                        selected = effectiveScope == "actions",
                        onClick = {
                            UiHaptics.tick(view)
                            scopeFilter = "actions"
                        },
                        label = { Text(stringResource(R.string.quick_jump_filter_actions)) },
                    )
                }
                if (hasAdminEntries) {
                    item {
                        FilterChip(
                            selected = effectiveScope == "admin",
                            onClick = {
                                UiHaptics.tick(view)
                                scopeFilter = "admin"
                                if (prefixAction) query = trimmed.removePrefix(">").trim()
                            },
                            label = { Text(stringResource(R.string.quick_jump_filter_admin)) },
                        )
                    }
                }
            }
            LazyColumn(
                Modifier
                    .fillMaxWidth()
                    .weight(1f, fill = false)
                    .heightIn(max = 400.dp),
            ) {
                itemsIndexed(filtered, key = { idx, (entry, _) -> "${entry.route}-${entry.isAction}-$idx" }) { idx, (entry, label) ->
                    val prevGroup = filtered.getOrNull(idx - 1)?.first?.group
                    if (idx == 0 || prevGroup != entry.group) {
                        val headerRes = when (entry.group) {
                            QuickJumpGroup.ACTIONS -> R.string.quick_jump_section_actions
                            QuickJumpGroup.MEMBER -> R.string.quick_jump_section_member
                            QuickJumpGroup.ADMIN -> R.string.quick_jump_section_admin
                        }
                        Text(
                            text = stringResource(headerRes),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = Spacing.xs, bottom = Spacing.xxs, start = Spacing.xxs),
                        )
                    } else {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    }
                    Surface(
                        onClick = {
                            UiHaptics.tick(view)
                            onDismiss()
                            onNavigate(entry.route)
                        },
                        color = MaterialTheme.colorScheme.surface,
                    ) {
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = 52.dp)
                                .padding(horizontal = Spacing.xxs, vertical = Spacing.xs),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                        ) {
                            Surface(
                                color = MaterialTheme.colorScheme.primaryContainer,
                                shape = CircleShape,
                                modifier = Modifier.size(36.dp),
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    Icon(
                                        entry.icon,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.primary,
                                        modifier = Modifier.size(18.dp),
                                    )
                                }
                            }
                            Text(
                                label,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.Medium,
                                modifier = Modifier.weight(1f),
                            )
                            entry.subtitleRes?.let { subRes ->
                                Surface(
                                    color = MaterialTheme.colorScheme.primaryContainer,
                                    contentColor = MaterialTheme.colorScheme.primary,
                                    shape = CircleShape,
                                ) {
                                    Text(
                                        stringResource(subRes),
                                        style = MaterialTheme.typography.labelSmall,
                                        modifier = Modifier.padding(horizontal = Spacing.xs, vertical = 2.dp),
                                    )
                                }
                            }
                            Icon(
                                Icons.Outlined.ChevronRight,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.size(18.dp),
                            )
                        }
                    }
                }
            }
        }
    }
}
