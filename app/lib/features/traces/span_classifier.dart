import 'package:assistant_api/assistant_api.dart';

const String _executeToolPrefix = 'execute_tool ';

/// True when [span] represents a tool invocation. A span is classified as a
/// tool span when its `name` starts with `"execute_tool "` OR, defensively,
/// when its attributes contain a non-null `tool_name`.
bool isToolSpan(SpanEntryResponse span) {
  if (span.name.startsWith(_executeToolPrefix)) return true;
  final attrs = span.attributes?.value;
  if (attrs is Map) {
    final toolName = attrs['tool_name'];
    return toolName != null && toolName is String && toolName.isNotEmpty;
  }
  return false;
}

/// Extract the tool name from a tool [span]. Prefers `attributes['tool_name']`,
/// falls back to the suffix after `"execute_tool "`, and returns an empty
/// string if neither source is available.
String toolNameFromSpan(SpanEntryResponse span) {
  final attrs = span.attributes?.value;
  if (attrs is Map) {
    final attrName = attrs['tool_name'];
    if (attrName is String && attrName.isNotEmpty) return attrName;
  }
  if (span.name.startsWith(_executeToolPrefix)) {
    return span.name.substring(_executeToolPrefix.length);
  }
  return '';
}
