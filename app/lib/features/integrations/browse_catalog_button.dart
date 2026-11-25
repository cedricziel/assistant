import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../../shared/platform/widgets.dart';

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
