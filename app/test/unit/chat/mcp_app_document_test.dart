import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/mcp_app/mcp_app_document.dart';

const _defaultPolicy =
    "default-src 'none'; script-src 'self' 'unsafe-inline'; "
    "style-src 'self' 'unsafe-inline'; img-src 'self' data:; "
    "media-src 'self' data:; connect-src 'none'; frame-src 'none'; "
    "base-uri 'self'; object-src 'none'";

McpApp _app(String html, {McpAppCsp csp = const McpAppCsp()}) =>
    McpApp(server: 'weather', resourceUri: 'ui://w/app', html: html, csp: csp);

void main() {
  group('mcpAppCsp', () {
    test('uses the restrictive default when no domains are declared', () {
      expect(mcpAppCsp(const McpAppCsp()), _defaultPolicy);
    });

    test('adds each declared domain to its directives only', () {
      final policy = mcpAppCsp(
        const McpAppCsp(
          connectDomains: ['https://api.example.com'],
          resourceDomains: ['https://cdn.example.com'],
          frameDomains: ['https://embed.example.com'],
          baseUriDomains: ['https://base.example.com'],
        ),
      );
      final directives = {
        for (final d in policy.split('; '))
          d.split(' ').first: d.split(' ').skip(1).join(' '),
      };

      expect(directives['default-src'], "'none'");
      expect(directives['connect-src'], 'https://api.example.com');
      for (final name in [
        'script-src',
        'style-src',
        'img-src',
        'font-src',
        'media-src',
      ]) {
        expect(
          directives[name],
          contains('https://cdn.example.com'),
          reason: '$name carries resource domains',
        );
      }
      expect(directives['script-src'], startsWith("'self' 'unsafe-inline'"));
      expect(directives['frame-src'], 'https://embed.example.com');
      expect(directives['base-uri'], 'https://base.example.com');
      expect(directives['object-src'], "'none'");
    });

    test('drops domains that would inject extra directives', () {
      final policy = mcpAppCsp(
        const McpAppCsp(
          connectDomains: ["https://ok.example.com; script-src *", "'self'"],
        ),
      );

      expect(policy, isNot(contains('script-src *')));
      expect(policy, contains("connect-src 'none'"));
    });
  });

  group('mcpAppAllow', () {
    test('is empty when no permission is requested', () {
      expect(mcpAppAllow(const McpAppPermissions()), '');
    });

    test('maps each requested permission to a feature policy', () {
      expect(
        mcpAppAllow(
          const McpAppPermissions(
            camera: true,
            microphone: true,
            geolocation: true,
            clipboardWrite: true,
          ),
        ),
        'camera; microphone; geolocation; clipboard-write',
      );
    });
  });

  group('mcpAppDocument', () {
    const meta =
        '<meta http-equiv="Content-Security-Policy" '
        'content="$_defaultPolicy">';

    test('puts the policy ahead of everything in the document', () {
      final doc = mcpAppDocument(
        _app('<html><script>early()</script><head></head></html>'),
      );

      expect(doc, startsWith(meta));
      expect(
        doc,
        endsWith('<html><script>early()</script><head></head></html>'),
      );
    });

    test('keeps the doctype first so the frame stays in standards mode', () {
      final doc = mcpAppDocument(_app('<!DOCTYPE html>\n<html></html>'));

      expect(doc, startsWith('<!DOCTYPE html>$meta'));
    });

    test('follows the policy with the bootstrap script', () {
      final doc = mcpAppDocument(_app('<p>hi</p>'), bootstrapScript: 'boot()');

      expect(doc, '$meta<script>boot()</script><p>hi</p>');
    });
  });
}
