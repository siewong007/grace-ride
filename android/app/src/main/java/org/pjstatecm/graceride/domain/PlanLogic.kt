package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.data.api.DraftVisitorEntryDto
import org.pjstatecm.graceride.data.api.PlanChangeDto
import org.pjstatecm.graceride.data.api.PlanDraftEntryDto
import org.pjstatecm.graceride.data.api.PlanDriverDto
import org.pjstatecm.graceride.data.api.PlanRideDto
import org.pjstatecm.graceride.data.api.PlanVisitorDto

/** Seat load summary on a given event leg for one driver. */
data class DriverLoad(
    val seats: Long,
    val rides: Long,
)

/** Projected visitor row on the event board (live visitors merged with draft visitor mutations). */
data class EffectiveVisitor(
    val key: String,
    val liveVisitorId: Long?,
    val tempId: String?,
    val draftOp: String?, // "add" | "update" | "delete" | null
    val name: String,
    val phone: String,
    val notes: String,
    val areaId: Long?,
    val areaName: String?,
    val pickupAddress: String,
    val seats: Long,
    val leg: String,
    val driverId: Long?,
    val driverName: String?,
    val liveSource: PlanVisitorDto?,
)

/** Summary of assignment and visitor changes inside an event plan draft compared with live rides/visitors. */
data class PlanDraftDiffSummary(
    val assignmentChanges: Int,
    val visitorAdds: Int,
    val visitorUpdates: Int,
    val visitorDeletes: Int,
) {
    val totalChanges: Int get() = assignmentChanges + visitorAdds + visitorUpdates + visitorDeletes
}

/**
 * Pure Kotlin parity port of `frontend/src/pages/admin/planLogic.ts`.
 * Drives constraint checking (group match, area match, accessibility, carpool/seat capacity),
 * batch assignment helpers (`applyLastTime`, `applyMatchReturnDrivers`), draft serialization,
 * and visitor-merge/diff calculations.
 */
object PlanLogic {
    private val LEG_RANK = mapOf("outbound" to 0, "return" to 1)

    fun legRank(leg: String): Int = LEG_RANK[leg] ?: 0

    /** True when the driver is permitted to serve the ride's group scope. */
    fun inGroup(ride: PlanRideDto, driver: PlanDriverDto): Boolean =
        ride.matchGroupId == null || ride.matchGroupId in driver.driveGroupIds

    /** True when the driver's vehicle satisfies wheelchair, low-step, and child-seat needs. */
    fun vehicleCompatible(ride: PlanRideDto, driver: PlanDriverDto): Boolean {
        val v = driver.vehicle
        val aid = ride.assistance
        if ("wheelchair" in aid && v?.wheelchairOk != true) return false
        if ("low_step" in aid && v?.lowStep != true) return false
        if ("child_seat" in aid && v?.childSeat != true) return false
        return true
    }

    /** True when the ride's pickup area is in the driver's configured service areas (or unrestricted). */
    fun servesArea(ride: PlanRideDto, driver: PlanDriverDto): Boolean =
        driver.areaIds.isEmpty() || ride.area.id in driver.areaIds

    /** Computes seats and booking count already assigned to [driverId] on [leg] (across plan rides + outside jobs). */
    fun driverLoad(
        driverId: Long,
        leg: String,
        rides: List<PlanRideDto>,
        assign: Map<Long, Long?>,
        driversById: Map<Long, PlanDriverDto>,
    ): DriverLoad {
        val driver = driversById[driverId]
        var seats = if (leg == "return") {
            driver?.otherReturnSeats ?: 0L
        } else {
            driver?.otherOutboundSeats ?: driver?.otherSeats ?: 0L
        }
        var rideCount = if (leg == "return") {
            driver?.otherReturnRides ?: 0L
        } else {
            driver?.otherOutboundRides ?: driver?.otherRides ?: 0L
        }
        for (ride in rides) {
            if (ride.leg == leg && assign[ride.id] == driverId) {
                seats += ride.seats
                rideCount += 1L
            }
        }
        return DriverLoad(seats = seats, rides = rideCount)
    }

    /** True when [driver] can take [ride] given current [assign] state without violating seats/carpool/group/vehicle. */
    fun fits(
        driver: PlanDriverDto,
        ride: PlanRideDto,
        rides: List<PlanRideDto>,
        assign: Map<Long, Long?>,
        driversById: Map<Long, PlanDriverDto>,
    ): Boolean {
        if (!inGroup(ride, driver) || !vehicleCompatible(ride, driver)) return false
        val load = driverLoad(driver.id, ride.leg, rides, assign, driversById)
        val currentlyHere = assign[ride.id] == driver.id
        if (!driver.carpool && (load.rides - if (currentlyHere) 1L else 0L) > 0L) return false
        val used = load.seats - if (currentlyHere) ride.seats else 0L
        return used + ride.seats <= driver.cap
    }

    /** Assigns every unplaced ride whose previous driver still fits; returns new map + placement count. */
    fun applyLastTime(
        rides: List<PlanRideDto>,
        assign: Map<Long, Long?>,
        driversById: Map<Long, PlanDriverDto>,
    ): Pair<Map<Long, Long?>, Int> {
        val next = assign.toMutableMap()
        var placed = 0
        for (ride in rides) {
            if (next[ride.id] != null) continue
            val lastDriverId = ride.last?.driverId ?: continue
            val driver = driversById[lastDriverId] ?: continue
            if (fits(driver, ride, rides, next, driversById)) {
                next[ride.id] = lastDriverId
                placed++
            }
        }
        return next to placed
    }

