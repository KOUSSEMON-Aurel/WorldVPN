import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../rust_gen/api/simple.dart' as rust;

final walletBalanceProvider =
    StateNotifierProvider<WalletBalanceNotifier, int>((ref) {
  return WalletBalanceNotifier();
});

class WalletBalanceNotifier extends StateNotifier<int> {
  WalletBalanceNotifier() : super(0) {
    refreshBalance();
  }

  Future<void> refreshBalance() async {
    try {
      final bal = await rust.getWalletBalance();
      state = bal;
    } catch (e) {
      // Keep previous state on error, or handle properly
    }
  }
}
