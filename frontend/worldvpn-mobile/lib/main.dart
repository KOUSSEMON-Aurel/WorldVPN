import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'ui/router.dart';
import 'rust_gen/frb_generated.dart';

void main() async {
  // Ensure Flutter binding is initialized before async calls
  WidgetsFlutterBinding.ensureInitialized();
  
  try {
    print("Initializing Rust...");
    // Timeout de 3 secondes max pour l'init Rust
    await RustLib.init().timeout(const Duration(seconds: 3));
    print("Rust initialized successfully.");
  } catch (e) {
    print("RUST INIT FAILED: $e");
    // On continue quand même pour afficher l'UI
  }
  
  runApp(const ProviderScope(child: WorldVpnApp()));
}

class WorldVpnApp extends StatelessWidget {
  const WorldVpnApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: 'WorldVPN',
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF0A0F1C), // Deep dark blue
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF00F2EA),   // Cyan Neon
          secondary: Color(0xFF7000FF), // Purple Neon
          surface: Color(0xFF131B2E),   // Lighter blue for cards
          background: Color(0xFF0A0F1C),
          onSurface: Colors.white,
        ),
        textTheme: GoogleFonts.outfitTextTheme(Theme.of(context).textTheme).apply(
          bodyColor: Colors.white,
          displayColor: Colors.white,
        ),
        useMaterial3: true,
      ),
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}
