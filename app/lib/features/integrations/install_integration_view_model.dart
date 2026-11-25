import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import 'caller_roles.dart';
import 'integration_failure.dart';

const _maxSlugLength = 32;
const _maxNameLength = 100;

/// What one install sheet may offer; build it with [installArgsForEntry] or
/// [installArgsForCustom].
class InstallArgs {
  const InstallArgs({
    required this.entry,
    required this.layers,
    required this.spaces,
  });

  /// The catalog entry to install, or null for a custom URL.
  final CatalogEntrySummary? entry;

  /// Layers the caller may install at, never empty.
  final List<IntegrationLayer> layers;

  /// Spaces the caller administers, for the space layer.
  final List<SpaceSummary> spaces;

  @override
  bool operator ==(Object other) =>
      other is InstallArgs &&
      other.entry == entry &&
      listEquals(other.layers, layers) &&
      listEquals(other.spaces, spaces);

  @override
  int get hashCode =>
      Object.hash(entry, Object.hashAll(layers), Object.hashAll(spaces));
}

/// The install options for [entry], or null when it can be installed nowhere.
InstallArgs? installArgsForEntry(CatalogEntrySummary entry, CallerRoles roles) {
  final spaces = roles.manageableSpaces;
  final layers = [
    for (final layer in entry.installableLayers)
      if (layer != IntegrationLayer.space || spaces.isNotEmpty) layer,
  ];
  if (layers.isEmpty) return null;
  return InstallArgs(entry: entry, layers: layers, spaces: spaces);
}

/// The install options for a custom URL, or null when the caller has none.
///
/// Org admins may always add one at the org layer; the space and user layers
/// need the policy to allow custom URLs.
InstallArgs? installArgsForCustom(
  IntegrationPolicyResponse policy,
  CallerRoles roles,
) {
  final spaces = roles.manageableSpaces;
  final layers = [
    if (roles.isOrgAdmin) IntegrationLayer.org,
    if (policy.allowCustomUrls && spaces.isNotEmpty) IntegrationLayer.space,
    if (policy.allowCustomUrls && policy.allowUserIntegrations)
      IntegrationLayer.user,
  ];
  if (layers.isEmpty) return null;
  return InstallArgs(entry: null, layers: layers, spaces: spaces);
}

final _nonSlugChars = RegExp(r'[^a-z0-9]+');
final _edgeHyphens = RegExp(r'^-+|-+$');

/// `curated/linear-app-mcp` becomes `linear-app-mcp`: lower-case, single
/// hyphens, shortened to fit. Mirrors the server's default.
String slugFromCatalogId(String id) {
  final slug = id
      .split('/')
      .last
      .toLowerCase()
      .replaceAll(_nonSlugChars, '-')
      .replaceAll(_edgeHyphens, '');
  final cut = slug.length > _maxSlugLength
      ? slug.substring(0, _maxSlugLength)
      : slug;
  return cut.replaceAll(_edgeHyphens, '');
}

final _kebab = RegExp(r'^[a-z0-9]+(-[a-z0-9]+)*$');

class InstallState {
  const InstallState({
    required this.args,
    required this.layer,
    this.spaceId,
    this.slug = '',
    this.name = '',
    this.connectUrl = '',
    this.headerName = '',
    this.headerPrefix = '',
    this.credentialMode = CredentialMode.shared,
    this.trust = IntegrationTrust.confirm,
    this.authKind,
    this.takenSlug,
    this.submitting = false,
    this.error,
    this.created,
  });

  final InstallArgs args;
  final IntegrationLayer layer;
  final String? spaceId;
  final String slug;
  final String name;
  final String connectUrl;
  final String headerName;
  final String headerPrefix;
  final CredentialMode credentialMode;
  final IntegrationTrust trust;

  /// The auth kind the user picked, for entries that do not say.
  final IntegrationAuthKind? authKind;

  /// A slug the server reported as already used.
  final String? takenSlug;
  final bool submitting;
  final String? error;

