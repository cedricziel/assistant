//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

import 'dart:async';

import 'package:built_value/json_object.dart';
import 'package:built_value/serializer.dart';
import 'package:dio/dio.dart';

import 'package:assistant_api/src/api_util.dart';
import 'package:assistant_api/src/model/create_interface_instance_request.dart';
import 'package:assistant_api/src/model/error_body.dart';
import 'package:assistant_api/src/model/interface_instance_response.dart';
import 'package:built_collection/built_collection.dart';

class InterfacesApi {
  final Dio _dio;

  final Serializers _serializers;

  const InterfacesApi(this._dio, this._serializers);

  /// &#x60;POST /api/orgs/{org_id}/spaces/{space_id}/interfaces&#x60; — create an interface instance.
  ///
  ///
  /// Parameters:
  /// * [orgId] - Organization ID
  /// * [spaceId] - Space ID
  /// * [createInterfaceInstanceRequest]
  /// * [cancelToken] - A [CancelToken] that can be used to cancel the operation
  /// * [headers] - Can be used to add additional headers to the request
  /// * [extras] - Can be used to add flags to the request
  /// * [validateStatus] - A [ValidateStatus] callback that can be used to determine request success based on the HTTP status of the response
  /// * [onSendProgress] - A [ProgressCallback] that can be used to get the send progress
  /// * [onReceiveProgress] - A [ProgressCallback] that can be used to get the receive progress
  ///
  /// Returns a [Future] containing a [Response] with a [InterfaceInstanceResponse] as data
  /// Throws [DioException] if API call or serialization fails
  Future<Response<InterfaceInstanceResponse>> createInterface({
    required String orgId,
    required String spaceId,
    required CreateInterfaceInstanceRequest createInterfaceInstanceRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final _path = r'/api/orgs/{org_id}/spaces/{space_id}/interfaces'
        .replaceAll(
            '{' r'org_id' '}',
            encodeQueryParameter(_serializers, orgId, const FullType(String))
                .toString())
        .replaceAll(
            '{' r'space_id' '}',
            encodeQueryParameter(_serializers, spaceId, const FullType(String))
                .toString());
    final _options = Options(
      method: r'POST',
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
          {
            'type': 'oauth2',
            'name': 'oauth2',
          },
          {
            'type': 'oauth2',
            'name': 'oauth2',
          },
        ],
        ...?extra,
      },
      contentType: 'application/json',
      validateStatus: validateStatus,
    );

    dynamic _bodyData;

    try {
      const _type = FullType(CreateInterfaceInstanceRequest);
      _bodyData = _serializers.serialize(createInterfaceInstanceRequest,
          specifiedType: _type);
    } catch (error, stackTrace) {
      throw DioException(
        requestOptions: _options.compose(
          _dio.options,
          _path,
        ),
        type: DioExceptionType.unknown,
        error: error,
        stackTrace: stackTrace,
      );
    }

    final _response = await _dio.request<Object>(
      _path,
      data: _bodyData,
      options: _options,
      cancelToken: cancelToken,
      onSendProgress: onSendProgress,
      onReceiveProgress: onReceiveProgress,
    );

    InterfaceInstanceResponse? _responseData;

    try {
      final rawResponse = _response.data;
      _responseData = rawResponse == null
          ? null
          : _serializers.deserialize(
              rawResponse,
              specifiedType: const FullType(InterfaceInstanceResponse),
            ) as InterfaceInstanceResponse;
    } catch (error, stackTrace) {
      throw DioException(
        requestOptions: _response.requestOptions,
        response: _response,
        type: DioExceptionType.unknown,
        error: error,
        stackTrace: stackTrace,
      );
    }

