// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_card.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentCard extends AgentCard {
  @override
  final AgentCapabilities capabilities;
  @override
  final BuiltList<String> defaultInputModes;
  @override
  final BuiltList<String> defaultOutputModes;
  @override
  final String description;
  @override
  final String? documentationUrl;
  @override
  final String? iconUrl;
  @override
  final String name;
  @override
  final AgentProvider? provider;
  @override
  final BuiltList<SecurityRequirement>? securityRequirements;
  @override
  final BuiltMap<String, SecurityScheme>? securitySchemes;
  @override
  final BuiltList<AgentCardSignature>? signatures;
  @override
  final BuiltList<AgentSkill> skills;
  @override
  final BuiltList<AgentInterface> supportedInterfaces;
  @override
  final String version;

  factory _$AgentCard([void Function(AgentCardBuilder)? updates]) =>
      (AgentCardBuilder()..update(updates))._build();

  _$AgentCard._(
      {required this.capabilities,
      required this.defaultInputModes,
      required this.defaultOutputModes,
      required this.description,
      this.documentationUrl,
      this.iconUrl,
      required this.name,
      this.provider,
      this.securityRequirements,
      this.securitySchemes,
      this.signatures,
      required this.skills,
      required this.supportedInterfaces,
      required this.version})
      : super._();
  @override
  AgentCard rebuild(void Function(AgentCardBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentCardBuilder toBuilder() => AgentCardBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentCard &&
        capabilities == other.capabilities &&
        defaultInputModes == other.defaultInputModes &&
        defaultOutputModes == other.defaultOutputModes &&
        description == other.description &&
        documentationUrl == other.documentationUrl &&
        iconUrl == other.iconUrl &&
        name == other.name &&
        provider == other.provider &&
        securityRequirements == other.securityRequirements &&
        securitySchemes == other.securitySchemes &&
        signatures == other.signatures &&
        skills == other.skills &&
        supportedInterfaces == other.supportedInterfaces &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, capabilities.hashCode);
    _$hash = $jc(_$hash, defaultInputModes.hashCode);
    _$hash = $jc(_$hash, defaultOutputModes.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, documentationUrl.hashCode);
    _$hash = $jc(_$hash, iconUrl.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, provider.hashCode);
    _$hash = $jc(_$hash, securityRequirements.hashCode);
    _$hash = $jc(_$hash, securitySchemes.hashCode);
    _$hash = $jc(_$hash, signatures.hashCode);
    _$hash = $jc(_$hash, skills.hashCode);
    _$hash = $jc(_$hash, supportedInterfaces.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentCard')
          ..add('capabilities', capabilities)
          ..add('defaultInputModes', defaultInputModes)
          ..add('defaultOutputModes', defaultOutputModes)
          ..add('description', description)
          ..add('documentationUrl', documentationUrl)
          ..add('iconUrl', iconUrl)
          ..add('name', name)
          ..add('provider', provider)
          ..add('securityRequirements', securityRequirements)
          ..add('securitySchemes', securitySchemes)
          ..add('signatures', signatures)
          ..add('skills', skills)
          ..add('supportedInterfaces', supportedInterfaces)
          ..add('version', version))
        .toString();
  }
}

class AgentCardBuilder implements Builder<AgentCard, AgentCardBuilder> {
  _$AgentCard? _$v;

  AgentCapabilitiesBuilder? _capabilities;
  AgentCapabilitiesBuilder get capabilities =>
      _$this._capabilities ??= AgentCapabilitiesBuilder();
  set capabilities(AgentCapabilitiesBuilder? capabilities) =>
      _$this._capabilities = capabilities;

  ListBuilder<String>? _defaultInputModes;
  ListBuilder<String> get defaultInputModes =>
      _$this._defaultInputModes ??= ListBuilder<String>();
  set defaultInputModes(ListBuilder<String>? defaultInputModes) =>
      _$this._defaultInputModes = defaultInputModes;

  ListBuilder<String>? _defaultOutputModes;
  ListBuilder<String> get defaultOutputModes =>
      _$this._defaultOutputModes ??= ListBuilder<String>();
  set defaultOutputModes(ListBuilder<String>? defaultOutputModes) =>
      _$this._defaultOutputModes = defaultOutputModes;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _documentationUrl;
  String? get documentationUrl => _$this._documentationUrl;
  set documentationUrl(String? documentationUrl) =>
      _$this._documentationUrl = documentationUrl;

