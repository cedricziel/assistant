/// What an `Image.url` resolves to under the surface URL policy.
///
/// Mirrors the server's `check_image_url` in `crates/a2ui/src/safety.rs`:
/// `https:` from any host, or a same-origin `/api/attachments/{id}`. The
/// server enforces `[ui] image_hosts` and attachment ownership; the client
/// repeats the scheme and shape checks on the resolved value, which a bound
/// data-model path can change after the server validated the surface.
sealed class ImageTarget {
  const ImageTarget();
}

/// An `https:` image on any host.
final class NetworkImageTarget extends ImageTarget {
  const NetworkImageTarget(this.url);
  final Uri url;
}

/// A same-origin attachment, loaded with the app's credentials.
final class AttachmentImageTarget extends ImageTarget {
  const AttachmentImageTarget(this.attachmentId);
  final String attachmentId;
}

/// The URL violates the policy; render a placeholder.
final class BlockedImageTarget extends ImageTarget {
  const BlockedImageTarget();
}

abstract final class UrlPolicy {
  static const _attachmentPrefix = '/api/attachments/';
  static final _uuid = RegExp(
    r'^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
  );

  /// Classifies a resolved `Image.url` value.
  static ImageTarget image(String url) {
    if (url.startsWith(_attachmentPrefix)) {
      final id = url.substring(_attachmentPrefix.length);
      return _uuid.hasMatch(id)
          ? AttachmentImageTarget(id)
          : const BlockedImageTarget();
    }
    final uri = Uri.tryParse(url);
    if (uri == null || uri.scheme != 'https' || uri.host.isEmpty) {
      return const BlockedImageTarget();
    }
    return NetworkImageTarget(uri);
  }
}