    /** Mirrors each assigned outbound driver onto the paired return ride if still unassigned and fitting. */
    fun applyMatchReturnDrivers(
        rides: List<PlanRideDto>,
        assign: Map<Long, Long?>,
        driversById: Map<Long, PlanDriverDto>,
    ): Pair<Map<Long, Long?>, Int> {
        val byId = rides.associateBy { it.id }
        val next = assign.toMutableMap()
        var matched = 0
        for (ride in rides) {
            if (ride.leg != "return" || next[ride.id] != null) continue
            val pairedId = ride.pairedRideId ?: continue
            val outboundDriverId = assign[pairedId] ?: byId[pairedId]?.driverId ?: continue
            val driver = driversById[outboundDriverId] ?: continue
            if (fits(driver, ride, rides, next, driversById)) {
                next[ride.id] = outboundDriverId
                matched++
            }
        }
        return next to matched
    }

    /** Builds only the modified assignment diffs to submit via `/plan` or `/plan/approve`. */
    fun changesOf(rides: List<PlanRideDto>, assign: Map<Long, Long?>): List<PlanChangeDto> =
        rides.mapNotNull { ride ->
            val current = assign[ride.id]
            if (current == ride.driverId) null
            else PlanChangeDto(rideId = ride.id, fromDriverId = ride.driverId, driverId = current)
        }

    /** Serializes the full current assignment map as sorted draft entries. */
    fun draftEntries(rides: List<PlanRideDto>, assign: Map<Long, Long?>): List<PlanDraftEntryDto> =
        rides.map { ride ->
            PlanDraftEntryDto(
                rideId = ride.id,
                driverId = if (assign.containsKey(ride.id)) assign[ride.id] else ride.driverId,
            )
        }

    /** Merges live visitors with staged draft add/update/delete entries for display & editing. */
    fun mergeEffectiveVisitors(
        liveVisitors: List<PlanVisitorDto>,
        draftVisitors: List<DraftVisitorEntryDto>,
    ): List<EffectiveVisitor> {
        val deletedLiveIds = draftVisitors
            .filter { it.op == "delete" && it.liveVisitorId != null }
            .mapNotNull { it.liveVisitorId }
            .toSet()
        val updatedByLiveId = draftVisitors
            .filter { it.op == "update" && it.liveVisitorId != null }
            .associateBy { it.liveVisitorId!! }

        val merged = mutableListOf<EffectiveVisitor>()
        for (live in liveVisitors) {
            if (live.id in deletedLiveIds) {
                merged += EffectiveVisitor(
                    key = "live-${live.id}",
                    liveVisitorId = live.id,
                    tempId = null,
                    draftOp = "delete",
                    name = live.name,
                    phone = live.phone,
                    notes = live.notes,
                    areaId = live.area?.id,
                    areaName = live.area?.name,
                    pickupAddress = live.pickupAddress,
                    seats = live.seats,
                    leg = live.leg,
                    driverId = live.driverId,
                    driverName = live.driverName,
                    liveSource = live,
                )
            } else {
                val mod = updatedByLiveId[live.id]
                merged += if (mod != null) {
                    EffectiveVisitor(
                        key = "live-${live.id}",
                        liveVisitorId = live.id,
                        tempId = mod.tempId.ifBlank { null },
                        draftOp = "update",
                        name = mod.name,
                        phone = mod.phone,
                        notes = mod.notes,
                        areaId = mod.areaId,
                        areaName = mod.areaName ?: live.area?.name,
                        pickupAddress = mod.pickupAddress,
                        seats = mod.seats ?: live.seats,
                        leg = mod.leg ?: live.leg,
                        driverId = mod.driverId,
                        driverName = mod.driverName,
                        liveSource = live,
                    )
                } else {
                    EffectiveVisitor(
                        key = "live-${live.id}",
                        liveVisitorId = live.id,
                        tempId = null,
                        draftOp = null,
                        name = live.name,
                        phone = live.phone,
                        notes = live.notes,
                        areaId = live.area?.id,
                        areaName = live.area?.name,
                        pickupAddress = live.pickupAddress,
                        seats = live.seats,
                        leg = live.leg,
                        driverId = live.driverId,
                        driverName = live.driverName,
                        liveSource = live,
                    )
                }
            }
        }
        for ((idx, added) in draftVisitors.filter { it.op == "add" }.withIndex()) {
            val tid = added.tempId.ifBlank { "draft-$idx-${added.name}" }
            merged += EffectiveVisitor(
                key = "draft-$tid",
                liveVisitorId = null,
                tempId = tid,
                draftOp = "add",
                name = added.name,
                phone = added.phone,
                notes = added.notes,
                areaId = added.areaId,
                areaName = added.areaName,
                pickupAddress = added.pickupAddress,
                seats = added.seats ?: 1L,
                leg = added.leg ?: "outbound",
                driverId = added.driverId,
                driverName = added.driverName,
                liveSource = null,
            )
        }
        return merged
    }

    /** Summarizes how a draft differs from live assignments and live visitors. */
    fun summarizeDraftDiff(
        rides: List<PlanRideDto>,
        draftAssignments: List<PlanDraftEntryDto>,
        draftVisitors: List<DraftVisitorEntryDto>,
    ): PlanDraftDiffSummary {
        val byRide = draftAssignments.associate { it.rideId to it.driverId }
        val assignmentChanges = rides.count { ride ->
            byRide.containsKey(ride.id) && byRide[ride.id] != ride.driverId
        }
        return PlanDraftDiffSummary(
            assignmentChanges = assignmentChanges,
            visitorAdds = draftVisitors.count { it.op == "add" },
            visitorUpdates = draftVisitors.count { it.op == "update" },
            visitorDeletes = draftVisitors.count { it.op == "delete" },
        )
    }
}
