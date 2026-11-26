import 'dart:typed_data';

import 'package:a2ui_flutter/a2ui_flutter.dart';
// The stock catalog is built on framework Material; the app's
// LegacyDesignBridge publishes the app theme to it.
// ignore: deprecated_member_use
import 'package:flutter/material.dart';

import 'attachment_image_loader.dart';
import 'url_policy.dart';

/// The assistant catalog's `Image`: the stock sizing variants, with the URL
/// policy applied to the resolved `url` and attachments loaded through
/// [loader], because the stock widget cannot send the bearer token.
ComponentWidgetBuilder assistantImageBuilder(AttachmentImageLoader loader) =>
    (context, node, props, buildChild) {
      final url = props.string('url') ?? '';
      if (url.isEmpty) return const SizedBox.shrink();
      final description = props.string('description') ?? '';
      final variant = props.string('variant');
      final fit =
          BoxFit.values.asNameMap()[props.string('fit')] ??
          (variant == 'header' ? BoxFit.cover : BoxFit.fill);

      final placeholder = _Placeholder(description);
      final image = switch (UrlPolicy.image(url)) {
        BlockedImageTarget() => placeholder,
        NetworkImageTarget(:final url) => Image.network(
          url.toString(),
          fit: fit,
          semanticLabel: description,
          excludeFromSemantics: description.isEmpty,
          errorBuilder: (_, _, _) => placeholder,
        ),
        AttachmentImageTarget(:final attachmentId) => _AttachmentImage(
          // A bound id that changes must start a fresh load.
          key: ValueKey(attachmentId),
          attachmentId: attachmentId,
          loader: loader,
          fit: fit,
          description: description,
          placeholder: placeholder,
        ),
      };
      return _sized(variant, image);
    };

Widget _sized(String? variant, Widget image) => switch (variant) {
  'icon' => _atStart(SizedBox.square(dimension: 24, child: image)),
  'avatar' => _atStart(
    SizedBox.square(dimension: 40, child: ClipOval(child: image)),
  ),
  'smallFeature' => _atStart(
    ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 100),
      child: image,
    ),
  ),
  'largeFeature' => ConstrainedBox(
    constraints: const BoxConstraints(maxHeight: 400),
    child: image,
  ),
  'header' => SizedBox(height: 200, child: image),
  _ => image,
};

Widget _atStart(Widget child) => Align(
  alignment: AlignmentDirectional.topStart,
  widthFactor: 1,
  heightFactor: 1,
  child: child,
);

/// Marks the placeholder shown for a blocked or failed image.
@visibleForTesting
const placeholderKey = Key('a2ui-image-placeholder');

class _Placeholder extends StatelessWidget {
  const _Placeholder(this.description);

  final String description;

  @override
  Widget build(BuildContext context) {
    const icon = Icon(Icons.broken_image, key: placeholderKey);
    return description.isEmpty
        ? icon
        : Semantics(image: true, label: description, child: icon);
  }
}

class _AttachmentImage extends StatefulWidget {
  const _AttachmentImage({
    super.key,
    required this.attachmentId,
    required this.loader,
    required this.fit,
    required this.description,
    required this.placeholder,
  });

  final String attachmentId;
  final AttachmentImageLoader loader;
  final BoxFit fit;
  final String description;
  final Widget placeholder;

  @override
  State<_AttachmentImage> createState() => _AttachmentImageState();
}

class _AttachmentImageState extends State<_AttachmentImage> {
  late final Future<Uint8List> _bytes = widget.loader.load(widget.attachmentId);

  @override
  Widget build(BuildContext context) => FutureBuilder<Uint8List>(
    future: _bytes,
    builder: (context, snapshot) {
      if (snapshot.hasError) return widget.placeholder;
      final bytes = snapshot.data;
      if (bytes == null) return const SizedBox.shrink();
      return Image.memory(
        bytes,
        fit: widget.fit,
        semanticLabel: widget.description,
        excludeFromSemantics: widget.description.isEmpty,
        errorBuilder: (_, _, _) => widget.placeholder,
      );
    },
  );
}
