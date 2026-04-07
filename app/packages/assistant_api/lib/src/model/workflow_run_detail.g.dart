// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_run_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowRunDetail extends WorkflowRunDetail {
  @override
  final WorkflowRunSummary run;
  @override
  final BuiltList<WorkflowRunStep> steps;

  factory _$WorkflowRunDetail(
          [void Function(WorkflowRunDetailBuilder)? updates]) =>
      (WorkflowRunDetailBuilder()..update(updates))._build();

  _$WorkflowRunDetail._({required this.run, required this.steps}) : super._();
  @override
  WorkflowRunDetail rebuild(void Function(WorkflowRunDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowRunDetailBuilder toBuilder() =>
      WorkflowRunDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowRunDetail &&
        run == other.run &&
        steps == other.steps;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, run.hashCode);
    _$hash = $jc(_$hash, steps.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowRunDetail')
          ..add('run', run)
          ..add('steps', steps))
        .toString();
  }
}

class WorkflowRunDetailBuilder
    implements Builder<WorkflowRunDetail, WorkflowRunDetailBuilder> {
  _$WorkflowRunDetail? _$v;

  WorkflowRunSummaryBuilder? _run;
  WorkflowRunSummaryBuilder get run =>
      _$this._run ??= WorkflowRunSummaryBuilder();
  set run(WorkflowRunSummaryBuilder? run) => _$this._run = run;

  ListBuilder<WorkflowRunStep>? _steps;
  ListBuilder<WorkflowRunStep> get steps =>
      _$this._steps ??= ListBuilder<WorkflowRunStep>();
  set steps(ListBuilder<WorkflowRunStep>? steps) => _$this._steps = steps;

  WorkflowRunDetailBuilder() {
    WorkflowRunDetail._defaults(this);
  }

  WorkflowRunDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _run = $v.run.toBuilder();
      _steps = $v.steps.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowRunDetail other) {
    _$v = other as _$WorkflowRunDetail;
  }

  @override
  void update(void Function(WorkflowRunDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowRunDetail build() => _build();

  _$WorkflowRunDetail _build() {
    _$WorkflowRunDetail _$result;
    try {
      _$result = _$v ??
          _$WorkflowRunDetail._(
            run: run.build(),
            steps: steps.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'run';
        run.build();
        _$failedField = 'steps';
        steps.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'WorkflowRunDetail', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
