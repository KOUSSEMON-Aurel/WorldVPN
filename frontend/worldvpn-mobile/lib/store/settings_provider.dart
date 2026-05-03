import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

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
  SettingsNotifier() : super(SettingsState()) {
    _load();
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    state = SettingsState(
      protocol: prefs.getString('settings_protocol') ?? "WireGuard",
      killSwitch: prefs.getBool('settings_killSwitch') ?? true,
      splitTunneling: prefs.getBool('settings_splitTunneling') ?? false,
      sharingMode: prefs.getBool('settings_sharingMode') ?? false,
      language: prefs.getString('settings_language') ?? "English (US)",
    );
  }

  void setProtocol(String protocol) {
    state = state.copyWith(protocol: protocol);
    SharedPreferences.getInstance()
        .then((p) => p.setString('settings_protocol', protocol));
  }

  void toggleKillSwitch(bool value) {
    state = state.copyWith(killSwitch: value);
    SharedPreferences.getInstance()
        .then((p) => p.setBool('settings_killSwitch', value));
  }

  void toggleSplitTunneling(bool value) {
    state = state.copyWith(splitTunneling: value);
    SharedPreferences.getInstance()
        .then((p) => p.setBool('settings_splitTunneling', value));
  }

  void toggleSharingMode(bool value) {
    state = state.copyWith(sharingMode: value);
    SharedPreferences.getInstance()
        .then((p) => p.setBool('settings_sharingMode', value));
  }

  void setLanguage(String lang) {
    state = state.copyWith(language: lang);
    SharedPreferences.getInstance()
        .then((p) => p.setString('settings_language', lang));
  }
}

final settingsProvider =
    StateNotifierProvider<SettingsNotifier, SettingsState>((ref) {
  return SettingsNotifier();
});
