package org.pjstatecm.graceride.notifications

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import org.pjstatecm.graceride.GraceRideApp
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.domain.KlTime
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Periodic fallback notification sync so Android still surfaces unread ride and trip
 * notifications when the process has no open foreground SSE stream.
 */
class BackgroundNotificationWorker(
    appContext: Context,
    params: WorkerParameters,
) : CoroutineWorker(appContext, params) {

    override suspend fun doWork(): Result {
        val app = applicationContext as? GraceRideApp ?: return Result.success()
        val container = app.container
        var signedIn = container.auth.state.value as? AuthState.SignedIn
        if (signedIn == null) {
            if (!container.cookieJar.hasCookies()) {
                return Result.success()
            }
            try {
                container.auth.refresh(throwOnError = true)
                signedIn = container.auth.state.value as? AuthState.SignedIn
            } catch (e: ApiException) {
                return if (e.status == 401 || e.status == 403 || e.status == 423 || runAttemptCount >= 3) {
                    Result.success()
                } else {
                    Result.retry()
                }
            } catch (_: IOException) {
                return if (runAttemptCount < 3) Result.retry() else Result.success()
            }
        }
        val me = signedIn?.me ?: return Result.success()
        if (!me.termsAccepted) return Result.success()
        val uid = me.id

        return try {
            val prefs = runSuspendCatching { container.community.notificationPrefs(passive = true) }.getOrNull()?.also {
                RealtimeStreamClient.writeCachedPrefs(container.secureStore, it)
            } ?: RealtimeStreamClient.readCachedPrefs(container.secureStore)
            val hideLockDetails = container.secureStore.getString("notif_hide_lock_details") == "1"
            RealtimeStreamClient.syncNotifications(
                community = container.community,
                auth = container.auth,
                store = container.secureStore,
                userId = uid,
                appContext = applicationContext,
                surfaceNew = true,
                lang = container.lang,
                hideLockDetails = hideLockDetails,
                prefs = prefs,
                passive = true,
            )
            // Keep home-screen widget fresh when scheduled worker wakes up.
            val nextRide = if (me.can("rides")) {
                runSuspendCatching {
                    container.rides.upcoming(passive = true)
                        .filter { it.status in org.pjstatecm.graceride.domain.RideStatus.active && it.viewer.isPassenger }
                        .minByOrNull { KlTime.sortKey(it.pickupAt) }
                }.getOrNull()
            } else null
            val nextEvent = if (nextRide == null && me.can("events")) {
                runSuspendCatching { container.events.upcoming(passive = true).firstOrNull() }.getOrNull()
            } else null
            val stillSignedIn = (container.auth.state.value as? AuthState.SignedIn)?.me?.id == uid
            if (stillSignedIn) {
                org.pjstatecm.graceride.widget.WidgetSnapshotStore.saveFromHome(
                    applicationContext,
                    signedIn = true,
                    nextRide = nextRide,
                    nextEvent = nextEvent,
                    hideLockDetails = hideLockDetails || (prefs?.hideLockDetails == true),
                )
                org.pjstatecm.graceride.widget.refreshGraceRideWidget(applicationContext)
            }
            Result.success()
        } catch (e: ApiException) {
            if (e.status == 401 || e.status == 423 || runAttemptCount >= 3) {
                Result.success()
            } else {
                Result.retry()
            }
        } catch (_: IOException) {
            if (runAttemptCount < 3) Result.retry() else Result.success()
        }
    }

    companion object {
        private const val UNIQUE_WORK_NAME = "gr_bg_notif_sync"

        fun schedule(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()
            val request = PeriodicWorkRequestBuilder<BackgroundNotificationWorker>(
                15,
                TimeUnit.MINUTES,
            )
                .setConstraints(constraints)
                .build()
            WorkManager.getInstance(context.applicationContext).enqueueUniquePeriodicWork(
                UNIQUE_WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                request,
            )
        }

        fun cancel(context: Context) {
            WorkManager.getInstance(context.applicationContext).cancelUniqueWork(UNIQUE_WORK_NAME)
        }
    }
}
