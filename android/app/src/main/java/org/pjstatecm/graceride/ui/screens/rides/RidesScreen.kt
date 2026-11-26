package org.pjstatecm.graceride.ui.screens.rides

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.AppRoute
import org.pjstatecm.graceride.ui.UiHaptics
import org.pjstatecm.graceride.ui.components.AnimatedOfflineBanner
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.GraceRideSpringEasing
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.SkeletonLoadingList
import org.pjstatecm.graceride.ui.components.animatorsEnabled
import org.pjstatecm.graceride.ui.navigateTab
import org.pjstatecm.graceride.ui.theme.GrPrimary
import org.pjstatecm.graceride.ui.theme.Spacing

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RidesScreen(container: AppContainer, nav: NavHostController) {
    val vm: RidesViewModel = viewModel(factory = RidesViewModel.factory(container))
    val state by vm.state.collectAsStateWithLifecycle()
    val view = LocalView.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var resumedOnce by remember { mutableStateOf(false) }

    DisposableEffect(lifecycleOwner) {
        val obs = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                if (resumedOnce) vm.refresh() else resumedOnce = true
            }
        }
        lifecycleOwner.lifecycle.addObserver(obs)
        onDispose { lifecycleOwner.lifecycle.removeObserver(obs) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.rides_title)) },
                navigationIcon = {
                    IconButton(onClick = { nav.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.a11y_back))
                    }
                },
            )
        },
        floatingActionButton = {
            if (state.tab == 0 && container.auth.me?.can("rides", "create") == true) {
                val bookLabel = stringResource(R.string.rides_book)
                ExtendedFloatingActionButton(
                    onClick = {
                        nav.popBackStack()
                        nav.navigateTab(AppRoute.Events)
                    },
                    containerColor = GrPrimary,
                    contentColor = Color.White,
                    modifier = Modifier.semantics(mergeDescendants = true) { contentDescription = bookLabel },
                    icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                    text = { Text(bookLabel) },
                )
            }
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            PrimaryTabRow(selectedTabIndex = state.tab) {
                Tab(selected = state.tab == 0, onClick = { UiHaptics.tick(view); vm.setTab(0) }, text = { Text(stringResource(R.string.rides_tab_upcoming)) })
                Tab(selected = state.tab == 1, onClick = { UiHaptics.tick(view); vm.setTab(1) }, text = { Text(stringResource(R.string.rides_tab_past)) })
            }
            AnimatedOfflineBanner(
                visible = !state.loading && state.offline,
                modifier = Modifier.padding(horizontal = Spacing.sm, vertical = Spacing.xxs),
            )
            val viewMode = when {
                state.loading -> "loading"
                state.error != null && state.rides.isEmpty() -> "error"
                else -> "tab-${state.tab}"
            }
            AnimatedContent(
                targetState = viewMode,
                transitionSpec = {
                    val tabTransition = initialState.startsWith("tab-") && targetState.startsWith("tab-")
                    if (tabTransition && animatorsEnabled()) {
                        val fromTab = initialState.removePrefix("tab-").toIntOrNull() ?: 0
                        val toTab = targetState.removePrefix("tab-").toIntOrNull() ?: 0
                        val dir = if (toTab > fromTab) 1 else -1
                        (slideInHorizontally(tween(240, easing = GraceRideSpringEasing)) { it * dir } +
                            fadeIn(tween(180)))
                            .togetherWith(
                                slideOutHorizontally(tween(220, easing = GraceRideSpringEasing)) { (-it / 3) * dir } +
                                    fadeOut(tween(140)),
                            )
                    } else {
                        fadeIn(tween(180)).togetherWith(fadeOut(tween(140)))
                    }
                },
                label = "rides_state",
                modifier = Modifier.weight(1f),
            ) { mode ->
                when (mode) {
                    "loading" -> SkeletonLoadingList(count = 4)
                    "error" -> ErrorView(state.error ?: "", vm::refresh)
                    else -> PullToRefreshBox(isRefreshing = state.refreshing, onRefresh = vm::refresh) {
                        val isUpcomingTab = mode == "tab-0"
                        LazyColumn(
                            Modifier.fillMaxSize().padding(horizontal = Spacing.sm),
                            verticalArrangement = Arrangement.spacedBy(Spacing.xs),
                            contentPadding = PaddingValues(top = Spacing.xs, bottom = if (isUpcomingTab) 88.dp else Spacing.xs),
                        ) {
                            if (state.rides.isEmpty()) {
                                item {
                                    EmptyView(
                                        text = stringResource(R.string.rides_empty),
                                        icon = Icons.Outlined.DirectionsCar,
                                    )
                                }
                            }
                            items(state.rides, key = { it.id }) { ride ->
                                RideCard(
                                    ride,
                                    photoUrlFor = container::fileUrl,
                                    onClick = { nav.navigate(AppRoute.RideDetail(ride.id)) },
                                    modifier = Modifier.animateItem(),
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
