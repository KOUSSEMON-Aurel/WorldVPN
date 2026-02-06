import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();

  void _handleLogin() {
    // TODO: Connect to backend
    context.go('/home');
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Stack(
        children: [
          // Background Glow
          Positioned.fill(
            child: Container(
              decoration: const BoxDecoration(
                gradient: RadialGradient(
                  center: Alignment.center,
                  radius: 0.8,
                  colors: [
                    Color(0x3300F2EA), // Cyan low opacity
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
                  // Logo Shield
                  Container(
                    width: 100,
                    height: 100,
                    decoration: BoxDecoration(
                      color: const Color(0xFF00F2EA).withOpacity(0.1),
                      borderRadius: BorderRadius.circular(24),
                      border: Border.all(color: const Color(0xFF00F2EA).withOpacity(0.3)),
                      boxShadow: [
                        BoxShadow(
                          color: const Color(0xFF00F2EA).withOpacity(0.2),
                          blurRadius: 30,
                          spreadRadius: 5,
                        ),
                      ],
                    ),
                    child: const Icon(LucideIcons.shield, size: 48, color: Color(0xFF00F2EA)),
                  ),
                  const SizedBox(height: 32),
                  
                  const Text(
                    "WorldVPN",
                    style: TextStyle(fontSize: 32, fontWeight: FontWeight.bold, letterSpacing: -1),
                  ),
                  Text(
                    "Decentralized Secure Perimeter",
                    style: TextStyle(color: Colors.white.withOpacity(0.5), fontSize: 12, fontFamily: 'monospace'),
                  ),
                  
                  const SizedBox(height: 48),
                  
                  // Username
                  TextField(
                    controller: _usernameController,
                    style: const TextStyle(fontFamily: 'monospace'),
                    decoration: InputDecoration(
                      labelText: "IDENTITY",
                      labelStyle: const TextStyle(fontSize: 10, letterSpacing: 2, fontWeight: FontWeight.bold),
                      filled: true,
                      fillColor: Colors.white.withOpacity(0.05),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                        borderSide: BorderSide(color: Colors.white.withOpacity(0.1)),
                      ),
                      enabledBorder: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                        borderSide: BorderSide(color: Colors.white.withOpacity(0.1)),
                      ),
                    ),
                  ),
                  
                  const SizedBox(height: 16),
                  
                  // Password
                  TextField(
                    controller: _passwordController,
                    obscureText: true,
                    decoration: InputDecoration(
                      labelText: "ACCESS KEY",
                      labelStyle: const TextStyle(fontSize: 10, letterSpacing: 2, fontWeight: FontWeight.bold),
                      filled: true,
                      fillColor: Colors.white.withOpacity(0.05),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                        borderSide: BorderSide(color: Colors.white.withOpacity(0.1)),
                      ),
                      enabledBorder: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                        borderSide: BorderSide(color: Colors.white.withOpacity(0.1)),
                      ),
                    ),
                  ),
                  
                  const SizedBox(height: 32),
                  
                  // Button
                  SizedBox(
                    width: double.infinity,
                    height: 56,
                    child: ElevatedButton(
                      onPressed: _handleLogin,
                      style: ElevatedButton.styleFrom(
                        backgroundColor: const Color(0xFF00F2EA),
                        foregroundColor: Colors.black,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                        elevation: 10,
                        shadowColor: const Color(0xFF00F2EA).withOpacity(0.5),
                      ),
                      child: const Text(
                        "INITIALIZE SESSION",
                        style: TextStyle(fontWeight: FontWeight.bold, letterSpacing: 1),
                      ),
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
