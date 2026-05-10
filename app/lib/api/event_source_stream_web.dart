import 'dart:async';
import 'dart:js_interop';

import 'package:web/web.dart' as web;

import 'event_source_stream.dart';

/// Open a real `EventSource` against [url]. Web-only — selected via
/// conditional import from `api_client.dart`.
EventSourceAdapter openEventSourceAdapter({required String url}) {
  return _WebEventSourceAdapter(url);
}

class _WebEventSourceAdapter implements EventSourceAdapter {
  _WebEventSourceAdapter(this._url) {
    _src = web.EventSource(_url);
    _wireEvent('snapshot');
    _wireEvent('upserted');
    _wireEvent('deleted');
    _src.onerror = ((web.Event _) {
      // EventSource auto-reconnects on transient errors. Only forward an
      // error to the consumer if the connection is in CLOSED state — that
      // means the browser gave up (often a 401 from the server).
      if (_src.readyState == web.EventSource.CLOSED) {
        if (!_ctrl.isClosed) {
          _ctrl.addError(StateError('EventSource closed'));
          _ctrl.close();
        }
      }
    }).toJS;
  }

  final String _url;
  late final web.EventSource _src;
  final _ctrl = StreamController<EventSourceMessage>();

  void _wireEvent(String name) {
    _src.addEventListener(
      name,
      ((web.Event evt) {
        if (evt.isA<web.MessageEvent>()) {
          final messageEvent = evt as web.MessageEvent;
          final data = messageEvent.data;
          final dataStr = data?.isA<JSString>() ?? false
              ? (data! as JSString).toDart
              : data?.toString() ?? '';
          if (!_ctrl.isClosed) {
            _ctrl.add(EventSourceMessage(event: name, data: dataStr));
          }
        }
      }).toJS,
    );
  }

  @override
  Stream<EventSourceMessage> get messages => _ctrl.stream;

  @override
  void close() {
    _src.close();
    if (!_ctrl.isClosed) _ctrl.close();
  }
}