  /// The installed integration, once the request succeeded.
  final IntegrationDetail? created;

  bool get isCustom => args.entry == null;

  /// The auth kind the catalog entry states, or null when it does not say.
  IntegrationAuthKind? get _catalogAuthKind => switch (args.entry?.auth.kind) {
    CatalogAuthKindResponse.none => IntegrationAuthKind.none,
    CatalogAuthKindResponse.header => IntegrationAuthKind.header,
    CatalogAuthKindResponse.oauth => IntegrationAuthKind.oauth,
    CatalogAuthKindResponse.mixed => IntegrationAuthKind.mixed,
    _ => null,
  };

  /// The user must pick none, header or OAuth: the entry does not say, or the
  /// URL is custom.
  bool get needsAuthChoice => _catalogAuthKind == null;

  IntegrationAuthKind? get effectiveAuthKind =>
      needsAuthChoice ? authKind : _catalogAuthKind;

  /// Header auth whose header the catalog does not name.
  bool get needsHeaderName =>
      (effectiveAuthKind == IntegrationAuthKind.header ||
          effectiveAuthKind == IntegrationAuthKind.mixed) &&
      args.entry?.auth.headerName == null;

  /// User-layer integrations always use the owner's own credential.
  bool get showsCredentialMode => layer != IntegrationLayer.user;

  String? get slugError {
    if (slug.isEmpty) return 'Enter a slug.';
    if (slug.length > _maxSlugLength) {
      return 'Use at most $_maxSlugLength characters.';
    }
    if (!_kebab.hasMatch(slug)) {
      return 'Use lower-case letters, digits and single hyphens.';
    }
    if (slug == takenSlug) {
      return 'This slug is already used here. Choose another.';
    }
    return null;
  }

  String? get nameError {
    final trimmed = name.trim();
    if (trimmed.isEmpty) return 'Enter a name.';
    if (trimmed.length > _maxNameLength) {
      return 'Use at most $_maxNameLength characters.';
    }
    return null;
  }

  String? get urlError {
    if (!isCustom) return null;
    final uri = Uri.tryParse(connectUrl.trim());
    final valid =
        uri != null &&
        (uri.scheme == 'http' || uri.scheme == 'https') &&
        uri.host.isNotEmpty;
    return valid ? null : 'Enter an http(s) URL.';
  }

  bool get canSubmit =>
      !submitting &&
      created == null &&
      slugError == null &&
      nameError == null &&
      urlError == null &&
      effectiveAuthKind != null &&
      (!needsHeaderName || headerName.trim().isNotEmpty) &&
      (layer != IntegrationLayer.space || spaceId != null);

  InstallState copyWith({
    IntegrationLayer? layer,
    Object? spaceId = _keep,
    String? slug,
    String? name,
    String? connectUrl,
    String? headerName,
    String? headerPrefix,
    CredentialMode? credentialMode,
    IntegrationTrust? trust,
    IntegrationAuthKind? authKind,
    Object? takenSlug = _keep,
    bool? submitting,
    Object? error = _keep,
    IntegrationDetail? created,
  }) => InstallState(
    args: args,
    layer: layer ?? this.layer,
    spaceId: identical(spaceId, _keep) ? this.spaceId : spaceId as String?,
    slug: slug ?? this.slug,
    name: name ?? this.name,
    connectUrl: connectUrl ?? this.connectUrl,
    headerName: headerName ?? this.headerName,
    headerPrefix: headerPrefix ?? this.headerPrefix,
    credentialMode: credentialMode ?? this.credentialMode,
    trust: trust ?? this.trust,
    authKind: authKind ?? this.authKind,
    takenSlug: identical(takenSlug, _keep)
        ? this.takenSlug
        : takenSlug as String?,
    submitting: submitting ?? this.submitting,
    error: identical(error, _keep) ? this.error : error as String?,
    created: created ?? this.created,
  );
}

const Object _keep = Object();

