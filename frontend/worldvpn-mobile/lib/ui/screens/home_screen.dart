import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../store/vpn_provider.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> with SingleTickerProviderStateMixin {
  String nodeGroup = "COMMUNITY";
  late AnimationController _pulseController;
  
  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 2),
    );
  }

  @override
  void dispose() {
    _pulseController.dispose();
    super.dispose();
  }
  
  void _toggleVpn() {
    final status = ref.read(vpnStatusProvider);
    final isConnected = status == "Connected";
    
    if (isConnected) {
        ref.read(vpnControllerProvider).disconnect();
        _pulseController.stop();
        _pulseController.reset();
    } else {
        ref.read(vpnControllerProvider).connect(nodeGroup);
        _pulseController.repeat(reverse: true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final status = ref.watch(vpnStatusProvider);
    final isConnected = status == "Connected";
    final primaryColor = Theme.of(context).colorScheme.primary;
    
    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            return Column(
              children: [
                // Header (Responsive height)
                _buildHeader(context),
                
                // Map & Action Area
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                    child: Container(
                      decoration: BoxDecoration(
                        color: const Color(0xFF131B2E),
                        borderRadius: BorderRadius.circular(32),
                        border: Border.all(color: Colors.white.withOpacity(0.05)),
                      ),
                      child: Stack(
                        alignment: Alignment.center,
                        children: [
                          // Background Map
                          _buildMapBackground(),
                          
                          // central Toggle button (Responsive size)
                          _buildPowerButton(isConnected, primaryColor, constraints.maxHeight),
                          
                          // Top Selector
                          Positioned(
                            top: 20,
                            child: _buildNodeSelector(),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                
                // Bottom Info Panel
                _buildInfoPanel(context, isConnected),
              ],
            );
          }
        ),
      ),
    );
  }

  Widget _buildHeader(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Row(
            children: [
              Icon(LucideIcons.shield, color: Color(0xFF00F2EA), size: 18),
              SizedBox(width: 8),
              Text("WorldVPN", style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
            ],
          ),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
            decoration: BoxDecoration(
              color: Colors.white.withOpacity(0.05),
              borderRadius: BorderRadius.circular(20),
            ),
            child: const Row(
              children: [
                Icon(LucideIcons.wallet, size: 12, color: Color(0xFF7000FF)),
                SizedBox(width: 6),
                Text("1,250 CR", style: TextStyle(fontFamily: 'monospace', fontWeight: FontWeight.bold, fontSize: 11)),
              ],
            ),
          )
        ],
      ),
    );
  }

  Widget _buildMapBackground() {
    return Opacity(
      opacity: 0.15,
      child: Image.network(
        "https://upload.wikimedia.org/wikipedia/commons/thumb/e/ec/World_map_blank_without_borders.svg/2000px-World_map_blank_without_borders.svg.png", 
        fit: BoxFit.contain,
        color: Colors.white,
      ),
    );
  }

  Widget _buildPowerButton(bool isConnected, Color primaryColor, double screenHeight) {
    // scale button based on screen height
    final buttonSize = screenHeight < 600 ? 120.0 : 160.0;
    
    return GestureDetector(
      onTap: _toggleVpn,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 300),
        width: buttonSize,
        height: buttonSize,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: const Color(0xFF0A0F1C),
          border: Border.all(
            color: isConnected ? primaryColor : Colors.white.withOpacity(0.1),
            width: 2,
          ),
          boxShadow: isConnected ? [
            BoxShadow(color: primaryColor.withOpacity(0.2), blurRadius: 40, spreadRadius: 5),
          ] : [],
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(LucideIcons.power, 
              size: buttonSize * 0.25, 
              color: isConnected ? primaryColor : Colors.white.withOpacity(0.3)
            ),
            const SizedBox(height: 8),
            Text(
              isConnected ? "CONNECTED" : "CONNECT",
              style: TextStyle(
                color: isConnected ? primaryColor : Colors.white.withOpacity(0.5),
                letterSpacing: 1.2,
                fontSize: buttonSize * 0.07,
                fontWeight: FontWeight.bold
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildNodeSelector() {
    return Container(
      padding: const EdgeInsets.all(4),
      decoration: BoxDecoration(
        color: Colors.black.withOpacity(0.5),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: Colors.white.withOpacity(0.1)),
      ),
      child: Row(
        children: [
          _buildToggleOption("Community", nodeGroup == "COMMUNITY"),
          _buildToggleOption("Public Gate", nodeGroup == "PUBLIC"),
        ],
      ),
    );
  }

  Widget _buildToggleOption(String label, bool isSelected) {
    return GestureDetector(
      onTap: () {
        setState(() {
          nodeGroup = label == "Community" ? "COMMUNITY" : "PUBLIC";
        });
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        decoration: BoxDecoration(
          color: isSelected ? Colors.white.withOpacity(0.1) : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: isSelected ? Colors.white : Colors.white54,
            fontSize: 12,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
          ),
        ),
      ),
    );
  }

  Widget _buildInfoPanel(BuildContext context, bool isConnected) {
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: [
          _buildStat("DOWNLOAD", "12.4 Mbps", LucideIcons.arrowDown),
          _buildStat("UPLOAD", "2.1 Mbps", LucideIcons.arrowUp),
          _buildStat("PING", "24 ms", LucideIcons.activity),
        ],
      ),
    );
  }

  Widget _buildStat(String label, String value, IconData icon) {
    return Column(
      children: [
        Row(
          children: [
            Icon(icon, size: 10, color: Colors.grey),
            const SizedBox(width: 4),
            Text(label, style: const TextStyle(color: Colors.grey, fontSize: 10, letterSpacing: 1)),
          ],
        ),
        const SizedBox(height: 4),
        Text(value, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13, fontFamily: 'monospace')),
      ],
    );
  }
}
