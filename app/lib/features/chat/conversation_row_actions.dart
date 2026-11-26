import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../shared/platform/adaptive_action_sheet.dart';
import '../../shared/platform/adaptive_dialog.dart';
import '../../shared/platform/adaptive_snack_bar.dart';
import 'conversation_list_provider.dart';

/// An action offered on a conversation row.
enum ConversationRowAction { rename, togglePin, toggleArchive, delete }

/// How the actions of a row are presented.
enum ConversationActionPresentation { menu, sheet }

/// Picks the presentation for a request.
///
/// The input kind decides whenever a pointer is involved: a mouse or
/// trackpad gets a popup menu, a finger or stylus gets a sheet (including on
/// mobile web, where the platform is not Apple but the input is touch).
/// Keyboard activation (`kind == null`) has no pointer, so the platform
/// decides: a menu on desktop and web, a sheet on iOS and Android.
ConversationActionPresentation conversationActionPresentation(
  PointerDeviceKind? kind, {
  required TargetPlatform platform,
  required bool isWeb,
}) {
  switch (kind) {
    case PointerDeviceKind.mouse:
    case PointerDeviceKind.trackpad:
      return ConversationActionPresentation.menu;
    case PointerDeviceKind.touch:
    case PointerDeviceKind.stylus:
    case PointerDeviceKind.invertedStylus:
      return ConversationActionPresentation.sheet;
    case PointerDeviceKind.unknown:
    case null:
      final mobile =
          !isWeb &&
          (platform == TargetPlatform.iOS ||
              platform == TargetPlatform.android);
      return mobile
          ? ConversationActionPresentation.sheet
          : ConversationActionPresentation.menu;
  }
}

/// Shows the row actions for [request] and performs the chosen one.
Future<void> showConversationRowActions(
  BuildContext context,
  WidgetRef ref,
  ConversationActionRequest request,
) async {
  final item = request.item;
  final pinned = item.pinnedAt != null;
  final archived = item.archivedAt != null;
  final presentation = conversationActionPresentation(
    request.kind,
    platform: defaultTargetPlatform,
    isWeb: kIsWeb,
  );

  final ConversationRowAction? chosen;
  switch (presentation) {
    case ConversationActionPresentation.menu:
      chosen = await _showMenu(
        context,
        request,
        pinned: pinned,
        archived: archived,
      );
    case ConversationActionPresentation.sheet:
      chosen = await showAdaptiveActionSheet<ConversationRowAction>(
        context: context,
        title: item.title,
        actions: [
          for (final action in ConversationRowAction.values)
            AdaptiveActionSheetAction<ConversationRowAction>(
              label: _label(action, pinned: pinned, archived: archived),
              isDestructive: action == ConversationRowAction.delete,
              onPressed: (ctx) => Navigator.of(ctx).pop(action),
            ),
        ],
      );
  }
  if (chosen == null || !context.mounted) return;

  switch (chosen) {
    case ConversationRowAction.rename:
      await _rename(context, ref, item);
    case ConversationRowAction.togglePin:
      final error = await ref
          .read(conversationListProvider.notifier)
          .setPinned(item.id, !pinned);
      if (error != null && context.mounted) {
        showAdaptiveSnackBar(
          context,
          'Could not ${pinned ? 'unpin' : 'pin'} conversation: $error',
        );
      }
    case ConversationRowAction.toggleArchive:
      final error = await ref
          .read(conversationListProvider.notifier)
          .setArchived(item.id, !archived);
      if (error != null && context.mounted) {
        showAdaptiveSnackBar(
          context,
          'Could not ${archived ? 'unarchive' : 'archive'} conversation: '
          '$error',
        );
      }
    case ConversationRowAction.delete:
      await confirmAndDeleteConversation(context, ref, item.id, item.title);
  }
}

/// Confirms, then deletes. Resolves `false` on decline or failure so the row
/// stays in the list.
Future<bool> confirmAndDeleteConversation(
  BuildContext context,
  WidgetRef ref,
  String id,
  String title,
) async {
  final confirm = await showAdaptiveConfirmDialog(
    context: context,
    title: 'Delete conversation?',
    content: '"$title" will be permanently deleted.',
    confirmLabel: 'Delete',
    isDestructive: true,
  );
  if (!confirm) return false;
  try {
    await ref.read(conversationListProvider.notifier).deleteConversation(id);
    return true;
  } catch (_) {
    return false;
  }
}

Future<void> _rename(
  BuildContext context,
  WidgetRef ref,
  ConversationListItem item,
) async {
  final title = await showAdaptiveTextInputDialog(
    context: context,
    title: 'Rename conversation',
    confirmLabel: 'Rename',
    initialText: item.title,
  );
  if (title == null || title == item.title || !context.mounted) return;
  final error = await ref
      .read(conversationListProvider.notifier)
      .renameConversation(item.id, title);
  if (error != null && context.mounted) {
    showAdaptiveSnackBar(context, 'Could not rename conversation: $error');
  }
}

String _label(
  ConversationRowAction action, {
  required bool pinned,
  required bool archived,
}) => switch (action) {
  ConversationRowAction.rename => 'Rename',
  ConversationRowAction.togglePin => pinned ? 'Unpin' : 'Pin',
  ConversationRowAction.toggleArchive => archived ? 'Unarchive' : 'Archive',
  ConversationRowAction.delete => 'Delete',
};

Future<ConversationRowAction?> _showMenu(
  BuildContext context,
  ConversationActionRequest request, {
  required bool pinned,
  required bool archived,
}) {
  final overlay = Overlay.of(context).context.findRenderObject() as RenderBox;
  final anchor = request.anchorRect;
  final position = request.position;
  // Below the overflow button, or at the pointer for a row gesture.
  final Rect target;
  if (anchor != null) {
    target = Rect.fromLTWH(anchor.left, anchor.bottom, anchor.width, 0);
  } else if (position != null) {
    target = position & Size.zero;
  } else {
    target = Rect.fromLTWH(0, 0, overlay.size.width, 0);
  }
  final error = Theme.of(context).colorScheme.error;

  return showMenu<ConversationRowAction>(
    context: context,
    position: RelativeRect.fromRect(target, Offset.zero & overlay.size),
    items: [
      for (final action in ConversationRowAction.values)
        PopupMenuItem<ConversationRowAction>(
          value: action,
          child: Text(
            _label(action, pinned: pinned, archived: archived),
            style: action == ConversationRowAction.delete
                ? TextStyle(color: error)
                : null,
          ),
        ),
    ],
  );
}
