import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../../rust_gen/api/simple.dart' as rust;

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  bool _isLoading = false;

  Future<void> _handleConnect() async {
    setState(() => _isLoading = true);

    try {
      final prefs = await SharedPreferences.getInstance();
      String? keyBase64 = prefs.getString('identity_key');
      List<int> privateKeyBytes;

      if (keyBase64 == null) {
        // Generate new identity
        privateKeyBytes = await rust.generateIdentity();
        prefs.setString('identity_key', base64Encode(privateKeyBytes));
      } else {
        privateKeyBytes = base64Decode(keyBase64);
      }

      // Anonymous Auth Flow via Rust
      await rust.loginAnonymously(privateKeyBytes: privateKeyBytes);

      if (mounted) {
        context.go('/home');
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text("AUTHENTICATION FAILED: $e"),
            backgroundColor: Colors.red,
          ),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  void _handleResetIdentity() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove('identity_key');
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text("Identity Key Reset."),
          backgroundColor: Colors.green,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Stack(
        children: [
          Positioned.fill(
            child: Container(
              decoration: const BoxDecoration(
                gradient: RadialGradient(
                  center: Alignment.center,
                  radius: 0.8,
                  colors: [
                    Color(0x3300F2EA),
                    Color(0xFF0A0F1C),
                  ],
                ),
              ),
            ),
          ),
          Center(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(32),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Container(
                    width: 100,
                    height: 100,
                    decoration: BoxDecoration(
                      color: const Color(0xFF00F2EA).withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(24),
                      border: Border.all(
                          color: const Color(0xFF00F2EA).withValues(alpha: 0.3)),
                      boxShadow: [
                        BoxShadow(
                          color: const Color(0xFF00F2EA).withValues(alpha: 0.2),
                          blurRadius: 30,
                          spreadRadius: 5,
                        ),
                      ],
                    ),
                    child: const Icon(LucideIcons.shield,
                        size: 48, color: Color(0xFF00F2EA)),
                  ),
                  const SizedBox(height: 32),
                  const Text(
                    "WorldVPN",
                    style: TextStyle(
                        fontSize: 32,
                        fontWeight: FontWeight.bold,
                        letterSpacing: -1),
                  ),
                  Text(
                    "Decentralized Anonymous Network",
                    style: TextStyle(
                        color: Colors.white.withValues(alpha: 0.5),
                        fontSize: 12,
                        fontFamily: 'monospace'),
                  ),
                  const SizedBox(height: 64),
                  SizedBox(
                    width: double.infinity,
                    height: 56,
                    child: ElevatedButton(
                      onPressed: _isLoading ? null : _handleConnect,
                      style: ElevatedButton.styleFrom(
                        backgroundColor: const Color(0xFF00F2EA),
                        foregroundColor: Colors.black,
                        shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(12)),
                        elevation: 10,
                        shadowColor:
                            const Color(0xFF00F2EA).withValues(alpha: 0.5),
                      ),
                      child: _isLoading
                          ? const SizedBox(
                              width: 24,
                              height: 24,
                              child: CircularProgressIndicator(
                                  color: Colors.black, strokeWidth: 2))
                          : const Text(
                              "CONNECT ANONYMOUSLY",
                              style: TextStyle(
                                  fontWeight: FontWeight.bold,
                                  letterSpacing: 1),
                            ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextButton(
                    onPressed: _isLoading ? null : _handleResetIdentity,
                    child: const Text(
                      "RESET IDENTITY KEY",
                      style: TextStyle(
                          color: Colors.white70,
                          fontWeight: FontWeight.bold,
                          letterSpacing: 1,
                          fontSize: 12),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
