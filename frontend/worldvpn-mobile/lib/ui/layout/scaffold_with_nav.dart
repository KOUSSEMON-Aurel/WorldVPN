import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import '../widgets/glass_bottom_nav.dart';

class ScaffoldWithNavBar extends StatelessWidget {
  const ScaffoldWithNavBar({
    required this.navigationShell,
    Key? key,
  }) : super(key: key ?? const ValueKey<String>('ScaffoldWithNavBar'));

  final StatefulNavigationShell navigationShell;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      extendBody: true, // Important pour l'effet de transparence derrière la nav bar
      body: Container(
        decoration: const BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [
              Color(0xFF0F172A), // Slate 900
              Color(0xFF020617), // Slate 950
            ],
          ),
        ),
        child: SafeArea(
          bottom: false, // Laisser le contenu aller sous la nav bar
          child: navigationShell,
        ),
      ),
      bottomNavigationBar: GlassBottomNav(navigationShell: navigationShell),
    );
  }
}
