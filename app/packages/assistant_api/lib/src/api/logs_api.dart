//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

import 'dart:async';

import 'package:built_value/json_object.dart';
import 'package:built_value/serializer.dart';
import 'package:dio/dio.dart';

import 'package:assistant_api/src/api_util.dart';
import 'package:assistant_api/src/model/log_entry_response.dart';
import 'package:built_collection/built_collection.dart';

class LogsApi {
  final Dio _dio;

  final Serializers _serializers;

  const LogsApi(this._dio, this._serializers);

  /// &#x60;GET /api/logs&#x60; — list recent log entries, newest first.
  ///
  ///
  /// Parameters:
  /// * [limit]
  /// * [offset]
  /// * [search]
  /// * [severity]
  /// * [since]
  /// * [until]
  /// * [traceId]
  /// * [conversation]
  /// * [cancelToken] - A [CancelToken] that can be used to cancel the operation
  /// * [headers] - Can be used to add additional headers to the request
  /// * [extras] - Can be used to add flags to the request
  /// * [validateStatus] - A [ValidateStatus] callback that can be used to determine request success based on the HTTP status of the response
  /// * [onSendProgress] - A [ProgressCallback] that can be used to get the send progress
  /// * [onReceiveProgress] - A [ProgressCallback] that can be used to get the receive progress
  ///
  /// Returns a [Future] containing a [Response] with a [BuiltList<LogEntryResponse>] as data
  /// Throws [DioException] if API call or serialization fails
  Future<Response<BuiltList<LogEntryResponse>>> listLogs({
    int? limit,
    int? offset,
    String? search,
    String? severity,
    DateTime? since,
    DateTime? until,
    String? traceId,
    String? conversation,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final _path = r'/api/logs';
    final _options = Options(
      method: r'GET',
      headers: <String, dynamic>{
        ...?headers,
      },
      extra: <String, dynamic>{
        'secure': <Map<String, String>>[
          {
            'type': 'http',
            'scheme': 'bearer',
            'name': 'bearer_token',
          },
        ],
        ...?extra,
      },
      validateStatus: validateStatus,
    );

    final _queryParameters = <String, dynamic>{
      r'limit': encodeQueryParameter(_serializers, limit, const FullType(int)),
      r'offset':
          encodeQueryParameter(_serializers, offset, const FullType(int)),
      r'search':
          encodeQueryParameter(_serializers, search, const FullType(String)),
      r'severity':
          encodeQueryParameter(_serializers, severity, const FullType(String)),
      r'since':
          encodeQueryParameter(_serializers, since, const FullType(DateTime)),
      r'until':
          encodeQueryParameter(_serializers, until, const FullType(DateTime)),
      r'trace_id':
          encodeQueryParameter(_serializers, traceId, const FullType(String)),
      r'conversation': encodeQueryParameter(
          _serializers, conversation, const FullType(String)),
    };

    final _response = await _dio.request<Object>(
      _path,
      options: _options,
      queryParameters: _queryParameters,
      cancelToken: cancelToken,
      onSendProgress: onSendProgress,
      onReceiveProgress: onReceiveProgress,
    );

    BuiltList<LogEntryResponse>? _responseData;

    try {
      final rawResponse = _response.data;
      _responseData = rawResponse == null
          ? null
          : _serializers.deserialize(
              rawResponse,
              specifiedType:
                  const FullType(BuiltList, [FullType(LogEntryResponse)]),
            ) as BuiltList<LogEntryResponse>;
    } catch (error, stackTrace) {
      throw DioException(
        requestOptions: _response.requestOptions,
        response: _response,
        type: DioExceptionType.unknown,
        error: error,
        stackTrace: stackTrace,
      );
    }

    return Response<BuiltList<LogEntryResponse>>(
      data: _responseData,
      headers: _response.headers,
      isRedirect: _response.isRedirect,
      requestOptions: _response.requestOptions,
      redirects: _response.redirects,
      statusCode: _response.statusCode,
      statusMessage: _response.statusMessage,
      extra: _response.extra,
    );
  }
}
