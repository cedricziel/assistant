/// No-op tray service for web and unsupported platforms.
class TrayService {
  static final TrayService _instance = TrayService._();
  factory TrayService() => _instance;
  TrayService._();

  Future<void> init() async {}
  void dispose() {}
}
