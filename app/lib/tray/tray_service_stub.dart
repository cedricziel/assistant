import 'package:flutter_riverpod/flutter_riverpod.dart';

/// No-op tray service for web and unsupported platforms.
class TrayService {
  static final TrayService _instance = TrayService._();
  factory TrayService() => _instance;
  TrayService._();

  Future<void> initIcon() async {}
  Future<void> initMenu(WidgetRef ref) async {}
  void dispose() {}
}
