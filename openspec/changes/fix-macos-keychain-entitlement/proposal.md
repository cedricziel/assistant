# Fix macOS App Launch Failure Due to Unresolved Keychain Entitlement

## Problem

The macOS app (distributed via GitHub Releases as a Developer ID signed + notarized bundle) **cannot be opened**. macOS returns:

```
Error Domain=NSPOSIXErrorDomain Code=163 "Unknown error: 163"
  → Launchd job spawn failed
```

AMFI (Apple Mobile File Integrity) kernel logs reveal the root cause:

```
AMFI: Code has restricted entitlements, but the validation of its code signature failed.
Unsatisfied Entitlements:
```

The `keychain-access-groups` entitlement in `Release.entitlements` contains unresolved Xcode build variables:

```xml
<key>keychain-access-groups</key>
<array>
    <string>$(AppIdentifierPrefix)$(CFBundleIdentifier)</string>
</array>
```

These `$(...)` placeholders are **not expanded** at build time for entitlements files. For non-sandboxed Developer ID apps, `keychain-access-groups` is a **restricted entitlement** that requires a provisioning profile to satisfy — but GitHub Releases distribution doesn't use provisioning profiles. AMFI therefore rejects the launch.

## Proposed Fix

Remove the `keychain-access-groups` entitlement from both `Release.entitlements` and `DebugProfile.entitlements`.

macOS automatically grants keychain access scoped to the team ID + bundle ID for Developer ID signed apps. The `flutter_secure_storage_darwin` plugin works with this default implicit group — no explicit entitlement is needed.

## Scope

- `app/macos/Runner/Release.entitlements` — remove `keychain-access-groups` entry
- `app/macos/Runner/DebugProfile.entitlements` — remove `keychain-access-groups` entry (same unresolved vars, works today only because sandbox mode changes AMFI validation)

## Verification

1. Build: `flutter build macos --release`
2. Sign + notarize
3. Confirm `codesign -d --entitlements -` no longer shows `keychain-access-groups`
4. Confirm app launches successfully
5. Confirm `flutter_secure_storage` read/write still works (stores/retrieves a test value)
