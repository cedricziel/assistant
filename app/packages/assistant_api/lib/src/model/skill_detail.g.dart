// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'skill_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SkillDetail extends SkillDetail {
  @override
  final String body;
  @override
  final String description;
  @override
  final bool isBuiltin;
  @override
  final String name;
  @override
  final String source_;

  factory _$SkillDetail([void Function(SkillDetailBuilder)? updates]) =>
      (SkillDetailBuilder()..update(updates))._build();

  _$SkillDetail._(
      {required this.body,
      required this.description,
      required this.isBuiltin,
      required this.name,
      required this.source_})
      : super._();
  @override
  SkillDetail rebuild(void Function(SkillDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SkillDetailBuilder toBuilder() => SkillDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SkillDetail &&
        body == other.body &&
        description == other.description &&
        isBuiltin == other.isBuiltin &&
        name == other.name &&
        source_ == other.source_;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, body.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, isBuiltin.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, source_.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SkillDetail')
          ..add('body', body)
          ..add('description', description)
          ..add('isBuiltin', isBuiltin)
          ..add('name', name)
          ..add('source_', source_))
        .toString();
  }
}

class SkillDetailBuilder implements Builder<SkillDetail, SkillDetailBuilder> {
  _$SkillDetail? _$v;

  String? _body;
  String? get body => _$this._body;
  set body(String? body) => _$this._body = body;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  bool? _isBuiltin;
  bool? get isBuiltin => _$this._isBuiltin;
  set isBuiltin(bool? isBuiltin) => _$this._isBuiltin = isBuiltin;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _source_;
  String? get source_ => _$this._source_;
  set source_(String? source_) => _$this._source_ = source_;

  SkillDetailBuilder() {
    SkillDetail._defaults(this);
  }

  SkillDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _body = $v.body;
      _description = $v.description;
      _isBuiltin = $v.isBuiltin;
      _name = $v.name;
      _source_ = $v.source_;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SkillDetail other) {
    _$v = other as _$SkillDetail;
  }

  @override
  void update(void Function(SkillDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SkillDetail build() => _build();

  _$SkillDetail _build() {
    final _$result = _$v ??
        _$SkillDetail._(
          body: BuiltValueNullFieldError.checkNotNull(
              body, r'SkillDetail', 'body'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'SkillDetail', 'description'),
          isBuiltin: BuiltValueNullFieldError.checkNotNull(
              isBuiltin, r'SkillDetail', 'isBuiltin'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'SkillDetail', 'name'),
          source_: BuiltValueNullFieldError.checkNotNull(
              source_, r'SkillDetail', 'source_'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
