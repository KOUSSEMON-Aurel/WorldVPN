import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../store/vpn_provider.dart';
import '../../store/settings_provider.dart';
import '../../store/wallet_provider.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen>
    with SingleTickerProviderStateMixin {
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

  @override
  Widget build(BuildContext context) {
    final status = ref.watch(vpnStatusProvider);
    final isConnected = status == "Connected";
    final isConnecting = status.contains("...") || status.contains("…");
    final downloadSpeed = ref.watch(vpnDownloadSpeedProvider);
    final uploadSpeed = ref.watch(vpnUploadSpeedProvider);

    final primaryColor = Theme.of(context).colorScheme.primary;

    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(builder: (context, constraints) {
          return Column(
            children: [
              // Header (Responsive height)
              _buildHeader(context),

              // Map & Action Area
              Expanded(
                child: Padding(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  child: Container(
                    decoration: BoxDecoration(
                      color: const Color(0xFF131B2E),
                      borderRadius: BorderRadius.circular(32),
                      border: Border.all(
                          color: Colors.white.withValues(alpha: 0.05)),
                    ),
                    child: Stack(
                      alignment: Alignment.center,
                      children: [
                        // Background Map
                        _buildMapBackground(),

                        // central Toggle button (Responsive size)
                        _buildPowerButton(isConnected, isConnecting,
                            primaryColor, constraints.maxHeight),

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
              _buildInfoPanel(context, isConnected, downloadSpeed, uploadSpeed),
            ],
          );
        }),
      ),
    );
  }

  Widget _buildHeader(BuildContext context) {
    final sharingMode =
        ref.watch(settingsProvider.select((s) => s.sharingMode));

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Row(
            children: [
              Icon(LucideIcons.shield, color: Color(0xFF00F2EA), size: 18),
              SizedBox(width: 8),
              Text("WorldVPN",
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
            ],
          ),
          Row(
            children: [
              // Sharing Toggle
              GestureDetector(
                onTap: () {
                  ref
                      .read(settingsProvider.notifier)
                      .toggleSharingMode(!sharingMode);
                  if (!sharingMode) {
                    ref.read(vpnControllerProvider).startSharing();
                  } else {
                    ref.read(vpnControllerProvider).stopSharing();
                  }
                },
                child: Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                  decoration: BoxDecoration(
                    color: sharingMode
                        ? const Color(0xFF00F2EA).withValues(alpha: 0.1)
                        : Colors.white.withValues(alpha: 0.05),
                    borderRadius: BorderRadius.circular(20),
                    border: Border.all(
                        color: sharingMode
                            ? const Color(0xFF00F2EA).withValues(alpha: 0.3)
                            : Colors.transparent),
                  ),
                  child: Row(
                    children: [
                      Icon(LucideIcons.share2,
                          size: 12,
                          color: sharingMode
                              ? const Color(0xFF00F2EA)
                              : Colors.white54),
                      const SizedBox(width: 6),
                      Text(sharingMode ? "NODE ACTIVE" : "SHARING OFF",
                          style: TextStyle(
                              color: sharingMode
                                  ? const Color(0xFF00F2EA)
                                  : Colors.white54,
                              fontWeight: FontWeight.bold,
                              fontSize: 9)),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                decoration: BoxDecoration(
                  color: Colors.white.withValues(alpha: 0.05),
                  borderRadius: BorderRadius.circular(20),
                ),
                child: Row(
                  children: [
                    const Icon(LucideIcons.wallet,
                        size: 12, color: Color(0xFF7000FF)),
                    const SizedBox(width: 6),
                    Text("${ref.watch(walletBalanceProvider)} CR",
                        style: const TextStyle(
                            fontFamily: 'monospace',
                            fontWeight: FontWeight.bold,
                            fontSize: 11)),
                  ],
                ),
              ),
            ],
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

  Widget _buildPowerButton(bool isConnected, bool isConnecting,
      Color primaryColor, double screenHeight) {
    // scale button based on screen height
    final buttonSize = screenHeight < 600 ? 120.0 : 160.0;

    if (isConnected && !_pulseController.isAnimating) {
      _pulseController.repeat(reverse: true);
    } else if (!isConnected && _pulseController.isAnimating) {
      _pulseController.stop();
    }

    return GestureDetector(
      onTap: isConnecting
          ? null
          : () {
              if (isConnected) {
                ref.read(vpnControllerProvider).disconnect();
              } else {
                ref.read(vpnControllerProvider).connect(nodeGroup);
              }
            },
      child: AnimatedBuilder(
        animation: _pulseController,
        builder: (context, child) {
          final pulseValue = isConnected ? _pulseController.value : 0.0;
          return Container(
            width: buttonSize,
            height: buttonSize,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              gradient: RadialGradient(
                colors: [
                  (isConnecting
                          ? Colors.amber
                          : (isConnected ? primaryColor : Colors.white))
                      .withValues(alpha: 0.15 + (pulseValue * 0.1)),
                  (isConnecting
                          ? Colors.amber
                          : (isConnected ? primaryColor : Colors.white))
                      .withValues(alpha: 0.05),
                ],
              ),
              border: Border.all(
                color: isConnecting
                    ? Colors.amber
                    : (isConnected
                        ? primaryColor
                        : Colors.white.withValues(alpha: 0.1)),
                width: 2 + (pulseValue * 2),
              ),
              boxShadow: (isConnected || isConnecting)
                  ? [
                      BoxShadow(
                        color: (isConnecting ? Colors.amber : primaryColor)
                            .withValues(alpha: 0.2 + (pulseValue * 0.1)),
                        blurRadius: 30 + (pulseValue * 20),
                        spreadRadius: 2 + (pulseValue * 5),
                      ),
                    ]
                  : [],
            ),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(isConnecting ? LucideIcons.refreshCw : LucideIcons.power,
                    size: buttonSize * 0.25,
                    color: isConnecting
                        ? Colors.amber
                        : (isConnected
                            ? primaryColor
                            : Colors.white.withValues(alpha: 0.3))),
                const SizedBox(height: 8),
                Text(
                  isConnecting
                      ? "BUSY"
                      : (isConnected ? "CONNECTED" : "CONNECT"),
                  style: TextStyle(
                      color: isConnecting
                          ? Colors.amber
                          : (isConnected
                              ? primaryColor
                              : Colors.white.withValues(alpha: 0.5)),
                      letterSpacing: 1.5,
                      fontSize: buttonSize * 0.08,
                      fontWeight: FontWeight.bold),
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildNodeSelector() {
    return Container(
      padding: const EdgeInsets.all(4),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
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
          color: isSelected
              ? Colors.white.withValues(alpha: 0.1)
              : Colors.transparent,
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

  Widget _buildInfoPanel(
      BuildContext context, bool isConnected, double download, double upload) {
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: [
          _buildStat("DOWNLOAD", "${download.toStringAsFixed(1)} Mbps",
              LucideIcons.arrowDown, isConnected),
          _buildStat("UPLOAD", "${upload.toStringAsFixed(1)} Mbps",
              LucideIcons.arrowUp, isConnected),
          _buildStat("PING", "24 ms", LucideIcons.activity, isConnected),
        ],
      ),
    );
  }

  Widget _buildStat(String label, String value, IconData icon, bool isActive) {
    return Column(
      children: [
        Row(
          children: [
            Icon(icon,
                size: 10,
                color: isActive ? const Color(0xFF00F2EA) : Colors.grey),
            const SizedBox(width: 4),
            Text(label,
                style: TextStyle(
                    color: isActive ? Colors.white70 : Colors.grey,
                    fontSize: 10,
                    letterSpacing: 1)),
          ],
        ),
        const SizedBox(height: 4),
        Text(value,
            style: TextStyle(
                color: isActive ? Colors.white : Colors.white38,
                fontWeight: FontWeight.bold,
                fontSize: 13,
                fontFamily: 'monospace')),
      ],
    );
  }
}
