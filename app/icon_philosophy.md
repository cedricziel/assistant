# App icon

The icon is an "ai" monogram. A ring forms the bowl of a lowercase "a", a
vertical stem closes it, and a white glowing dot above the stem doubles as the
dot of an "i".

## Palette

- Strokes: linear gradient from blue `#4C8DFF` to violet `#8B6CF0`.
- Dot: white, with a glow.
- Background: navy radial gradient from `#1B1F4A` at the centre to `#080C1A`
  at the edges.

## Why a monogram

The icon has to read at 16px in a favicon and the macOS menu bar. The previous
artwork, a fine-line node constellation, turned to noise at that size. Two
thick shapes and one dot survive the downscale.

## Constraints when editing

- Keep strokes thick. Thin lines vanish in the small sizes.
- Keep the glyph inside the central 80% safe zone of `icon_source.svg`.
  The maskable PWA icons and platform masks crop everything outside it.
- Keep `icon_glyph.svg` pure black on transparent. The tray icon is a macOS
  template image: macOS draws it from the alpha channel alone and tints it to
  match the menu bar.

After editing, run `make icons` (see `README.md`).
