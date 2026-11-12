# CLI banner

`version-banner.txt` spells `musubi.rs` in Small Slant. Generate or verify it
with the `banner` operation of the verified standards engine fixed in
[standards/engine.json](../../../standards/engine.json). The engine owns its
renderer, pinned font data and output normalization.

Embed the generated ASCII text with `include_str!`. Normal product builds and
runtime need no font, Node.js or FIGlet implementation. Preserve one final newline
and the engine's canonical spacing; use its nonmutating check when updating.

Font attribution: [SmSlant by Glenn Chappell](https://github.com/cmatsuoka/figlet/blob/master/fonts/smslant.flf).
The fixed standards engine supplies the font data; this product does not vendor it.
