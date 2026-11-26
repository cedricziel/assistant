// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tool_permission_rule_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ToolPermissionRuleSummary extends ToolPermissionRuleSummary {
  @override
  final String createdAt;
  @override
  final String effect;
  @override
  final String id;
  @override
  final String rule;
  @override
  final String toolName;
  @override
  final String userId;

  factory _$ToolPermissionRuleSummary(
          [void Function(ToolPermissionRuleSummaryBuilder)? updates]) =>
      (ToolPermissionRuleSummaryBuilder()..update(updates))._build();

  _$ToolPermissionRuleSummary._(
      {required this.createdAt,
      required this.effect,
      required this.id,
      required this.rule,
      required this.toolName,
      required this.userId})
      : super._();
  @override
  ToolPermissionRuleSummary rebuild(
          void Function(ToolPermissionRuleSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ToolPermissionRuleSummaryBuilder toBuilder() =>
      ToolPermissionRuleSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ToolPermissionRuleSummary &&
        createdAt == other.createdAt &&
        effect == other.effect &&
        id == other.id &&
        rule == other.rule &&
        toolName == other.toolName &&
        userId == other.userId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, effect.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, rule.hashCode);
    _$hash = $jc(_$hash, toolName.hashCode);
    _$hash = $jc(_$hash, userId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ToolPermissionRuleSummary')
          ..add('createdAt', createdAt)
          ..add('effect', effect)
          ..add('id', id)
          ..add('rule', rule)
          ..add('toolName', toolName)
          ..add('userId', userId))
        .toString();
  }
}

class ToolPermissionRuleSummaryBuilder
    implements
        Builder<ToolPermissionRuleSummary, ToolPermissionRuleSummaryBuilder> {
  _$ToolPermissionRuleSummary? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _effect;
  String? get effect => _$this._effect;
  set effect(String? effect) => _$this._effect = effect;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _rule;
  String? get rule => _$this._rule;
  set rule(String? rule) => _$this._rule = rule;

  String? _toolName;
  String? get toolName => _$this._toolName;
  set toolName(String? toolName) => _$this._toolName = toolName;

  String? _userId;
  String? get userId => _$this._userId;
  set userId(String? userId) => _$this._userId = userId;

  ToolPermissionRuleSummaryBuilder() {
    ToolPermissionRuleSummary._defaults(this);
  }

  ToolPermissionRuleSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _effect = $v.effect;
      _id = $v.id;
      _rule = $v.rule;
      _toolName = $v.toolName;
      _userId = $v.userId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ToolPermissionRuleSummary other) {
    _$v = other as _$ToolPermissionRuleSummary;
  }

  @override
  void update(void Function(ToolPermissionRuleSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ToolPermissionRuleSummary build() => _build();

  _$ToolPermissionRuleSummary _build() {
    final _$result = _$v ??
        _$ToolPermissionRuleSummary._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'ToolPermissionRuleSummary', 'createdAt'),
          effect: BuiltValueNullFieldError.checkNotNull(
              effect, r'ToolPermissionRuleSummary', 'effect'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'ToolPermissionRuleSummary', 'id'),
          rule: BuiltValueNullFieldError.checkNotNull(
              rule, r'ToolPermissionRuleSummary', 'rule'),
          toolName: BuiltValueNullFieldError.checkNotNull(
              toolName, r'ToolPermissionRuleSummary', 'toolName'),
          userId: BuiltValueNullFieldError.checkNotNull(
              userId, r'ToolPermissionRuleSummary', 'userId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
