# Third-party material

First-party Musubi source is dual licensed under MIT or Apache-2.0. This choice
does not replace the licenses or provenance of retained upstream material.

- `scripts/vendor/unicode-16.0.0/CaseFolding.txt` is Unicode 16.0 data. Its
  original [license](scripts/vendor/unicode-16.0.0/LICENSE.txt) and
  [source notice](scripts/vendor/unicode-16.0.0/README.md) are retained.
- Safe 1.4.1 compiler artifacts and source-derived fixtures retain the reviewed
  upstream notices embedded in their metadata. Source and deployment pins, npm
  integrity and source-byte comparison are defined by
  [ADR 0004](docs/adr/0004-versioned-safe-adapter.md).
- Fixed compiler evidence under `external-acceptance/` retains its recorded
  source, compiler and artifact provenance. It does not attest that a separate
  checkout was built from an observed revision.
- Cargo dependencies retain their respective licenses. The prepared CLI SBOM
  describes its actual runtime dependency graph; test-only and build-only
  dependencies do not become shipped runtime components.
