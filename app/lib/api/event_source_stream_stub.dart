import 'event_source_stream.dart';

/// Stub used on non-web builds. Conditional import in `api_client.dart`
/// only delegates here when `kIsWeb`, so this stub should never actually
/// be invoked. Throws if it is — better than silently returning the wrong
/// thing.
EventSourceAdapter openEventSourceAdapter({required String url}) {
  throw UnsupportedError('EventSource is only available on web builds');
}
