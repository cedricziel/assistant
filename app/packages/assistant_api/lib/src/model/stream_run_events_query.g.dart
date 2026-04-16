// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'stream_run_events_query.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$StreamRunEventsQuery extends StreamRunEventsQuery {
  @override
  final int? since;

  factory _$StreamRunEventsQuery(
          [void Function(StreamRunEventsQueryBuilder)? updates]) =>
      (StreamRunEventsQueryBuilder()..update(updates))._build();

  _$StreamRunEventsQuery._({this.since}) : super._();
  @override
  StreamRunEventsQuery rebuild(
          void Function(StreamRunEventsQueryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  StreamRunEventsQueryBuilder toBuilder() =>
      StreamRunEventsQueryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is StreamRunEventsQuery && since == other.since;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, since.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'StreamRunEventsQuery')
          ..add('since', since))
        .toString();
  }
}

class StreamRunEventsQueryBuilder
    implements Builder<StreamRunEventsQuery, StreamRunEventsQueryBuilder> {
  _$StreamRunEventsQuery? _$v;

  int? _since;
  int? get since => _$this._since;
  set since(int? since) => _$this._since = since;

  StreamRunEventsQueryBuilder() {
    StreamRunEventsQuery._defaults(this);
  }

  StreamRunEventsQueryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _since = $v.since;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(StreamRunEventsQuery other) {
    _$v = other as _$StreamRunEventsQuery;
  }

  @override
  void update(void Function(StreamRunEventsQueryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  StreamRunEventsQuery build() => _build();

  _$StreamRunEventsQuery _build() {
    final _$result = _$v ??
        _$StreamRunEventsQuery._(
          since: since,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
