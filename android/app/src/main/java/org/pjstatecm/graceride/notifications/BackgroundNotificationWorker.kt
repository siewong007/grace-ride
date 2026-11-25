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
import org.pjstatecm.graceride.data.repo.AuthState
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
                container.auth.refresh()
                signedIn = container.auth.state.value as? AuthState.SignedIn
            } catch (e: ApiException) {
                return if (e.status == 401 || e.status == 423 || runAttemptCount >= 3) {
                    Result.success()
                } else {
                    Result.retry()
                }
            } catch (_: IOException) {
                return if (runAttemptCount < 3) Result.retry() else Result.success()
            }
        }
        val uid = signedIn?.me?.id ?: return Result.success()

        return try {
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
            )
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
