import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'ui/router.dart';
import 'rust_gen/frb_generated.dart';
import 'rust_gen/api/simple.dart' as rust;

void main() async {
  // Ensure Flutter binding is initialized before async calls
  WidgetsFlutterBinding.ensureInitialized();

  try {
    debugPrint("🚀 Initializing WorldVPN Rust Library...");
    await RustLib.init();
    debugPrint("✅ Rust Library initialized");

    // Configurer l'URL du backend de production
    await rust.setBackendUrl(url: "https://worldvpn-backend.onrender.com");
    debugPrint("🌐 Backend URL set to production");

    // Auto-login silencieux: génère ou restaure l'identité
    _silentLogin();
  } catch (e, stack) {
    debugPrint("❌ CRITICAL: Rust Initialization failed: $e");
    debugPrint(stack.toString());
  }

  runApp(const ProviderScope(child: WorldVpnApp()));
}

/// Génère une nouvelle identité (si inexistante) et s'authentifie silencieusement.
/// Non-bloquant — les erreurs sont loguées uniquement.
Future<void> _silentLogin() async {
  try {
    final prefs = await SharedPreferences.getInstance();
    List<int> privateKeyBytes;

    final keyBase64 = prefs.getString('identity_key');
    if (keyBase64 == null) {
      privateKeyBytes = await rust.generateIdentity();
      await prefs.setString('identity_key', base64Encode(privateKeyBytes));
      debugPrint("🔑 New identity generated");
    } else {
      privateKeyBytes = base64Decode(keyBase64);
      debugPrint("🔑 Identity restored from storage");
    }

    await rust.loginAnonymously(privateKeyBytes: privateKeyBytes);
    debugPrint("✅ Anonymous login successful");
  } catch (e) {
    // Non-fatal: the user can still browse the app; connection will fail gracefully
    debugPrint("⚠️ Silent login failed (backend unreachable?): $e");
  }
}

class WorldVpnApp extends StatelessWidget {
  const WorldVpnApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: 'WorldVPN',
      themeMode: ThemeMode.dark,
      theme: ThemeData(
        brightness: Brightness.dark,
        useMaterial3: true,
        scaffoldBackgroundColor: const Color(0xFF0A0F1C), // Deep dark blue
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF00F2EA), // Cyan Neon
          secondary: Color(0xFF7000FF), // Purple Neon
          surface: Color(0xFF131B2E), // Lighter blue for cards
          onSurface: Colors.white,
        ),
        textTheme:
            GoogleFonts.outfitTextTheme(Theme.of(context).textTheme).apply(
          bodyColor: Colors.white,
          displayColor: Colors.white,
        ),
      ),
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}
