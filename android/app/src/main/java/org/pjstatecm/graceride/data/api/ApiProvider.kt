package org.pjstatecm.graceride.data.api

/**
 * Thread-safe delegate holding the active [ApiService] instance so repositories stay immutable
 * even when debug builds switch server base URLs via `AppContainer.setBaseUrl`.
 */
class ApiProvider(initial: ApiService) {
    @Volatile
    private var current: ApiService = initial

    fun get(): ApiService = current

    fun swap(next: ApiService) {
        current = next
    }
}
