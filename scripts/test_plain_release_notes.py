import unittest

from plain_release_notes import LIMIT, plain

# The shape release-please writes into a GitHub release.
RELEASE = """## [0.1.13](https://github.com/cedricziel/assistant/compare/v0.1.12...v0.1.13) (2026-09-20)


### Features

* human-readable app name and a launch screen ([#93](https://github.com/cedricziel/assistant/issues/93)) ([2c79bd7](https://github.com/cedricziel/assistant/commit/2c79bd7758b546ba29dce53bd1bbdf1e8c60a59c))
* **kanban:** open, create and edit tasks ([#66](https://github.com/cedricziel/assistant/issues/66)) ([dab4e9d](https://github.com/cedricziel/assistant/commit/dab4e9d74673d1435e1ba8703266d0ac6b911761))


### Bug Fixes

* **auth:** treat an unreadable stored session as signed out ([#126](https://github.com/cedricziel/assistant/issues/126)) ([68a39dd](https://github.com/cedricziel/assistant/commit/68a39dd399efb6cc5efb4d37dd9a3da8c9503785))
"""

EXPECTED = """0.1.13 (2026-09-20)

Features
- human-readable app name and a launch screen
- kanban: open, create and edit tasks

Bug Fixes
- auth: treat an unreadable stored session as signed out"""


class PlainReleaseNotes(unittest.TestCase):
    def test_a_release_becomes_plain_text(self):
        self.assertEqual(plain(RELEASE), EXPECTED)

    def test_no_markdown_is_left(self):
        text = plain(RELEASE)
        for marker in ("#", "**", "](", "[", "`"):
            self.assertNotIn(marker, text, marker)

    def test_links_keep_their_text(self):
        self.assertEqual(
            plain("* see [the docs](https://example.com/docs) first"),
            "- see the docs first",
        )

    def test_emphasis_and_code_lose_their_marks(self):
        self.assertEqual(
            plain("* **bold**, *italic*, _also_ and `code`"),
            "- bold, italic, also and code",
        )

    def test_a_breaking_change_heading_is_kept_as_a_title(self):
        self.assertEqual(
            plain("### ⚠ BREAKING CHANGES\n\n* drop the old API"),
            "⚠ BREAKING CHANGES\n- drop the old API",
        )

    def test_a_reference_that_is_not_the_trailing_one_stays(self):
        self.assertEqual(
            plain("* fix the crash in [#12](https://example.com/12) again"),
            "- fix the crash in #12 again",
        )

    def test_a_parenthesized_reference_in_the_middle_stays(self):
        self.assertEqual(
            plain("* explain ([#12](https://example.com/12)) behavior ([#13](https://example.com/13))"),
            "- explain (#12) behavior",
        )

    def test_lines_about_other_platforms_are_left_out(self):
        # App Review rejects "What's New" that mentions them (guideline 2.3.10).
        self.assertEqual(
            plain(
                "### Bug Fixes\n\n"
                "* no white screen after unlock on iOS and Android\n"
                "* the Windows zip starts again\n"
                "* **windows:** sign the installer\n"
                "* **linux:** package for arm64\n"
                "* the sidebar stays usable in short Mac windows\n"
                "\n### Features\n\n* Android widgets"
            ),
            "Bug Fixes\n- the sidebar stays usable in short Mac windows",
        )

    def test_a_word_that_only_contains_a_platform_name_stays(self):
        self.assertEqual(plain("* resize the Windowsill panel"), "- resize the Windowsill panel")

    def test_windows_followed_by_a_colon_in_the_description_stays(self):
        self.assertEqual(
            plain("* **chat:** restore Mac windows: keep their positions"),
            "- chat: restore Mac windows: keep their positions",
        )

    def test_a_section_about_another_platform_is_left_out(self):
        self.assertEqual(
            plain("### Android\n\n* fix login\n\n### Fixed\n\n* fix the Mac sidebar"),
            "Fixed\n- fix the Mac sidebar",
        )

    def test_empty_and_blank_input_give_nothing(self):
        self.assertEqual(plain(""), "")
        self.assertEqual(plain("\n\n  \n"), "")

    def test_long_notes_are_cut_at_a_line_to_fit_testflight(self):
        lines = "\n".join(f"* item number {i} with some words to fill the line" for i in range(400))
        text = plain(f"### Features\n\n{lines}")
        self.assertLessEqual(len(text), LIMIT)
        self.assertTrue(text.endswith("…"))
        self.assertNotIn("\n\n\n", text)
        self.assertTrue(text.splitlines()[-2].startswith("- item number"))


if __name__ == "__main__":
    unittest.main()