    return Response<InterfaceInstanceResponse>(
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

  /// &#x60;DELETE /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}&#x60; — delete an interface instance.
  ///
  ///
  /// Parameters:
  /// * [orgId] - Organization ID
  /// * [spaceId] - Space ID
  /// * [id] - Interface instance ID
  /// * [cancelToken] - A [CancelToken] that can be used to cancel the operation
  /// * [headers] - Can be used to add additional headers to the request
  /// * [extras] - Can be used to add flags to the request
  /// * [validateStatus] - A [ValidateStatus] callback that can be used to determine request success based on the HTTP status of the response
  /// * [onSendProgress] - A [ProgressCallback] that can be used to get the send progress
  /// * [onReceiveProgress] - A [ProgressCallback] that can be used to get the receive progress
  ///
  /// Returns a [Future]
  /// Throws [DioException] if API call or serialization fails
  Future<Response<void>> deleteInterface({
    required String orgId,
    required String spaceId,
    required String id,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final _path = r'/api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}'
        .replaceAll(
            '{' r'org_id' '}',
            encodeQueryParameter(_serializers, orgId, const FullType(String))
                .toString())
        .replaceAll(
            '{' r'space_id' '}',
            encodeQueryParameter(_serializers, spaceId, const FullType(String))
                .toString())
        .replaceAll(
            '{' r'id' '}',
            encodeQueryParameter(_serializers, id, const FullType(String))
                .toString());
    final _options = Options(
      method: r'DELETE',
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
          {
            'type': 'oauth2',
            'name': 'oauth2',
          },
          {
            'type': 'oauth2',
            'name': 'oauth2',
          },
        ],
        ...?extra,
      },
      validateStatus: validateStatus,
    );

    final _response = await _dio.request<Object>(
      _path,
      options: _options,
      cancelToken: cancelToken,
      onSendProgress: onSendProgress,
      onReceiveProgress: onReceiveProgress,
    );

    return _response;
  }

  /// &#x60;GET /api/orgs/{org_id}/spaces/{space_id}/interfaces&#x60; — list interface instances.
  ///
  ///
  /// Parameters:
  /// * [orgId] - Organization ID
  /// * [spaceId] - Space ID
  /// * [cancelToken] - A [CancelToken] that can be used to cancel the operation
  /// * [headers] - Can be used to add additional headers to the request
  /// * [extras] - Can be used to add flags to the request
  /// * [validateStatus] - A [ValidateStatus] callback that can be used to determine request success based on the HTTP status of the response
  /// * [onSendProgress] - A [ProgressCallback] that can be used to get the send progress
  /// * [onReceiveProgress] - A [ProgressCallback] that can be used to get the receive progress
  ///
  /// Returns a [Future] containing a [Response] with a [BuiltList<InterfaceInstanceResponse>] as data
  /// Throws [DioException] if API call or serialization fails
  Future<Response<BuiltList<InterfaceInstanceResponse>>> listInterfaces({
    required String orgId,
    required String spaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final _path = r'/api/orgs/{org_id}/spaces/{space_id}/interfaces'
        .replaceAll(
            '{' r'org_id' '}',
            encodeQueryParameter(_serializers, orgId, const FullType(String))
                .toString())
        .replaceAll(
            '{' r'space_id' '}',
            encodeQueryParameter(_serializers, spaceId, const FullType(String))
                .toString());
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
          {
            'type': 'oauth2',
            'name': 'oauth2',
          },
          {
            'type': 'oauth2',
            'name': 'oauth2',
          },
        ],
        ...?extra,
      },
      validateStatus: validateStatus,
    );

    final _response = await _dio.request<Object>(
      _path,
      options: _options,
      cancelToken: cancelToken,
      onSendProgress: onSendProgress,
      onReceiveProgress: onReceiveProgress,
    );

    BuiltList<InterfaceInstanceResponse>? _responseData;

    try {
      final rawResponse = _response.data;
      _responseData = rawResponse == null
          ? null
          : _serializers.deserialize(
              rawResponse,
              specifiedType: const FullType(
                  BuiltList, [FullType(InterfaceInstanceResponse)]),
            ) as BuiltList<InterfaceInstanceResponse>;
    } catch (error, stackTrace) {
      throw DioException(
        requestOptions: _response.requestOptions,
        response: _response,
        type: DioExceptionType.unknown,
        error: error,
        stackTrace: stackTrace,
      );
    }

    return Response<BuiltList<InterfaceInstanceResponse>>(
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
