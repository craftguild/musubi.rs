Pinned Unicode 16.0 data for ownership path casefolding. This vendored data is not handwritten source.

- [CaseFolding.txt](https://www.unicode.org/Public/16.0.0/ucd/CaseFolding.txt): 86,092 bytes;
  SHA-256 `6f1f9c588eb4a5c718d9e8f93b782685e5c7fec872cf05e8e6878053599e09bb`.
- [Unicode license](https://www.unicode.org/license.txt): 1,995 bytes;
  SHA-256 `e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.

Retained verbatim on 2026-09-07. `ownership_unicode.py` uses only C and F mappings (full default,
non-Turkic casefold), verifies the exact table bytes, and leaves unmapped code points unchanged.
Rust uses the same Unicode version through pinned `caseless` 0.2.2. The shared conformance gate must
check all mappings, so upgrading either implementation cannot silently change path collisions.
