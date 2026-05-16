import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Icon that picks between a Cupertino glyph and a Material glyph based on
/// the runtime platform.
///
/// Use this when the iOS-native (SF Symbols) and Material icon for the
/// same concept have different [IconData]. Both arguments are required so
/// that call sites are explicit about the pair.
class AdaptiveIcon extends StatelessWidget {
  const AdaptiveIcon({
    super.key,
    required this.cupertino,
    required this.material,
    this.size,
    this.color,
  });

  final IconData cupertino;
  final IconData material;
  final double? size;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    return Icon(isAppleTouch ? cupertino : material, size: size, color: color);
  }
}
