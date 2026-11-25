import 'package:assistant_api/assistant_api.dart';

/// The list-row view of an integration the server returned in full.
IntegrationSummary integrationSummaryOf(IntegrationDetail detail) =>
    IntegrationSummary(
      (b) => b
        ..id = detail.id
        ..layer = detail.layer
        ..slug = detail.slug
        ..name = detail.name
        ..connectUrl = detail.connectUrl
        ..catalogEntryId = detail.catalogEntryId
        ..authKind = detail.authKind
        ..credentialMode = detail.credentialMode
        ..trust = detail.trust
        ..enabled = detail.enabled
        ..status = detail.status
        ..hasCredential = detail.hasCredential
        ..ownerUserId = detail.ownerUserId
        ..spaceId = detail.spaceId
        ..createdAt = detail.createdAt
        ..updatedAt = detail.updatedAt,
    );