/// Form state and submission for installing one integration.
class InstallIntegrationViewModel extends Notifier<InstallState> {
  InstallIntegrationViewModel(this.args);

  final InstallArgs args;

  @override
  InstallState build() {
    final entry = args.entry;
    // Least privileged first: a personal install is the safest default.
    final layer = [
      IntegrationLayer.user,
      IntegrationLayer.space,
      IntegrationLayer.org,
    ].firstWhere(args.layers.contains);
    final initial = InstallState(
      args: args,
      layer: layer,
      slug: entry == null ? '' : slugFromCatalogId(entry.id),
      name: entry?.name ?? '',
      spaceId: args.spaces.isEmpty ? null : args.spaces.first.id,
    );
    return initial.copyWith(credentialMode: _defaultModeFor(initial));
  }

  void setLayer(IntegrationLayer layer) {
    if (!args.layers.contains(layer)) return;
    state = state.copyWith(layer: layer);
  }

  void setSpace(String spaceId) => state = state.copyWith(spaceId: spaceId);

  void setSlug(String slug) => state = state.copyWith(slug: slug);

  void setName(String name) => state = state.copyWith(name: name);

  void setConnectUrl(String url) => state = state.copyWith(connectUrl: url);

  void setHeaderName(String name) => state = state.copyWith(headerName: name);

  void setHeaderPrefix(String prefix) =>
      state = state.copyWith(headerPrefix: prefix);

  void setCredentialMode(CredentialMode mode) =>
      state = state.copyWith(credentialMode: mode);

  void setTrust(IntegrationTrust trust) => state = state.copyWith(trust: trust);

  void setAuthKind(IntegrationAuthKind kind) {
    final next = state.copyWith(authKind: kind);
    state = next.copyWith(credentialMode: _defaultModeFor(next));
  }

  /// Installs the integration; check [InstallState.created] and
  /// [InstallState.error] afterwards.
  Future<void> submit() async {
    final form = state;
    if (!form.canSubmit) return;
    state = form.copyWith(submitting: true, error: null);
    try {
      final response = await ref
          .read(apiClientProvider)!
          .integrations
          .createIntegration(createIntegrationRequest: _requestFor(form));
      if (!ref.mounted) return;
      state = state.copyWith(submitting: false, created: response.data);
    } catch (error) {
      if (!ref.mounted) return;
      final failure = describeIntegrationFailure(error, slug: form.slug);
      state = state.copyWith(
        submitting: false,
        takenSlug: failure.kind == IntegrationFailureKind.conflict
            ? form.slug
            : null,
        error: failure.kind == IntegrationFailureKind.conflict
            ? null
            : failure.message,
      );
    }
  }

  CredentialMode _defaultModeFor(InstallState form) =>
      form.effectiveAuthKind == IntegrationAuthKind.oauth
      ? CredentialMode.perUser
      : CredentialMode.shared;

  CreateIntegrationRequest _requestFor(InstallState form) {
    final entry = form.args.entry;
    return CreateIntegrationRequest(
      (b) => b
        ..layer = form.layer
        ..spaceId = form.layer == IntegrationLayer.space ? form.spaceId : null
        ..catalogEntryId = entry?.id
        ..connectUrl = entry == null ? form.connectUrl.trim() : null
        ..slug = form.slug
        ..name = form.name.trim()
        ..authKind = form.needsAuthChoice ? form.authKind : null
        ..headerName = form.needsHeaderName ? form.headerName.trim() : null
        ..headerValuePrefix =
            form.needsHeaderName && form.headerPrefix.isNotEmpty
            ? form.headerPrefix
            : null
        ..credentialMode = form.showsCredentialMode ? form.credentialMode : null
        ..trust = form.trust,
    );
  }
}

final installIntegrationViewModelProvider = NotifierProvider.autoDispose
    .family<InstallIntegrationViewModel, InstallState, InstallArgs>(
      InstallIntegrationViewModel.new,
    );
