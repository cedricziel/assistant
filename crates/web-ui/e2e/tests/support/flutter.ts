import type { Page } from "@playwright/test";

/**
 * Builds Flutter's semantics tree by clicking its "Enable accessibility"
 * placeholder.
 *
 * The click lands on the placeholder's centre. Flutter picks a mobile
 * enabler when the user agent says Android (the Pixel 7 project on a Linux
 * host), and that enabler ignores clicks more than a pixel off centre. On a
 * macOS host the same project reports `MacIntel` and gets the desktop
 * enabler, which takes any click.
 */
export async function enableAccessibility(page: Page) {
  const placeholder = page.getByRole("button", {
    name: "Enable accessibility",
  });
  await placeholder.evaluate((el) => {
    const rect = el.getBoundingClientRect();
    el.dispatchEvent(
      new MouseEvent("click", {
        bubbles: true,
        cancelable: true,
        clientX: rect.left + rect.width / 2,
        clientY: rect.top + rect.height / 2,
      }),
    );
  });
  // The mobile enabler builds the tree after a short delay.
  await placeholder.waitFor({ state: "detached" });
}
