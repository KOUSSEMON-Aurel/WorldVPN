import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../store/settings_provider.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final settingsNotifier = ref.read(settingsProvider.notifier);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            return ListView(
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
              children: [
                const Text(
                  "Settings",
                  style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold, letterSpacing: -0.5),
                ),
                const SizedBox(height: 32),
                
                // Profile Section
                _buildSectionHeader("Account"),
                _buildSettingTile(
                  context,
                  icon: LucideIcons.user,
                  title: "Aurel K.",
                  subtitle: "Premium Plan • View Profile",
                  trailing: const Icon(LucideIcons.chevronRight, size: 16, color: Colors.grey),
                  onTap: () => context.push('/settings/profile'),
                ),
                const SizedBox(height: 24),

                // VPN Settings
                _buildSectionHeader("VPN Configuration"),
                _buildSettingTile(
                  context,
                  icon: LucideIcons.shield,
                  title: "Default Protocol",
                  subtitle: settings.protocol,
                  trailing: const Icon(LucideIcons.chevronRight, size: 16, color: Colors.grey),
                  onTap: () => _showProtocolDialog(context, ref),
                ),
                _buildSwitchTile(
                  "Kill Switch",
                  "Auto-block internet on drop",
                  settings.killSwitch,
                  (val) => settingsNotifier.toggleKillSwitch(val),
                ),
                _buildSwitchTile(
                  "Split Tunneling",
                  "Bypass specific apps",
                  settings.splitTunneling,
                  (val) => settingsNotifier.toggleSplitTunneling(val),
                ),
                const SizedBox(height: 24),

                // General Settings
                _buildSectionHeader("General"),
                _buildSettingTile(
                  context,
                  icon: LucideIcons.bell,
                  title: "Notifications",
                  onTap: () {},
                ),
                 _buildSettingTile(
                  context,
                  icon: LucideIcons.languages,
                  title: "Language",
                  subtitle: settings.language,
                  onTap: () {},
                ),
                const SizedBox(height: 48),

                // Support links
                _buildSectionHeader("Support"),
                _buildSettingTile(
                  context,
                  icon: LucideIcons.info,
                  title: "Version",
                  subtitle: "1.0.0-alpha.1",
                  onTap: () {},
                ),
                const SizedBox(height: 20),
              ],
            );
          }
        ),
      ),
    );
  }

  void _showProtocolDialog(BuildContext context, WidgetRef ref) {
    final current = ref.read(settingsProvider).protocol;
    final protocols = [
      "WireGuard", 
      "OpenVPN", 
      "Shadowsocks", 
      "IKEv2", 
      "Hysteria 2", 
      "Trojan", 
      "VLESS"
    ];
    final primaryColor = Theme.of(context).colorScheme.primary;

    showDialog(
      context: context,
      builder: (context) => BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 5, sigmaY: 5),
        child: Dialog(
          backgroundColor: const Color(0xFF131B2E),
          insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 40),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(24),
            side: BorderSide(color: primaryColor.withOpacity(0.3), width: 1),
          ),
          child: Padding(
            padding: const EdgeInsets.all(24.0),
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      Icon(LucideIcons.shield, color: primaryColor, size: 20),
                      const SizedBox(width: 12),
                      const Text("SELECT PROTOCOL", 
                        style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, letterSpacing: 1.5)
                      ),
                    ],
                  ),
                  const SizedBox(height: 20),
                  ...protocols.map((p) {
                    final isSelected = current == p;
                    return InkWell(
                      onTap: () {
                        ref.read(settingsProvider.notifier).setProtocol(p);
                        Navigator.pop(context);
                      },
                      borderRadius: BorderRadius.circular(16),
                      child: Container(
                        margin: const EdgeInsets.symmetric(vertical: 4),
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                        decoration: BoxDecoration(
                          color: isSelected ? primaryColor.withOpacity(0.1) : Colors.transparent,
                          borderRadius: BorderRadius.circular(16),
                          border: Border.all(
                            color: isSelected ? primaryColor.withOpacity(0.5) : Colors.transparent,
                          ),
                        ),
                        child: Row(
                          children: [
                            Text(p, style: TextStyle(
                              color: isSelected ? primaryColor : Colors.white,
                              fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                            )),
                            const Spacer(),
                            if (isSelected) Icon(LucideIcons.checkCircle, color: primaryColor, size: 18),
                          ],
                        ),
                      ),
                    );
                  }),
                  const SizedBox(height: 16),
                  TextButton(
                    onPressed: () => Navigator.pop(context),
                    child: const Text("CLOSE", style: TextStyle(color: Colors.grey)),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildSectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12, left: 4),
      child: Text(
        title.toUpperCase(),
        style: const TextStyle(
          color: Colors.grey,
          fontSize: 11,
          fontWeight: FontWeight.bold,
          letterSpacing: 1.5,
        ),
      ),
    );
  }

  Widget _buildSettingTile(BuildContext context, {
    required IconData icon,
    required String title,
    String? subtitle,
    Widget? trailing,
    required VoidCallback onTap,
  }) {
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.03),
        borderRadius: BorderRadius.circular(18),
      ),
      child: ListTile(
        onTap: onTap,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        leading: Container(
          padding: const EdgeInsets.all(10),
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.primary.withOpacity(0.05),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Icon(icon, color: Theme.of(context).colorScheme.primary, size: 18),
        ),
        title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
        subtitle: subtitle != null ? Text(subtitle, style: const TextStyle(color: Colors.grey, fontSize: 11)) : null,
        trailing: trailing,
      ),
    );
  }

  Widget _buildSwitchTile(String title, String subtitle, bool value, ValueChanged<bool> onChanged) {
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.03),
        borderRadius: BorderRadius.circular(18),
      ),
      child: SwitchListTile(
        value: value,
        onChanged: onChanged,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 0),
        title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
        subtitle: Text(subtitle, style: const TextStyle(color: Colors.grey, fontSize: 11)),
        activeColor: const Color(0xFF00F2EA),
      ),
    );
  }
}
