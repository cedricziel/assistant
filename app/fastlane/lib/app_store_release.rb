# frozen_string_literal: true

# Parameters for the TestFlight upload and the App Review submission of the
# iOS app, kept apart from the Fastfile so they can be tested without fastlane.
module AppStoreRelease
  APP_IDENTIFIER = "com.cedricziel.assistant.ios"

  # States in which an App Store version can still be edited and sent to review.
  SUBMITTABLE_STATES = %w[
    PREPARE_FOR_SUBMISSION READY_FOR_REVIEW DEVELOPER_REJECTED REJECTED METADATA_REJECTED INVALID_BINARY
  ].freeze

  CONTACT_KEYS = %w[REVIEW_CONTACT_FIRST_NAME REVIEW_CONTACT_LAST_NAME REVIEW_CONTACT_PHONE REVIEW_CONTACT_EMAIL].freeze
  DEMO_KEYS = %w[REVIEW_DEMO_URL REVIEW_DEMO_USER REVIEW_DEMO_PASSWORD].freeze

  def self.pubspec_version(pubspec)
    pubspec[/^version:\s*([^\s+]+)/, 1]
  end

  def self.release_notes(path)
    return nil if path.to_s.empty? || !File.exist?(path)

    notes = File.read(path).strip
    notes.empty? ? nil : notes
  end

  # True when the version has been sent to review or gone further: sending it
  # again would pull it back and lose its place in the queue.
  def self.already_submitted?(states)
    states.any? && states.none? { |state| SUBMITTABLE_STATES.include?(state) }
  end

  # The person App Review contacts and, when all of REVIEW_DEMO_* are set, the
  # demo account it signs in to the server with. Nil without the full contact:
  # App Store Connect then keeps what it already has.
  def self.app_review_information(env)
    value = ->(key) { env[key].to_s.strip }
    return nil if CONTACT_KEYS.any? { |key| value.(key).empty? }

    info = {
      first_name: value.("REVIEW_CONTACT_FIRST_NAME"),
      last_name: value.("REVIEW_CONTACT_LAST_NAME"),
      phone_number: value.("REVIEW_CONTACT_PHONE"),
      email_address: value.("REVIEW_CONTACT_EMAIL")
    }
    if DEMO_KEYS.none? { |key| value.(key).empty? }
      info[:demo_user] = value.("REVIEW_DEMO_USER")
      info[:demo_password] = value.("REVIEW_DEMO_PASSWORD")
      info[:notes] = "Sign in to the assistant server at #{value.('REVIEW_DEMO_URL')} with the demo account."
    end
    info
  end

  def self.testflight_options(notes)
    params = { skip_waiting_for_build_processing: true }
    params[:changelog] = notes if notes
    params
  end

  def self.submit_options(version:, dry_run:, notes:, review:)
    params = {
      app_identifier: APP_IDENTIFIER,
      platform: "ios",
      app_version: version,
      # Without a build number, deliver waits for this version's TestFlight
      # build to finish processing and attaches it.
      skip_binary_upload: true,
      skip_screenshots: true,
      skip_metadata: false,
      submit_for_review: !dry_run,
      # Publishing an approved version stays a manual step.
      automatic_release: false,
      # A version still waiting for review has to be pulled back before the
      # next one can take its place.
      reject_if_possible: true,
      force: true,
      precheck_include_in_app_purchases: false,
      run_precheck_before_submit: false,
      submission_information: { export_compliance_uses_encryption: false }
    }
    params[:release_notes] = { "en-US" => notes } if notes
    params[:app_review_information] = review if review
    params
  end
end
