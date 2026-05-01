package com.aurel.worldvpn.worldvpn_mobile

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {

    private val CHANNEL = "worldvpn/vpn"
    private val VPN_PERMISSION_REQ = 1337

    // Stores the pending result while Android shows the VPN permission dialog
    private var pendingPermResult: MethodChannel.Result? = null

    override fun configureFlutterEngine(engine: FlutterEngine) {
        super.configureFlutterEngine(engine)

        MethodChannel(engine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {

                    // ── Demander la permission VPN Android (popup unique) ──────────
                    "prepare" -> {
                        val intent = VpnService.prepare(this)
                        if (intent == null) {
                            // Déjà accordé — pas de popup
                            result.success(true)
                        } else {
                            pendingPermResult = result
                            startActivityForResult(intent, VPN_PERMISSION_REQ)
                        }
                    }

                    // ── Démarrer le tunnel ─────────────────────────────────────────
                    "start" -> {
                        val json = call.argument<String>("connect_response")
                        if (json == null) {
                            result.error("MISSING_CONFIG", "connect_response is required", null)
                            return@setMethodCallHandler
                        }
                        val svcIntent = Intent(this, WorldVpnService::class.java)
                            .putExtra(WorldVpnService.EXTRA_CONFIG, json)
                        startService(svcIntent)
                        result.success(null)
                    }

                    // ── Arrêter le tunnel ──────────────────────────────────────────
                    "stop" -> {
                        stopService(Intent(this, WorldVpnService::class.java))
                        result.success(null)
                    }

                    else -> result.notImplemented()
                }
            }
    }

    // Résultat de la popup Android "WorldVPN veut créer un réseau VPN"
    @Deprecated("Deprecated in Java")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode == VPN_PERMISSION_REQ) {
            pendingPermResult?.success(resultCode == Activity.RESULT_OK)
            pendingPermResult = null
        }
        super.onActivityResult(requestCode, resultCode, data)
    }
}
