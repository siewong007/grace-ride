package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.R

enum class RideStatusKind { ACTIVE, PENDING, DONE, CANCELLED }

object RideStatus {
    val active = setOf("pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed", "in_progress")
    val terminal = setOf("completed", "cancelled", "no_show", "alt_transport")

    fun labelRes(status: String): Int = when (status) {
        "pending_driver" -> R.string.ride_status_pending_driver
        "pending_passenger" -> R.string.ride_status_pending_passenger
        "pending_approval" -> R.string.ride_status_pending_approval
        "pending_driver_confirm" -> R.string.ride_status_pending_driver_confirm
        "confirmed" -> R.string.ride_status_confirmed
        "in_progress" -> R.string.ride_status_in_progress
        "completed" -> R.string.ride_status_completed
        "cancelled" -> R.string.ride_status_cancelled
        "no_show" -> R.string.ride_status_no_show
        "alt_transport" -> R.string.ride_status_alt_transport
        else -> R.string.ride_status_pending_driver
    }

    fun kind(status: String): RideStatusKind = when (status) {
        "confirmed", "in_progress" -> RideStatusKind.ACTIVE
        "completed" -> RideStatusKind.DONE
        "cancelled", "no_show", "alt_transport" -> RideStatusKind.CANCELLED
        else -> RideStatusKind.PENDING
    }

    /** May the passenger cancel from this state (mirrors backend guard)? */
    fun cancellable(status: String): Boolean = status in setOf(
        "pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed"
    )
}
