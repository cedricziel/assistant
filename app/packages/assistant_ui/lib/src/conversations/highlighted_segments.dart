import 'package:material_ui/material_ui.dart';

import '../models/conversation_search_item.dart';

/// Text built from [segments], with the highlighted ones in bold and the
/// theme's primary colour.
///
/// The other segments use [style], falling back to the ambient text style. No
/// offsets are parsed: the caller already split the text. No segments render
/// nothing.
class HighlightedSegments extends StatelessWidget {
  const HighlightedSegments(
    this.segments, {
    super.key,
    this.style,
    this.maxLines,
    this.overflow = TextOverflow.ellipsis,
  });

  final List<SearchSegmentItem> segments;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow overflow;

  @override
  Widget build(BuildContext context) {
    if (segments.isEmpty) return const SizedBox.shrink();

    final highlight = TextStyle(
      fontWeight: FontWeight.bold,
      color: Theme.of(context).colorScheme.primary,
    );

    return Text.rich(
      TextSpan(
        children: [
          for (final segment in segments)
            TextSpan(
              text: segment.text,
              style: segment.highlighted ? highlight : null,
            ),
        ],
      ),
      style: style,
      maxLines: maxLines,
      overflow: overflow,
    );
  }
}
