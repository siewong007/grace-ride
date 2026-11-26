package org.pjstatecm.graceride.platform

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Reactive OS connectivity monitor exposing [isOnline]. Falls back to `true` when constructed
 * without an Android [Context] in JVM unit tests.
 */
class NetworkMonitor(context: Context?) {
    private val cm: ConnectivityManager? =
        context?.applicationContext?.getSystemService(ConnectivityManager::class.java)

    private val _isOnline = MutableStateFlow(computeOnline())
    val isOnline: StateFlow<Boolean> = _isOnline.asStateFlow()

    private val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) {
            _isOnline.value = computeOnline()
        }

        override fun onLost(network: Network) {
            _isOnline.value = false
        }

        override fun onCapabilitiesChanged(network: Network, networkCapabilities: NetworkCapabilities) {
            _isOnline.value = networkCapabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) &&
                networkCapabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)
        }
    }

    init {
        cm?.let {
            runCatching { it.registerDefaultNetworkCallback(callback) }
        }
    }

    private fun computeOnline(): Boolean {
        val manager = cm ?: return true
        val active = manager.activeNetwork ?: return false
        val caps = manager.getNetworkCapabilities(active) ?: return false
        return caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) &&
            caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)
    }

    fun unregister() {
        cm?.let { runCatching { it.unregisterNetworkCallback(callback) } }
    }
}
