import 'package:a2ui_core/a2ui_core.dart';
import 'package:a2ui_flutter/a2ui_flutter.dart';
import 'package:a2ui_flutter/basic_catalog.dart';

import 'assistant_image.dart';
import 'attachment_image_loader.dart';
import 'link_opener.dart';

/// The catalog id every surface carries; `catalogId` in
/// `crates/a2ui/catalog/assistant-v1.json`.
const kAssistantCatalogId =
    'https://assistant.invalid/a2ui/catalogs/assistant/v1/catalog.json';

/// The fifteen components of the assistant catalog. Video, AudioPlayer and
/// Modal are out (ADR-0019 section 1); `crates/a2ui/catalog/components.txt`
/// is the shared fixture.
const _assistantComponents = {
  'Text',
  'Image',
  'Icon',
  'Row',
  'Column',
  'List',
  'Card',
  'Tabs',
  'Divider',
  'Button',
  'TextField',
  'CheckBox',
  'ChoicePicker',
  'Slider',
  'DateTimeInput',
};

/// Builds the assistant catalog from the stock basic components, under the
/// assistant catalog id, with `Image` replaced by the app implementation.
///
/// [openUrl] receives `openUrl` calls, which the catalog runs only from a user
/// gesture. It defaults to [ignoreLink], so nothing opens until the policy
/// checked implementation of PR 14b is installed.
CatalogImplementation assistantCatalog({
  required AttachmentImageLoader imageLoader,
  LinkOpener openUrl = ignoreLink,
}) {
  final basic = basicCatalog(openUrl: openUrl);
  return CatalogImplementation(
    id: kAssistantCatalogId,
    components: [
      for (final component in BasicComponents.all)
        if (_assistantComponents.contains(component.name))
          component.name == 'Image'
              ? ComponentImplementation(
                  name: 'Image',
                  schema: component.schema,
                  allowedParents: component.allowedParents,
                  allowedChildren: component.allowedChildren,
                  builder: assistantImageBuilder(imageLoader),
                )
              : component,
    ],
    functions: [
      for (final function in basic.functions.values)
        function.name == 'regex' ? _CappedRegex(function) : function,
    ],
    schemaId: kAssistantCatalogId,
    title: 'Assistant catalog',
    description:
        'The assistant catalog: A2UI v1.0 basic components minus '
        'Video, AudioPlayer and Modal.',
    protocolVersion: basic.protocolVersion,
    definitions: basic.definitions,
  );
}

/// The most code points the `regex` function evaluates (design D7). The
/// library's own cap is 4,096; the client evaluates with a backtracking
/// engine, so this one is lower.
const kRegexInputCap = 1000;

/// The basic `regex` function with a tighter input cap. Longer input fails
/// the check without compiling or running the pattern.
class _CappedRegex extends FunctionImplementation {
  _CappedRegex(this._inner)
    : super(
        name: _inner.name,
        argumentSchema: _inner.argumentSchema,
        returnType: _inner.returnType,
        description: _inner.description,
        allowedCallers: _inner.declaredAllowedCallers,
        requiresUserActivation: _inner.declaredRequiresUserActivation,
        definition: _inner.definition,
      );

  final FunctionImplementation _inner;

  @override
  Object? execute(
    Map<String, dynamic> args,
    DataContext context, [
    CancellationSignal? cancellationSignal,
  ]) {
    final value = args['value'];
    final text = value is String ? value : (value?.toString() ?? '');
    if (text.runes.length > kRegexInputCap) {
      return const ValidationResult(
        valid: false,
        message: 'Value does not match required pattern.',
      );
    }
    return _inner.execute(args, context, cancellationSignal);
  }
}
