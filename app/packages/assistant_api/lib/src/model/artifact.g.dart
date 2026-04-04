// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'artifact.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$Artifact extends Artifact {
  @override
  final String artifactId;
  @override
  final String? description;
  @override
  final BuiltList<String>? extensions;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final String? name;
  @override
  final BuiltList<ModelPart> parts;

  factory _$Artifact([void Function(ArtifactBuilder)? updates]) =>
      (ArtifactBuilder()..update(updates))._build();

  _$Artifact._(
      {required this.artifactId,
      this.description,
      this.extensions,
      this.metadata,
      this.name,
      required this.parts})
      : super._();
  @override
  Artifact rebuild(void Function(ArtifactBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ArtifactBuilder toBuilder() => ArtifactBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is Artifact &&
        artifactId == other.artifactId &&
        description == other.description &&
        extensions == other.extensions &&
        metadata == other.metadata &&
        name == other.name &&
        parts == other.parts;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, artifactId.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, extensions.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, parts.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'Artifact')
          ..add('artifactId', artifactId)
          ..add('description', description)
          ..add('extensions', extensions)
          ..add('metadata', metadata)
          ..add('name', name)
          ..add('parts', parts))
        .toString();
  }
}

class ArtifactBuilder implements Builder<Artifact, ArtifactBuilder> {
  _$Artifact? _$v;

  String? _artifactId;
  String? get artifactId => _$this._artifactId;
  set artifactId(String? artifactId) => _$this._artifactId = artifactId;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  ListBuilder<String>? _extensions;
  ListBuilder<String> get extensions =>
      _$this._extensions ??= ListBuilder<String>();
  set extensions(ListBuilder<String>? extensions) =>
      _$this._extensions = extensions;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  ListBuilder<ModelPart>? _parts;
  ListBuilder<ModelPart> get parts =>
      _$this._parts ??= ListBuilder<ModelPart>();
  set parts(ListBuilder<ModelPart>? parts) => _$this._parts = parts;

  ArtifactBuilder() {
    Artifact._defaults(this);
  }

  ArtifactBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _artifactId = $v.artifactId;
      _description = $v.description;
      _extensions = $v.extensions?.toBuilder();
      _metadata = $v.metadata?.toBuilder();
      _name = $v.name;
      _parts = $v.parts.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(Artifact other) {
    _$v = other as _$Artifact;
  }

  @override
  void update(void Function(ArtifactBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  Artifact build() => _build();

  _$Artifact _build() {
    _$Artifact _$result;
    try {
      _$result = _$v ??
          _$Artifact._(
            artifactId: BuiltValueNullFieldError.checkNotNull(
                artifactId, r'Artifact', 'artifactId'),
            description: description,
            extensions: _extensions?.build(),
            metadata: _metadata?.build(),
            name: name,
            parts: parts.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'extensions';
        _extensions?.build();
        _$failedField = 'metadata';
        _metadata?.build();

        _$failedField = 'parts';
        parts.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'Artifact', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
