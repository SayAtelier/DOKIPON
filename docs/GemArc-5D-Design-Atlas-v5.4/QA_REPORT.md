# QA Report — GemArc 5D Design Atlas v5.4

## Verified in the current environment

| Gate | Result | Evidence |
|---|---|---|
| Syntax / offline bundle | PASS | `node --check` for `scene3d.js`, `app.js`, `prompt1d.js`; bundler generates inline `index.html` |
| 3D/4D/5D regression | **23/23 PASS** | `tests/unified_3d_5d_acceptance.py`, `tests/acceptance-v53-results.json`（同一 v5.4 HTML 上重新执行） |
| 0D/1D/2D integration | **14/14 PASS** | `tests/v54_semantics.py`, `tests/acceptance-v54-results.json` |
| Graph provenance & bindings | PASS | 19 nodes / 26 edges; valid endpoints and independent Node/Edge picking tested |
| 4D & 5D shared 3D scene | PASS, fallback path | 3D scene graph node/edge counts match across modes |
| 5D Guard recalculation | PASS | `success`, `failed`, `recovered` verified with changed inputs |
| Existing 144 model cases | PASS | All returned configured expected terminal in model test; no assertion of product E2E execution |
| 1D prompt dynamics | PASS | Requirement/phase change causes regenerated prompt with FR/BD/DD/Case refs |
| Mobile 390 px | PASS for layout/navigation smoke check | 390 px viewport: root scrollWidth=390, prompt width=362, side menu opens and closes |
| Document data | PASS | 41 embedded docs, including universal v5.4 prompt and UX spec |
| ZIP archive | PASS | Package integrity is verified using CRC test and SHA-256 manifest |

## Not verified / scope boundary

- **NOT_VERIFIED**: WebGL GPU rendering in hardware browsers; headless environment disabled WebGL, so 3D picking was exercised on the explicitly labeled interactive 3D projection fallback. UI fallback is not a replacement for WebGL compatibility qualification.
- **NOT_VERIFIED**: Flutter SDK compilation, Android/iOS simulator or real-device tests, LINE MINI App approval, LIFF production identity, actual Rust/Axum service execution, payments/PSP/production storage.
- **DESIGNED, NOT EXECUTED**: 42 extra backend/mobile/full-stack E2E/Chaos/Security cases. They are design requirements only, not test passes.
- **PARTIAL**: 11-role production-grade gemstone cut registry (current interactive viewer uses octahedral nodes); GPU native Three.js rendering is not implemented as required by the original v3.0 prompt. Automated contrast auditing, Japanese long-copy at 200% text zoom, production-like concurrency/load testing. Current design targets WCAG 2.2 AA and explicitly tracks these as release gates.
- **NOT CLAIMED**: source-based confirmation of Feliture private microservice/data/payment internals. Unknown components remain Designed/Hypothetical.

## Important invariant

0D does not occupy a view, 1D is Prompt, 2D is Diagram; the same canonical graph drives 3D, dynamic 4D and causal 5D. Viewer presentation never decides business transitions. This is a runnable **design atlas**, not a released application.
