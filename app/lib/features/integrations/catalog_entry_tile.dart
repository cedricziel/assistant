import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'auth_kind_badge.dart';

/// One catalog entry in the browser list.
class CatalogEntryTile extends StatelessWidget {
  const CatalogEntryTile({super.key, required this.entry, required this.onTap});

  final CatalogEntrySummary entry;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    return Card(
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(AssistantRadius.md),
        child: Padding(
          padding: const EdgeInsets.all(AssistantSpacing.md),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              CatalogEntryIcon(name: entry.name, iconUrl: entry.iconUrl),
              const SizedBox(width: AssistantSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            entry.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              fontWeight: FontWeight.w600,
                              fontSize: 14,
                            ),
                          ),
                        ),
                        const SizedBox(width: AssistantSpacing.sm),
                        AuthKindBadge(kind: entry.auth.kind),
                      ],
                    ),
                    Text(
                      entry.domain,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                    if (entry.description.isNotEmpty) ...[
                      const SizedBox(height: AssistantSpacing.xs),
                      Text(
                        entry.description,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(fontSize: 12),
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

const _iconSize = 40.0;

/// Entry icon loaded from `icon_url`, falling back to the name's initial.
class CatalogEntryIcon extends StatelessWidget {
  const CatalogEntryIcon({super.key, required this.name, this.iconUrl});

  final String name;
  final String? iconUrl;

  @override
  Widget build(BuildContext context) {
    final url = iconUrl;
    final fallback = _InitialAvatar(name: name);
    if (url == null || url.isEmpty) return fallback;

    return ClipRRect(
      borderRadius: BorderRadius.circular(AssistantRadius.sm),
      child: Image.network(
        url,
        width: _iconSize,
        height: _iconSize,
        fit: BoxFit.contain,
        errorBuilder: (_, _, _) => fallback,
      ),
    );
  }
}

class _InitialAvatar extends StatelessWidget {
  const _InitialAvatar({required this.name});

  final String name;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final initial = name.isEmpty ? '?' : name.substring(0, 1).toUpperCase();

    return SizedBox.square(
      dimension: _iconSize,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colors.primaryContainer,
          borderRadius: BorderRadius.circular(AssistantRadius.sm),
        ),
        child: Center(
          child: Text(
            initial,
            style: TextStyle(
              fontWeight: FontWeight.w700,
              color: colors.onPrimaryContainer,
            ),
          ),
        ),
      ),
    );
  }
}
