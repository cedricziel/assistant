## ADDED Requirements

### Requirement: ApiClient intercepts 401 responses with refresh-once-then-deactivate

The Flutter `ApiClient` (`app/lib/api/api_client.dart`) SHALL register a Dio interceptor that, on any response with status `401`, MUST attempt exactly one OAuth2 refresh per failed request and on persistent failure MUST deactivate the active context. The interceptor MUST NOT enter a refresh loop.

#### Scenario: First 401 triggers a single refresh and retry

- **WHEN** any API request to the assistant server returns HTTP 401 AND the active context has OAuth2 credentials with a valid refresh token AND the request has not already been retried
- **THEN** the interceptor SHALL call `OAuthService.refresh(refreshToken, clientId)`, persist the new credentials to the context (via `contextsProvider.notifier.saveContext`), update the bearer header on the in-flight request, mark the request with an `x-retried` flag in `RequestOptions.extra`, and re-issue the request, returning the retry's response to the caller

#### Scenario: Refresh failure deactivates the active context

- **WHEN** an API request returns HTTP 401 AND the refresh attempt fails (network error, server rejected the refresh token, or the context has no refresh token)
- **THEN** the interceptor SHALL call `activeContextProvider.notifier.deactivate()` AND propagate the original 401 `DioException` to the caller via `handler.next(err)`

#### Scenario: Already-retried request does not refresh again

- **WHEN** a request returns HTTP 401 AND `RequestOptions.extra['x-retried']` is `true`
- **THEN** the interceptor SHALL NOT attempt another refresh AND SHALL call `activeContextProvider.notifier.deactivate()` AND propagate the 401 to the caller

### Requirement: Concurrent 401s share a single refresh attempt

When multiple in-flight requests fail with 401 simultaneously, the interceptor SHALL coalesce them onto a single refresh call (single-flight). All waiting requests MUST observe the same refresh outcome.

#### Scenario: Two parallel 401s, refresh succeeds

- **WHEN** two requests A and B both return 401 within a small window AND a refresh is already in flight when B's error handler runs
- **THEN** B SHALL await the same refresh future as A AND on success both A and B SHALL retry once with the new bearer token

#### Scenario: Two parallel 401s, refresh fails

- **WHEN** two requests A and B both return 401 AND the shared refresh attempt fails
- **THEN** `deactivate()` SHALL be called exactly once AND both A and B SHALL receive their original 401 errors

### Requirement: Router redirects to login after deactivation

The router (`app_router.dart`) SHALL redirect to `/login` whenever the active context becomes null while the user is on a protected route. This requirement is satisfied by the existing redirect rule at `app_router.dart:128-130`; this spec ratifies that the interceptor's `deactivate()` call MUST be the trigger for that redirect when 401 recovery fails.

#### Scenario: Interceptor deactivation triggers login redirect

- **WHEN** the interceptor calls `activeContextProvider.notifier.deactivate()` while the user is on `/chat` or any other authenticated route
- **THEN** the next router redirect evaluation SHALL produce `/login` AND the user SHALL be navigated there without manual interaction
