// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14b.1.
import 'package:assistant_app/features/chat/a2ui/url_policy.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('UrlPolicy.link', () {
    for (final url in [
      'https://example.org/a?b=c',
      'HTTPS://Example.org/a',
      'mailto:a@example.org',
    ]) {
      test('$url is allowed', () {
        expect(UrlPolicy.link(url), isNotNull);
      });
    }

    for (final url in [
      'javascript:alert(1)',
      'data:text/html;base64,PHNjcmlwdD4=',
      'http://example.org/a',
      'tel:+4912345',
      'file:///etc/passwd',
      '/api/attachments/3f2b8c1e-1d4a-4c8e-9a57-0b6d1f2e7a90',
      'https:///nohost',
      'mailto:',
      '',
      'not a url',
    ]) {
      test('"$url" is blocked', () {
        expect(UrlPolicy.link(url), isNull);
      });
    }

    test('returns the parsed target', () {
      expect(
        UrlPolicy.link('https://example.org/a'),
        Uri.parse('https://example.org/a'),
      );
    });
  });
}