  String? _iconUrl;
  String? get iconUrl => _$this._iconUrl;
  set iconUrl(String? iconUrl) => _$this._iconUrl = iconUrl;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  AgentProviderBuilder? _provider;
  AgentProviderBuilder get provider =>
      _$this._provider ??= AgentProviderBuilder();
  set provider(AgentProviderBuilder? provider) => _$this._provider = provider;

  ListBuilder<SecurityRequirement>? _securityRequirements;
  ListBuilder<SecurityRequirement> get securityRequirements =>
      _$this._securityRequirements ??= ListBuilder<SecurityRequirement>();
  set securityRequirements(
          ListBuilder<SecurityRequirement>? securityRequirements) =>
      _$this._securityRequirements = securityRequirements;

  MapBuilder<String, SecurityScheme>? _securitySchemes;
  MapBuilder<String, SecurityScheme> get securitySchemes =>
      _$this._securitySchemes ??= MapBuilder<String, SecurityScheme>();
  set securitySchemes(MapBuilder<String, SecurityScheme>? securitySchemes) =>
      _$this._securitySchemes = securitySchemes;

  ListBuilder<AgentCardSignature>? _signatures;
  ListBuilder<AgentCardSignature> get signatures =>
      _$this._signatures ??= ListBuilder<AgentCardSignature>();
  set signatures(ListBuilder<AgentCardSignature>? signatures) =>
      _$this._signatures = signatures;

  ListBuilder<AgentSkill>? _skills;
  ListBuilder<AgentSkill> get skills =>
      _$this._skills ??= ListBuilder<AgentSkill>();
  set skills(ListBuilder<AgentSkill>? skills) => _$this._skills = skills;

  ListBuilder<AgentInterface>? _supportedInterfaces;
  ListBuilder<AgentInterface> get supportedInterfaces =>
      _$this._supportedInterfaces ??= ListBuilder<AgentInterface>();
  set supportedInterfaces(ListBuilder<AgentInterface>? supportedInterfaces) =>
      _$this._supportedInterfaces = supportedInterfaces;

  String? _version;
  String? get version => _$this._version;
  set version(String? version) => _$this._version = version;

  AgentCardBuilder() {
    AgentCard._defaults(this);
  }

  AgentCardBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _capabilities = $v.capabilities.toBuilder();
      _defaultInputModes = $v.defaultInputModes.toBuilder();
      _defaultOutputModes = $v.defaultOutputModes.toBuilder();
      _description = $v.description;
      _documentationUrl = $v.documentationUrl;
      _iconUrl = $v.iconUrl;
      _name = $v.name;
      _provider = $v.provider?.toBuilder();
      _securityRequirements = $v.securityRequirements?.toBuilder();
      _securitySchemes = $v.securitySchemes?.toBuilder();
      _signatures = $v.signatures?.toBuilder();
      _skills = $v.skills.toBuilder();
      _supportedInterfaces = $v.supportedInterfaces.toBuilder();
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentCard other) {
    _$v = other as _$AgentCard;
  }

  @override
  void update(void Function(AgentCardBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentCard build() => _build();

  _$AgentCard _build() {
    _$AgentCard _$result;
    try {
      _$result = _$v ??
          _$AgentCard._(
            capabilities: capabilities.build(),
            defaultInputModes: defaultInputModes.build(),
            defaultOutputModes: defaultOutputModes.build(),
            description: BuiltValueNullFieldError.checkNotNull(
                description, r'AgentCard', 'description'),
            documentationUrl: documentationUrl,
            iconUrl: iconUrl,
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'AgentCard', 'name'),
            provider: _provider?.build(),
            securityRequirements: _securityRequirements?.build(),
            securitySchemes: _securitySchemes?.build(),
            signatures: _signatures?.build(),
            skills: skills.build(),
            supportedInterfaces: supportedInterfaces.build(),
            version: BuiltValueNullFieldError.checkNotNull(
                version, r'AgentCard', 'version'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'capabilities';
        capabilities.build();
        _$failedField = 'defaultInputModes';
        defaultInputModes.build();
        _$failedField = 'defaultOutputModes';
        defaultOutputModes.build();

        _$failedField = 'provider';
        _provider?.build();
        _$failedField = 'securityRequirements';
        _securityRequirements?.build();
        _$failedField = 'securitySchemes';
        _securitySchemes?.build();
        _$failedField = 'signatures';
        _signatures?.build();
        _$failedField = 'skills';
        skills.build();
        _$failedField = 'supportedInterfaces';
        supportedInterfaces.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AgentCard', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
