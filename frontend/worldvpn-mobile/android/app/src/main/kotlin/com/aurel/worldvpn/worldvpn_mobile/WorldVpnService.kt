package com.aurel.worldvpn.worldvpn_mobile

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor

class WorldVpnService : VpnService() {

    companion object {
        // Charge libvpngo.so compilée depuis vpn-go/
        init { System.loadLibrary("vpngo") }

        const val EXTRA_CONFIG = "connect_response_json"
    }

    private var tunInterface: ParcelFileDescriptor? = null

    // JNI — fonctions exportées depuis tunnel.go via CGO
    private external fun StartTunnel(tunFd: Int, configJson: String): Int
    private external fun StopTunnel()

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val configJson = intent?.getStringExtra(EXTRA_CONFIG)
            ?: return START_NOT_STICKY

        startForeground(1, buildNotification())

        // Construit l'interface TUN : Android route TOUT le trafic dedans
        tunInterface = Builder()
            .setSession("WorldVPN")
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)           // IPv4 complet
            .addRoute("::", 0)                 // IPv6 complet
            .addDnsServer("1.1.1.1")
            .addDnsServer("1.0.0.1")
            .setMtu(1420)
            .setBlocking(false)
            // Exclure l'app elle-même (évite la boucle sur ses propres appels backend)
            .addDisallowedApplication(packageName)
            .establish()

        val fd = tunInterface?.detachFd()
        if (fd == null) {
            stopSelf()
            return START_NOT_STICKY
        }

        // Démarrer le tunnel Go dans un thread séparé
        Thread {
            val result = StartTunnel(fd, configJson)
            if (result != 0) {
                android.util.Log.e("WorldVpnService", "StartTunnel failed: $result")
                stopSelf()
            }
        }.apply { isDaemon = true; name = "vpngo-tunnel"; start() }

        return START_STICKY
    }

    override fun onDestroy() {
        StopTunnel()
        tunInterface?.close()
        super.onDestroy()
    }

    private fun buildNotification(): Notification {
        val channelId = "worldvpn_channel"
        val manager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager

        manager.createNotificationChannel(
            NotificationChannel(channelId, "WorldVPN", NotificationManager.IMPORTANCE_LOW)
                .apply { description = "Tunnel VPN actif" }
        )

        return Notification.Builder(this, channelId)
            .setContentTitle("WorldVPN actif")
            .setContentText("Connexion sécurisée en cours…")
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .build()
    }
}
