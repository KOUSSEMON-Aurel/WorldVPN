import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../rust_gen/api/simple.dart' as rust;
import 'settings_provider.dart';

final vpnStatusProvider = StateProvider<String>((ref) => "Disconnected");
final vpnDownloadSpeedProvider = StateProvider<double>((ref) => 0.0);
final vpnUploadSpeedProvider = StateProvider<double>((ref) => 0.0);
final vpnProtocolProvider = StateProvider<String>((ref) => "Idle");

// Provider pour les actions
final vpnControllerProvider = Provider((ref) {
  final controller = VpnController(ref);
  controller.init(); // Commence à écouter le flux Rust
  return controller;
});

class VpnController {
  final Ref ref;
  VpnController(this.ref);

  void init() {
    // Listen to real-time events from Rust
    rust.registerStatusStream().listen((event) {
      ref.read(vpnStatusProvider.notifier).state = event.status;
      ref.read(vpnProtocolProvider.notifier).state = event.protocol;
      ref.read(vpnDownloadSpeedProvider.notifier).state = event.downloadSpeed;
      ref.read(vpnUploadSpeedProvider.notifier).state = event.uploadSpeed;
    }, onError: (e) {
      ref.read(vpnStatusProvider.notifier).state = "Stream Error";
    });
  }

  Future<void> connect(String nodeGroup) async {
    final settings = ref.read(settingsProvider);
    ref.read(vpnStatusProvider.notifier).state = "Contacting Hub...";

    try {
      // Real Rust Call for Connection Cascade
      await rust.startVpnConnection(
        protocolStr: settings.protocol,
        countryCode: "FR", // Default or current selected
      );
    } catch (e) {
      ref.read(vpnStatusProvider.notifier).state = "Connection Error: $e";
    }
  }

  Future<void> startSharing() async {
    try {
      await rust.startSharing();
    } catch (e) {
      // Sharing error handled by UI state
    }
  }

  Future<void> stopSharing() async {
    try {
      await rust.stopSharing();
    } catch (e) {
      // Stop sharing error
    }
  }

  Future<void> disconnect() async {
    try {
      await rust.stopVpnConnection();
      ref.read(vpnStatusProvider.notifier).state = "Disconnected";
      ref.read(vpnDownloadSpeedProvider.notifier).state = 0.0;
      ref.read(vpnUploadSpeedProvider.notifier).state = 0.0;
    } catch (e) {
      // Quiet fail
    }
  }
}
