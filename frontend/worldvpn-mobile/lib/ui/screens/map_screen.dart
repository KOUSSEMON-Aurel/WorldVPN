import 'package:flutter/material.dart';
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';

class MapScreen extends StatelessWidget {
  const MapScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Stack(
        children: [
          FlutterMap(
            options: const MapOptions(
              initialCenter: LatLng(20.0, 0.0), // World View
              initialZoom: 2.5,
              minZoom: 2.0,
              maxZoom: 18.0,
              interactionOptions: InteractionOptions(
                 flags: InteractiveFlag.all & ~InteractiveFlag.rotate,
              ),
            ),
            children: [
              TileLayer(
                urlTemplate: 'https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png', // CartoDB Dark Matter
                userAgentPackageName: 'com.worldvpn.app',
                subdomains: const ['a', 'b', 'c', 'd'],
                retinaMode: RetinaMode.isHighDensity(context),
              ),
              MarkerLayer(
                markers: [
                  _buildNodeMarker(const LatLng(48.8566, 2.3522), "Paris", Colors.cyan),
                  _buildNodeMarker(const LatLng(40.7128, -74.0060), "New York", Colors.purple),
                  _buildNodeMarker(const LatLng(35.6762, 139.6503), "Tokyo", Colors.cyan),
                ],
              ),
            ],
          ),
          // Gradient Overlay Top
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            height: 100,
            child: Container(
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topCenter,
                  end: Alignment.bottomCenter,
                  colors: [
                    const Color(0xFF0A0F1C).withOpacity(0.9),
                    Colors.transparent,
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Marker _buildNodeMarker(LatLng point, String label, Color color) {
    return Marker(
      point: point,
      width: 40,
      height: 40,
      child: Column(
        children: [
          Icon(Icons.location_on, color: color, size: 30),
        ],
      ),
    );
  }
}
