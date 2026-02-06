import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../rust_gen/api/simple.dart';
import 'settings_provider.dart';

final vpnStatusProvider = StateProvider<String>((ref) => "Disconnected");

// Provider pour les actions
final vpnControllerProvider = Provider((ref) => VpnController(ref));

class VpnController {
  final Ref ref; // Changé WidgetRef -> Ref pour être utilisé dans les providers
  VpnController(this.ref);

  Future<void> connect(String node) async {
    final settings = ref.read(settingsProvider);
    ref.read(vpnStatusProvider.notifier).state = "Connecting...";
    try {
      // Real Rust Call
      final message = await greet(name: "Aurel (Mobile) over ${settings.protocol}");
      print("RUST RESPONSE: $message");
      
      await Future.delayed(const Duration(seconds: 1));
      ref.read(vpnStatusProvider.notifier).state = "Connected";
    } catch (e) {
      ref.read(vpnStatusProvider.notifier).state = "Error: $e";
    }
  }

  Future<void> disconnect() async {
    // TODO: Appeler Rust Bridge ici
    // await RustLib.instance.api.stopVpn();
    
    ref.read(vpnStatusProvider.notifier).state = "Disconnected";
  }
}
