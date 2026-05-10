import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import '../../store/wallet_provider.dart';

// Definir une classe locale pour l'instant car le code Rust n'a pas encore été régénéré en Dart
class LocalTransaction {
  final String id;
  final String txType;
  final double amount;
  final String date;
  final String description;

  const LocalTransaction({
    required this.id,
    required this.txType,
    required this.amount,
    required this.date,
    required this.description,
  });
}

class WalletScreen extends ConsumerStatefulWidget {
  const WalletScreen({super.key});

  @override
  ConsumerState<WalletScreen> createState() => _WalletScreenState();
}

class _WalletScreenState extends ConsumerState<WalletScreen> {
  List<LocalTransaction> _transactions = [];
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _fetchTransactions();
  }

  Future<void> _fetchTransactions() async {
    // Simulacre temporaire car getTransactions() n'est pas encore dans le bridge généré
    // Mais le backend Rust est prêt dès que le bridge sera régénéré.
    await Future.delayed(const Duration(milliseconds: 500));
    if (mounted) {
      setState(() {
        _transactions = [
          const LocalTransaction(
            id: "1",
            txType: "EARNED",
            amount: 10.5,
            date: "2026-05-01",
            description: "P2P Sharing Reward",
          ),
        ];
        _isLoading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final balance = ref.watch(walletBalanceProvider);

    return Scaffold(
      backgroundColor: const Color(0xFF0A0F1C),
      body: CustomScrollView(
        slivers: [
          _buildSliverAppBar(context, balance),
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.all(24.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    "Recent Activity",
                    style: TextStyle(
                      fontSize: 18,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(height: 16),
                  if (_isLoading)
                    const Center(child: CircularProgressIndicator())
                  else if (_transactions.isEmpty)
                    const Center(
                      child: Padding(
                        padding: EdgeInsets.symmetric(vertical: 40),
                        child: Text("No recent transactions",
                            style: TextStyle(color: Colors.grey)),
                      ),
                    )
                  else
                    ..._transactions.map((tx) => _buildTransactionItem(tx)),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTransactionItem(LocalTransaction tx) {
    final bool isEarned = tx.txType == "EARNED";
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.05),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Row(
        children: [
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: isEarned
                  ? Colors.green.withOpacity(0.2)
                  : Colors.red.withOpacity(0.2),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(
              isEarned ? LucideIcons.arrowDownLeft : LucideIcons.arrowUpRight,
              color: isEarned ? Colors.green : Colors.red,
              size: 20,
            ),
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(tx.description,
                    style: const TextStyle(fontWeight: FontWeight.bold)),
                Text(tx.date,
                    style: const TextStyle(color: Colors.grey, fontSize: 12)),
              ],
            ),
          ),
          Text(
            "${isEarned ? '+' : ''}${tx.amount} CR",
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: isEarned ? Colors.green : Colors.white,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSliverAppBar(BuildContext context, int balance) {
    return SliverToBoxAdapter(
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text("My Wallet",
                  style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold)),
              const SizedBox(height: 24),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(24),
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    colors: [
                      Theme.of(context).colorScheme.primary,
                      Theme.of(context).colorScheme.secondary
                    ],
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                  ),
                  borderRadius: BorderRadius.circular(24),
                  boxShadow: [
                    BoxShadow(
                      color: Theme.of(context)
                          .colorScheme
                          .primary
                          .withOpacity(0.3),
                      blurRadius: 20,
                      offset: const Offset(0, 10),
                    ),
                  ],
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text("Total Balance",
                        style: TextStyle(color: Colors.white70, fontSize: 14)),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        const Icon(LucideIcons.wallet,
                            color: Colors.white, size: 24),
                        const SizedBox(width: 12),
                        Text("$balance CR",
                            style: const TextStyle(
                                color: Colors.white,
                                fontSize: 32,
                                fontWeight: FontWeight.bold,
                                fontFamily: 'monospace')),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
