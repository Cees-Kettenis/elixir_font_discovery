# Changelog

## 0.2.0 - 2026-08-29

- Resolved `system-ui` through platform user-interface font families before falling back to the generic sans-serif family.
- Returned `:unsupported_font` for variable fonts whose selected variation cannot be represented by unchanged font-file bytes.
- Distinguished unavailable font sources and malformed installed fonts from missing families.
- Recalculated the whole-font checksum when extracting a standalone face from a TrueType collection.

## 0.1.1 - 2026-08-28

- Allowed RustlerPrecompiled 0.8 so applications can share the dependency with libraries such as `resvg`.

## 0.1.0 - 2026-08-28

- Added installed-font discovery through Fontconfig, CoreText, and DirectWrite.
- Added generic CSS family matching and weight/style selection.
- Added precompiled NIF releases for supported Linux, macOS, and Windows targets.
