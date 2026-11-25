import 'package:dio/dio.dart';

/// Header the server reads to pick the space a data-plane request acts in.
const spaceIdHeader = 'X-Space-Id';

/// Query parameter equivalent of [spaceIdHeader], for the browser
/// `EventSource`, which cannot set headers.
const spaceIdQueryParam = 'space_id';

/// Dio interceptor that stamps every request with the selected space's id.
///
/// [spaceId] is read per request, so changing the space selector takes effect
/// on the next call without rebuilding the client. A header the call already
/// set wins.
class SpaceHeaderInterceptor extends Interceptor {
  SpaceHeaderInterceptor({required this.spaceId});

  final String? Function() spaceId;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    final id = spaceId();
    if (id != null && id.isNotEmpty) {
      options.headers.putIfAbsent(spaceIdHeader, () => id);
    }
    handler.next(options);
  }
}
