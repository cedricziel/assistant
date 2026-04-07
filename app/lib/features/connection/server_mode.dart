/// Represents how the Flutter app connects to the assistant backend.
enum ServerMode {
  /// Use the Rust binary bundled inside the macOS app bundle.
  /// The binary is started automatically as a child process.
  embedded,

  /// Connect to a user-specified remote (or manually started local) server.
  remote,
}
