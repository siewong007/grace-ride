package org.pjstatecm.graceride.notifications

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationManagerCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import org.pjstatecm.graceride.GraceRideApp

class ReadinessActionReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val rideId = intent.getLongExtra(EXTRA_RIDE_ID, -1L)
        val status = intent.getStringExtra(EXTRA_STATUS)
        val notifId = intent.getIntExtra(EXTRA_NOTIF_ID, -1)
        if (rideId <= 0L || status.isNullOrBlank()) return

        val app = context.applicationContext as? GraceRideApp ?: return
        val pending = goAsync()
        CoroutineScope(SupervisorJob() + Dispatchers.IO).launch {
            try {
                withTimeout(8_000) {
                    app.container.rides.readiness(rideId, status)
                    if (notifId > 0) {
                        NotificationManagerCompat.from(context).cancel(notifId)
                    }
                    org.pjstatecm.graceride.widget.WidgetSnapshotStore.markReadyLocally(context, rideId, status)
                    org.pjstatecm.graceride.widget.refreshGraceRideWidget(context)
                    app.container.auth.refresh()
                }
            } catch (_: Exception) {
                // Keep notification intact so user can tap into the ride detail screen.
            } finally {
                pending.finish()
            }
        }
    }

    companion object {
        private const val EXTRA_RIDE_ID = "ride_id"
        private const val EXTRA_STATUS = "readiness_status"
        private const val EXTRA_NOTIF_ID = "notif_id"

        fun createIntent(context: Context, rideId: Long, status: String, notifId: Int): Intent =
            Intent(context, ReadinessActionReceiver::class.java).apply {
                putExtra(EXTRA_RIDE_ID, rideId)
                putExtra(EXTRA_STATUS, status)
                putExtra(EXTRA_NOTIF_ID, notifId)
            }
    }
}
