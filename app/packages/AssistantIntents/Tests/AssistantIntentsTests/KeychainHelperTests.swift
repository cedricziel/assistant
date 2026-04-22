import XCTest

@testable import AssistantIntents

final class KeychainHelperTests: XCTestCase {

  /// Unique service name per test run to avoid cross-contamination with the
  /// real app's Keychain items.
  private let service = "com.cedricziel.assistant.tests.\(UUID().uuidString)"

  private lazy var keychain = KeychainHelper(service: service)

  override func tearDown() {
    // Clean up test Keychain items.
    keychain.delete(key: "assistant_siri_server_url")
    keychain.delete(key: "assistant_siri_auth_token")
    super.tearDown()
  }

  // MARK: - hasCredentials

  func testHasCredentials_returnsFalse_whenNothingStored() {
    XCTAssertFalse(keychain.hasCredentials)
  }

  func testHasCredentials_returnsTrue_whenOnlyServerURLStored() {
    keychain.write(key: "assistant_siri_server_url", value: "https://example.com")
    // No auth token written — server does not require auth.
    XCTAssertTrue(
      keychain.hasCredentials,
      "hasCredentials should be true with only a server URL (no auth token)"
    )
  }

  func testHasCredentials_returnsTrue_whenBothURLAndTokenStored() {
    keychain.write(key: "assistant_siri_server_url", value: "https://example.com")
    keychain.write(key: "assistant_siri_auth_token", value: "tok-123")
    XCTAssertTrue(keychain.hasCredentials)
  }

  func testHasCredentials_returnsFalse_whenServerURLIsEmpty() {
    keychain.write(key: "assistant_siri_server_url", value: "")
    XCTAssertFalse(keychain.hasCredentials)
  }

  // MARK: - Read / Write / Delete

  func testWriteAndRead() {
    keychain.write(key: "assistant_siri_server_url", value: "https://test.local")
    XCTAssertEqual(keychain.serverURL, "https://test.local")
  }

  func testDeleteRemovesValue() {
    keychain.write(key: "assistant_siri_auth_token", value: "secret")
    keychain.delete(key: "assistant_siri_auth_token")
    XCTAssertNil(keychain.authToken)
  }
}
