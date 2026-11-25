import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';

/// Opens the catalog, where integrations are installed.
class BrowseCatalogButton extends StatelessWidget {
  const BrowseCatalogButton({super.key, required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return AdaptiveButton.plain(
      onPressed: () => context.push(AppRoutes.integrations),
      child: Text(label),
    );
  }
}

/// A left-aligned [BrowseCatalogButton] above a list.
class CatalogActionBar extends StatelessWidget {
  const CatalogActionBar({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(
        AssistantSpacing.lg,
        AssistantSpacing.sm,
        AssistantSpacing.lg,
        0,
      ),
      child: Align(alignment: Alignment.centerLeft, child: child),
    );
  }
}
