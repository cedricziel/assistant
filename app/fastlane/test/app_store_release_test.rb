require "minitest/autorun"
require "tmpdir"
require_relative "../lib/app_store_release"

class AppStoreReleaseTest < Minitest::Test
  CONTACT = {
    "REVIEW_CONTACT_FIRST_NAME" => " Ada ",
    "REVIEW_CONTACT_LAST_NAME" => "Lovelace",
    "REVIEW_CONTACT_PHONE" => "+49 30 1234567",
    "REVIEW_CONTACT_EMAIL" => "review@example.com"
  }.freeze

  DEMO = {
    "REVIEW_DEMO_URL" => "https://demo.example.com",
    "REVIEW_DEMO_USER" => "reviewer",
    "REVIEW_DEMO_PASSWORD" => "secret"
  }.freeze

  def test_the_version_comes_from_pubspec_without_the_build_number
    assert_equal "0.1.165", AppStoreRelease.pubspec_version("name: app\nversion: 0.1.165+12\n")
    assert_equal "0.1.165", AppStoreRelease.pubspec_version("version: 0.1.165\n")
  end

  def test_a_pubspec_without_a_version_gives_nil
    assert_nil AppStoreRelease.pubspec_version("name: app\n")
  end

  def test_release_notes_are_read_and_trimmed
    Dir.mktmpdir do |dir|
      path = File.join(dir, "notes.txt")
      File.write(path, "\nFeatures\n- one\n\n")
      assert_equal "Features\n- one", AppStoreRelease.release_notes(path)
    end
  end

  def test_unset_missing_or_empty_notes_give_nil
    assert_nil AppStoreRelease.release_notes(nil)
    assert_nil AppStoreRelease.release_notes("")
    assert_nil AppStoreRelease.release_notes("/does/not/exist.txt")
    Dir.mktmpdir do |dir|
      path = File.join(dir, "notes.txt")
      File.write(path, " \n")
      assert_nil AppStoreRelease.release_notes(path)
    end
  end

  def test_a_new_or_editable_version_can_be_submitted
    refute AppStoreRelease.already_submitted?([])
    refute AppStoreRelease.already_submitted?(["PREPARE_FOR_SUBMISSION"])
    refute AppStoreRelease.already_submitted?(["REJECTED"])
  end

  def test_a_version_in_review_or_released_is_left_alone
    assert AppStoreRelease.already_submitted?(["WAITING_FOR_REVIEW"])
    assert AppStoreRelease.already_submitted?(["IN_REVIEW"])
    assert AppStoreRelease.already_submitted?(["READY_FOR_SALE"])
  end

  def test_review_information_needs_the_whole_contact
    assert_nil AppStoreRelease.app_review_information({})
    assert_nil AppStoreRelease.app_review_information(CONTACT.merge("REVIEW_CONTACT_EMAIL" => " "))
  end

  def test_review_information_without_a_demo_account_has_the_contact_only
    assert_equal(
      {
        first_name: "Ada",
        last_name: "Lovelace",
        phone_number: "+49 30 1234567",
        email_address: "review@example.com"
      },
      AppStoreRelease.app_review_information(CONTACT)
    )
  end

  def test_review_information_with_a_demo_account_tells_review_where_to_sign_in
    info = AppStoreRelease.app_review_information(CONTACT.merge(DEMO))
    assert_equal "reviewer", info[:demo_user]
    assert_equal "secret", info[:demo_password]
    assert_includes info[:notes], "https://demo.example.com"
  end

  def test_a_partial_demo_account_is_left_out
    info = AppStoreRelease.app_review_information(CONTACT.merge(DEMO).merge("REVIEW_DEMO_PASSWORD" => ""))
    refute info.key?(:demo_user)
    refute info.key?(:notes)
  end

  def test_submit_attaches_the_testflight_build_and_submits
    params = AppStoreRelease.submit_options(version: "0.1.166", dry_run: false, notes: nil, review: nil)
    assert_equal "com.cedricziel.assistant.ios", params[:app_identifier]
    assert_equal "0.1.166", params[:app_version]
    assert params[:skip_binary_upload], "the build comes from TestFlight"
    refute params.key?(:build_number), "deliver picks the version's build"
    assert params[:submit_for_review]
    refute params[:automatic_release], "publishing an approved version stays manual"
    assert params[:reject_if_possible]
    refute params.key?(:release_notes)
    refute params.key?(:app_review_information)
  end

  def test_a_dry_run_neither_submits_nor_pulls_back_a_version_in_review
    params = AppStoreRelease.submit_options(version: "0.1.166", dry_run: true, notes: nil, review: nil)
    refute params[:submit_for_review]
    refute params[:reject_if_possible], "deliver cancels any waiting submission before it uploads"
  end

  def test_notes_and_review_information_are_passed_on
    review = { first_name: "Ada" }
    params = AppStoreRelease.submit_options(version: "0.1.166", dry_run: false, notes: "Fixes", review: review)
    assert_equal({ "en-US" => "Fixes" }, params[:release_notes])
    assert_equal review, params[:app_review_information]
  end

  def test_testflight_notes_become_the_changelog
    assert_equal({ skip_waiting_for_build_processing: true }, AppStoreRelease.testflight_options(nil))
    assert_equal "Fixes", AppStoreRelease.testflight_options("Fixes")[:changelog]
  end
end
