import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../rust_gen/api/simple.dart' as rust;
import 'settings_provider.dart';

// ── State providers (unchanged — UI reads these) ──────────────────────────
final vpnStatusProvider = StateProvider<String>((ref) => "Disconnected");
final vpnDownloadSpeedProvider = StateProvider<double>((ref) => 0.0);
final vpnUploadSpeedProvider = StateProvider<double>((ref) => 0.0);
final vpnProtocolProvider = StateProvider<String>((ref) => "Idle");

/// MethodChannel vers WorldVpnService.kt → libvpngo.so
const _vpnChannel = MethodChannel('worldvpn/vpn');

// ── Controller provider ───────────────────────────────────────────────────
final vpnControllerProvider = Provider((ref) {
  final controller = VpnController(ref);
  controller.init();
  return controller;
});

class VpnController {
  final Ref ref;
  VpnController(this.ref);

  void init() {
    // Écoute toujours le flux Rust pour les stats (NAT, partage, etc.)
    rust.registerStatusStream().listen((event) {
      ref.read(vpnStatusProvider.notifier).state = event.status;
      ref.read(vpnProtocolProvider.notifier).state = event.protocol;
      ref.read(vpnDownloadSpeedProvider.notifier).state = event.downloadSpeed;
      ref.read(vpnUploadSpeedProvider.notifier).state = event.uploadSpeed;
    }, onError: (_) {
      ref.read(vpnStatusProvider.notifier).state = "Stream Error";
    });
  }

  /// Connexion au VPN.
  /// 1. Rust fait la sélection de protocole + appel backend matchmaking
  /// 2. Flutter demande la permission Android VPN (popup unique)
  /// 3. Flutter démarre WorldVpnService (Go tunnel)
  Future<void> connect(String nodeGroup) async {
    final settings = ref.read(settingsProvider);
    ref.read(vpnStatusProvider.notifier).state = "Connecting…";

    try {
      // ── 1. Matchmaking backend via Rust (inchangé) ──────────────────
      // startVpnMatchmaking retourne le ConnectResponse JSON brut
      final connectJson = await rust.startVpnMatchmaking(
        protocolStr: settings.protocol,
        countryCode: "FR",
        nodeGroup: nodeGroup,
      );

      // ── 2. Demander la permission VPN Android (popup unique) ─────────
      final bool granted = await _vpnChannel.invokeMethod('prepare');
      if (!granted) {
        ref.read(vpnStatusProvider.notifier).state = "Disconnected";
        return;
      }

      // ── 3. Démarrer le tunnel Go via Kotlin ─────────────────────────
      await _vpnChannel.invokeMethod('start', {
        'connect_response': connectJson,
      });

      ref.read(vpnStatusProvider.notifier).state = "Connected";
    } on PlatformException catch (e) {
      ref.read(vpnStatusProvider.notifier).state = "Error: ${e.message}";
    } catch (e) {
      ref.read(vpnStatusProvider.notifier).state = "Error: $e";
    }
  }

  Future<void> disconnect() async {
    try {
      await _vpnChannel.invokeMethod('stop');
      await rust.stopVpnConnection();
      ref.read(vpnStatusProvider.notifier).state = "Disconnected";
      ref.read(vpnDownloadSpeedProvider.notifier).state = 0.0;
      ref.read(vpnUploadSpeedProvider.notifier).state = 0.0;
    } catch (_) {}
  }

  Future<void> startSharing() async {
    try {
      await rust.startSharing();
    } catch (_) {}
  }

  Future<void> stopSharing() async {
    try {
      await rust.stopSharing();
    } catch (_) {}
  }
}
