import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class GlassBottomNav extends StatelessWidget {
  final StatefulNavigationShell navigationShell;

  const GlassBottomNav({
    super.key,
    required this.navigationShell,
  });

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: const BorderRadius.only(
        topLeft: Radius.circular(20),
        topRight: Radius.circular(20),
      ),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
        child: Container(
          decoration: BoxDecoration(
            color: Colors.black.withOpacity(0.6),
            border: Border(
              top: BorderSide(
                color: Colors.white.withOpacity(0.1),
                width: 1,
              ),
            ),
          ),
          child: NavigationBar(
            backgroundColor: Colors.transparent,
            indicatorColor: Colors.cyan.withOpacity(0.2),
            selectedIndex: navigationShell.currentIndex,
            onDestinationSelected: (index) {
              navigationShell.goBranch(
                index,
                initialLocation: index == navigationShell.currentIndex,
              );
            },
            destinations: const [
              NavigationDestination(
                icon: Icon(Icons.dashboard_outlined, color: Colors.grey),
                selectedIcon: Icon(Icons.dashboard, color: Colors.cyanAccent),
                label: 'Dash',
              ),
              NavigationDestination(
                icon: Icon(Icons.map_outlined, color: Colors.grey),
                selectedIcon: Icon(Icons.map, color: Colors.cyanAccent),
                label: 'Map',
              ),
              NavigationDestination(
                icon: Icon(Icons.account_balance_wallet_outlined, color: Colors.grey),
                selectedIcon: Icon(Icons.account_balance_wallet, color: Colors.cyanAccent),
                label: 'Wallet',
              ),
              NavigationDestination(
                icon: Icon(Icons.settings_outlined, color: Colors.grey),
                selectedIcon: Icon(Icons.settings, color: Colors.cyanAccent),
                label: 'Settings',
              ),
            ],
          ),
        ),
      ),
    );
  }
}
