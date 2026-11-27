// The full-size image viewer's close button is labelled.
//
// Specified by: openspec/changes/chat-a11y-and-code-blocks/specs/
//   chat-accessibility/spec.md ("Icon-only controls have labels and adequate
//   targets").

import 'package:assistant_app/features/chat/message_attachments.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  testWidgets('the image viewer close button is labelled "Close image"', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: MessageAttachments(
            isUser: true,
            imageBaseUrl: 'http://localhost',
            attachments: [
              ChatAttachment(
                id: 'a1',
                filename: 'photo.png',
                mimeType: 'image/png',
                url: '/api/attachments/a1',
              ),
            ],
          ),
        ),
      ),
    );

    await tester.tap(find.byType(CachedNetworkImage));
    await tester.pump();

    final close = find.widgetWithIcon(IconButton, Icons.close);
    expect(close, findsOneWidget, reason: 'the viewer dialog opened');
    expect(tester.widget<IconButton>(close).tooltip, 'Close image');
  });
}
