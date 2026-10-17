# GemArc 5D Flutter — Verified Deliverable State

## Environment

- Flutter executable: not installed; Flutter compilation / analyze / widget tests **not run**.
- Dart executable: not installed; Dart VM tests **not run**.
- Container internet DNS: unavailable; no dependency download.
- Local Python static model/trace validator: **passed** (1,323 assertions).
- Archive build: see release manifest.

## Deliverable status

| Requirement | Current state |
|---|---|
| Flutter Material 3 app entry, navigation, 5 workspaces | Source implemented, runtime unverified |
| 2D independent node/edge hit testing, focus | Source implemented, runtime unverified |
| 3D Building/Floor/Room | Flutter software-projected 3D (not GPU engine), runtime unverified |
| Architecture gem glyph differentiation | Source implemented (3D only), runtime unverified |
| Workflow pan/zoom, ports validation, move, add, connect, import/export | Source implemented, runtime unverified |
| 4D timeline, step, play/pause | Source implemented, runtime unverified |
| 5D bounded deterministic guard engine, compensation, baseline diff | Source implemented; **independent Python model semantics passed** |
| 16 domain FSM / 144 Case specs | JSON generated, 144 terminal checks passed |
| Tests | Flutter unit + widget source added; **not run** |
| Mobile native platform skeleton | Bootstrap script generates with installed Flutter SDK; not precompiled |
| LINE MINI App | Optional LIFF SDK bridge source; no actual channel / auth test |
| Real source-backed Feliture server topology | Unknown; only public site evidence and explicit Hypothetical |
| Graphology/Sigma/Dexie/Dagre and formal BPMN parser | Not implemented in Flutter port |
| Native graph DB and durable sync | Not implemented |
| 90% branch coverage | **Not measured** for Flutter |
| IPA reference docs | 30 Japanese-language Markdown specs, traceability XLSX |

## Correctness boundaries

Static checks are **not equivalent to compilation**. Python performs an independent walk of declarative Rule graphs and checks expected terminals; production Flutter behavior may still differ until Dart and integration tests pass. The preserved `legacy-web/` is an independently runnable prior JS preview and must not be mistaken for compiled Flutter output.
