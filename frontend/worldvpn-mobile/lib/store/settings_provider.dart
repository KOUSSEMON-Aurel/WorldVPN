import 'package:flutter_riverpod/flutter_riverpod.dart';

class SettingsState {
  final String protocol;
  final bool killSwitch;
  final bool splitTunneling;
  final bool sharingMode;
  final String language;

  SettingsState({
    this.protocol = "WireGuard",
    this.killSwitch = true,
    this.splitTunneling = false,
    this.sharingMode = false,
    this.language = "English (US)",
  });

  SettingsState copyWith({
    String? protocol,
    bool? killSwitch,
    bool? splitTunneling,
    bool? sharingMode,
    String? language,
  }) {
    return SettingsState(
      protocol: protocol ?? this.protocol,
      killSwitch: killSwitch ?? this.killSwitch,
      splitTunneling: splitTunneling ?? this.splitTunneling,
      sharingMode: sharingMode ?? this.sharingMode,
      language: language ?? this.language,
    );
  }
}

class SettingsNotifier extends StateNotifier<SettingsState> {
  SettingsNotifier() : super(SettingsState());

  void setProtocol(String protocol) => state = state.copyWith(protocol: protocol);
  void toggleKillSwitch(bool value) => state = state.copyWith(killSwitch: value);
  void toggleSplitTunneling(bool value) => state = state.copyWith(splitTunneling: value);
  void toggleSharingMode(bool value) => state = state.copyWith(sharingMode: value);
  void setLanguage(String lang) => state = state.copyWith(language: lang);
}

final settingsProvider = StateNotifierProvider<SettingsNotifier, SettingsState>((ref) {
  return SettingsNotifier();
});
