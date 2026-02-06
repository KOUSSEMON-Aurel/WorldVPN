import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:go_router/go_router.dart';

class ProfileScreen extends StatelessWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final primaryColor = Theme.of(context).colorScheme.primary;

    return Scaffold(
      backgroundColor: const Color(0xFF0A0F1C),
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(LucideIcons.arrowLeft, color: Colors.white),
          onPressed: () => context.pop(),
        ),
        title: const Text("My Profile", style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
      ),
      body: ListView(
        padding: const EdgeInsets.all(24),
        children: [
          // Avatar & Name
          Center(
            child: Column(
              children: [
                Container(
                  width: 100,
                  height: 100,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    gradient: LinearGradient(colors: [primaryColor, Colors.purple]),
                    border: Border.all(color: Colors.white.withOpacity(0.2), width: 4),
                  ),
                  child: const Icon(LucideIcons.user, size: 50, color: Colors.white),
                ),
                const SizedBox(height: 16),
                const Text("Aurel K.", style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold)),
                const Text("aurel@worldvpn.com", style: TextStyle(color: Colors.grey)),
              ],
            ),
          ),
          const SizedBox(height: 40),

          // Subscription Card
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: primaryColor.withOpacity(0.1),
              borderRadius: BorderRadius.circular(24),
              border: Border.all(color: primaryColor.withOpacity(0.3)),
            ),
            child: Column(
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text("PREMIUM PLAN", style: TextStyle(fontWeight: FontWeight.bold, letterSpacing: 1.2)),
                    Icon(LucideIcons.crown, color: primaryColor, size: 20),
                  ],
                ),
                const SizedBox(height: 12),
                const Text("Your subscription is active and will renew on March 2, 2026.", 
                  style: TextStyle(color: Colors.white70, fontSize: 13)
                ),
                const SizedBox(height: 20),
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton(
                    onPressed: () {},
                    style: ElevatedButton.styleFrom(
                      backgroundColor: primaryColor,
                      foregroundColor: Colors.black,
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                    ),
                    child: const Text("MANAGE SUBSCRIPTION", style: TextStyle(fontWeight: FontWeight.bold, fontSize: 12)),
                  ),
                )
              ],
            ),
          ),
          const SizedBox(height: 32),

          // Details Section
          _buildInfoRow(LucideIcons.mail, "Email", "aurel@worldvpn.com"),
          _buildInfoRow(LucideIcons.calendar, "Joined", "Jan 2024"),
          _buildInfoRow(LucideIcons.globe, "Default Region", "Europe (Paris)"),
          
          const SizedBox(height: 40),
          
          // Logout
          TextButton.icon(
            onPressed: () {},
            icon: const Icon(LucideIcons.logOut, color: Colors.redAccent, size: 20),
            label: const Text("Log Out", style: TextStyle(color: Colors.redAccent, fontWeight: FontWeight.bold)),
          ),
        ],
      ),
    );
  }

  Widget _buildInfoRow(IconData icon, String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 16),
      child: Row(
        children: [
          Icon(icon, size: 20, color: Colors.grey),
          const SizedBox(width: 16),
          Text(label, style: const TextStyle(color: Colors.grey)),
          const Spacer(),
          Text(value, style: const TextStyle(fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }
}
