// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_status_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseStatusEvent extends SseStatusEvent {
  @override
  final String message;

  factory _$SseStatusEvent([void Function(SseStatusEventBuilder)? updates]) =>
      (SseStatusEventBuilder()..update(updates))._build();

  _$SseStatusEvent._({required this.message}) : super._();
  @override
  SseStatusEvent rebuild(void Function(SseStatusEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseStatusEventBuilder toBuilder() => SseStatusEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseStatusEvent && message == other.message;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseStatusEvent')
          ..add('message', message))
        .toString();
  }
}

class SseStatusEventBuilder
    implements Builder<SseStatusEvent, SseStatusEventBuilder> {
  _$SseStatusEvent? _$v;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  SseStatusEventBuilder() {
    SseStatusEvent._defaults(this);
  }

  SseStatusEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _message = $v.message;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseStatusEvent other) {
    _$v = other as _$SseStatusEvent;
  }

  @override
  void update(void Function(SseStatusEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseStatusEvent build() => _build();

  _$SseStatusEvent _build() {
    final _$result = _$v ??
        _$SseStatusEvent._(
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'SseStatusEvent', 'message'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
