from pathlib import Path
import json, itertools, collections
p=Path(__file__).parent
out=p/'design-docs/architecture';out.mkdir(parents=True,exist_ok=True)
contract={
 'schema':'gemarc.scene/v2','version':'5.3','coordinateSystem':{'handedness':'left','x':'right','y':'up','z':'forward','unit':'logical-design-unit','cameraForward':'positive-z','projection':'LHC explicit view projection; conversion boundary for RH engines'},
 'canonicalGraphRef':'gemarc.architecture/v1','roomsAreVirtual':True,
 'scene':{'floorIds':['experience','domain','infrastructure'],'viewModes':['3d','4d','5d'], 'singleGraphRevision':True},
 'binding':{'id':'binding:event:gateway','flowNodeId':'gateway','graphNodeId':'f-transport','verification':'Designed','mappingEvidence':'Explicit design mapping, not a runtime observation'},
 'interaction':{'pickPriority':['node','edge','room','floor'],'edgePickTolerancePx':14,'nodePickTolerancePx':24,'selectionSemantics':'shared across all dimensions','focusOpacityIsPresentationOnly':True},
 'exampleFrame':{'id':'frame:CASE-CHECKOUT-003:02','caseId':'CASE-CHECKOUT-003','sequence':2,'logicalTimeMs':1000,'graphNodeId':'f-checkout-state','graphEdgeIds':[], 'ruleId':'validate-b','animation':'isolate','stateDelta':{'paymentStatus':{'before':'pending','after':'unknown'}},'evidenceGrade':'Hypothetical'},
 'causal':{'causalRuleRef':'gemarc.causal/v1','guardLanguage':'restricted deterministic AST (no eval)','maxSteps':60,'branchModes':['normal','abnormal','boundary','recovery','composite'],'counterfactualSameWorld':True,'compensationNeverAssumesExternalCommit':True},
 'a11y':{'readableNodeEdgeInspector':True,'nonColorStatusText':True,'fallbackDiagram2d':True,'reduceMotion':True},
 'acceptance':{'graphIdsStableInAllViews':True,'replayDeterministic':True,'5dBranchChangesAfterInput':True,'leftHandedOrientationTest':True,'noInventedEdges':True}
}
(p/'reference-models/unified_scene_contract.json').write_text(json.dumps(contract,ensure_ascii=False,indent=2),encoding='utf8')
bindings={'schema':'gemarc.causal-spatial-binding/v1','coordinateSystem':'LHC','flowNodeToGraphNode':{'gateway':'f-transport','webclient':'f-web','catalog':'f-catalog-repo','inventory':'f-checkout-state','payment':'f-payment-boundary','orders':'f-orders-ui','notifier':'f-orders-ui','account':'f-session','store':'f-storage'},'bindingGrade':'Designed','unboundPolicy':'display-unbound; never create graph nodes/edges implicitly'}
(p/'reference-models/causal_spatial_bindings.json').write_text(json.dumps(bindings,ensure_ascii=False,indent=2),encoding='utf8')
md={}
md['UNIFIED_3D_4D_5D_CONTRACT.md']='''# 統合 3D／4D／5D アーキテクチャ詳細設計書

> 適用：Feliture Flutter Commerce の **設計モデル**。未提供の製品ソースを実装済みと見なさない。参考：IPA 共通フレーム2013（文書構成）、非機能要求グレード2018（品質分類）。公式認証ではない。

## 1. 必須不変条件

- **0D**：根拠、信頼度、出典。**1D**：型付き有向／無向、多重／自己ループの Node–Edge。**2D**：UML、ER、BPMN、DFD、グラフの正規表記。
- **3D = Canonical Graph + Semantic Spatial Model**：同じ Node/Edge の stable ID を 3D の Room/Floor/Building に配置する。Edge はメッシュ／ルートを持つ選択可能な実体。Room は閲覧用の仮想区分であり、実ネットワークと混同しない。
- **4D = 3D + Event/Time/State**：3D Scene を置換しない。時刻 t の graph ID に状態差分、エフェクト、方向、遅延（論理時間）を重ねる。seek、pause、step、resume、snapshot、deterministic replay を提供。
- **5D = 4D + Intervention/Cause/Counterfactual**：Normal baseline と Alternative scenario を同じ graphRevision/sceneId にバインドし、Guard、異常注入、責任主体、復旧・補償、結果を比較する。演出の変更だけでは 5D にならない。
- WorldModel／GraphModel／EventModel／CausalModel は正、ScenePresentation（座標、色、透明度、カメラ）は従。Focus やドラッグ操作は業務判断に反映しない。

## 2. 左手座標系契約（LHC）

| 軸 | 正方向 | 例 |
|---|---|---|
| +X | 画面右／空間右 | Room の左右配置 |
| +Y | 上 | Floor の高さ |
| +Z | 前進（カメラ奥側） | 奥行きの増加 |

初期 camera は Z の負側から正の Z 方向を向く。Yaw は Y 軸回転、Pitch はカメラ上下角、Zoom は camera と target の距離。`forward = normalize(target - eye)`、`right = normalize(cross(worldUp, forward))`、`up = cross(forward, right)`。ポインタ水平ドラッグで Yaw、垂直ドラッグで Pitch、ホイール／ピンチで Zoom。すべての picking は投影後の Node/Edge 幾何に対し計算。画面座標（左上原点）と world y（上方向）を混同しない。

Three.js は通常右手系のため、`Adapter(LHC→RH)` を境界に一度だけ適用する。推奨変換 `RH(x,y,z) = (x,y,-z)`、逆変換も同じ。ただしミラー変換で三角面の winding、front-face culling、normal/tangent、ray cast を検証し、親子 Transform に二重変換を加えない。**無検証のカメラミラー scale(-1,1,1) は禁止。** Flutter Native の 3D Runtime と Web の WebGL は同一 Graph/Scene JSON と投影テストで適合性を検証する。

## 3. Object contract と間接参照

- GraphNode `{id, kind, name, evidenceIds, verification, properties}`。
- GraphEdge `{id, sourceId, targetId, directed, type, condition, ports, evidenceIds}`。平行 Edge は ID ごとに独立した 3D pick object とする。
- SpatialContainer `{id, type: Building|Floor|Room, parentId, label, semanticMapping, virtual: true}`。
- SceneObject `{objectId, graphRef: nodeId|edgeId, localTransform, worldTransform, boundingGeometry, labelAnchor}`。
- CausalBinding `{flowNodeId, graphNodeId, evidenceGrade, explicitMappingEvidence}`。Unknown は **UNBOUND** として表示し、Edge を捏造しない。
- EventFrame `{caseId, graphRevision, sequence, logicalTimeMs, ruleId, nodeId, edgeIds[], stateDelta, eventType, visualEffects[]}`。
- CausalRun `{runId, inputHash, baselineRunId, interventions[], frames[], terminal, invariantResults[], diff[]}`。

バージョン、安定 ID、単位、時刻の基準、禁止値、参照整合性、拡張フィールドを JSON Schema で検証する。契約の機械可読例：`reference-models/unified_scene_contract.json`、`reference-models/causal_spatial_bindings.json`。

## 4. 動的 3D レンダラ（設計）

描画順：`GraphStore → Semantic Scene Mapper → Scene Snapshot(t) → 3D renderer → Object picking → Inspector`。3D/4D/5D で 3D viewport を再生成しても graphRevision は変わらない。描画オブジェクトへの参照は `graphRef` のみを保持し、コピーしたグラフは正としない。

4D `Update(t)` は TimelineEngine の確定 event frames を適用。時間依存の影響：Node 状態色＋ラベル、Edge 光路、メッセージ流速、room 隔離、リトライ待ち、部分障害、回復。アニメーションは frame から派生させ、色だけで結果を表さない。時間進行は `logicalTimeMs` に基づき、ブラウザ fps を業務時間と混同しない。

5D `Run(interventions)` は純粋な Guard/State/Compensation engine から trace を作る。Normal と Fault Injection の並列表示、時間同期、branch fork／merge、差分／失敗ノードへの Focus、復旧不能なケースの安全終端を要求する。実際の支払／通知／LINE メッセージは発火させない。

## 5. ハンドリング・性能

クリック選択：Node と Edge を別個に Ray/Screen-space hit test。前面に近いオブジェクト、Graph Layer、明示 focus の順で曖昧性を解決。ロック・解除・Undo・Compare Focus・Reset を実装。Graph が巨大な場合は depth／node budget／time budget、LOD、GPU instance、virtual labels によって負荷を管理。

WebGL が利用できない環境では 2D 選択可能グラフに降格し、データ／診断を失わない。Reduced motion と keyboard navigation を提供。LINE WebView のメモリ制限で fps／shader 品質を動的に低減。

## 6. 受入条件（設計対象）

| Case ID | 操作 | 期待結果 |
|---|---|---|
| SCENE-001 | 3D でノード選択 | 2D と同じ stable ID・in/out edge 表示 |
| SCENE-002 | 3D で平行 Edge 選択 | 指定 Edge ID のみ選択、端点 Node へ吸収しない |
| SCENE-003 | LHC x 正方向移動 | 既定 camera に対して右へ投影、Z が正なら奥へ |
| SCENE-004 | orbit → reset | world transform の変更なし、camera のみ復帰 |
| SCENE-005 | 4D 再生と seek | 同一 scene の物体に event/state が順次適用 |
| SCENE-006 | 5D Guard 変更 | 同一 scene の branch/terminal/active objects が変化 |
| SCENE-007 | 5D compensation failure | 回復完了を示さず安全な failed terminal |
| SCENE-008 | 再レンダ／比較 | graphRevision 不変、ID 変更なし |
| SCENE-009 | 未対応 flowNodeId | UNBOUND 警告、架空 Node/Edge を作らない |
| SCENE-010 | Three.js RH adapter | triangle winding、ray picking、法線／方向が一致 |

## 7. 表現機能と本番エンジンの区別

Design Atlas はオフライン内蔵 WebGL で実際に 3D オブジェクトと Edge を描画する **設計閲覧用の参照実装**。Flutter 本体の 3D runtime 実装と LINE WebView での実機描画品質は未検証。上記契約を Flutter/OpenGL/WebGPU/Three.js Adapter 等の選定に関わらず満たすこと。
'''
md['AI_IMPLEMENTATION_BLUEPRINT.md']='''# AI による Flutter + Full-Stack Microservices 開発実行仕様（承認対象）

> 対象製品：Feliture Commerce の設計案。非公開実装・DB・既存認証方式は Unknown。承認済み要件が不足する場合は実装前に差分を Issue に切り出し、無根拠にサービスを発明しない。実装完了を宣言できる条件は本章の Gates に限定する。

## A. 目標成果物と責任境界

| 工程 | 実装成果 | アプリケーション責務 | Gate |
|---|---|---|---|
| Requirement | `docs/requirements/*.md`、NFR、RTM | PO・法務・設計者の受入条件 | 全要求に受入基準と Evidence Grade |
| Basic Design | C4/L1–L3、UML、ER、DFD、ADR、脅威モデル | bounded context と依存方向 | Context / Container / Component consistency |
| Detailed Design | `openapi.yaml`、`asyncapi.yaml`、`db/migrations`、DDD interfaces、state charts | BFF と各サービス | Lint／schema／contract test |
| Flutter Client | `apps/flutter_app`、`packages/domain`、`packages/api_client`、`packages/scene_contract` | Web/Android/iOS 共有 | analyze、widget/unit/golden、実機 |
| LINE Client | `apps/line_web` または Flutter Web target + `liff_adapter` | WebView/SDK/公開審査 | 内外ブラウザ、LINE Policy、申請 |
| Backend | BFF、Identity、Catalog、Cart、Inventory、Order、Payment Adapter | サーバ権限／データ整合性 | IT/contract/E2E/chaos/security |
| Infrastructure | Docker Compose 開発環境、IaC、CI/CD、Telemetry | 本番／開発設定分離 | rebuild、rollback、backup restoration |
| QA | 144 model cases + full-stack design case contracts | 観測/計測に基づく受入 | UT/IT/ST/UAT、品質レポート |

## B. 微服務／境界（Design, not Observed）

**推奨初期デプロイ**：BFF + Catalog + Checkout/Order + Identity は独立 deployability を確保しつつ、チーム規模に応じて共通 deploy unit（モジュラモノリス）を選択できる。境界だけを 7 分割して各サービス個別 DB／キューを強制しない。負荷、所有者、変更頻度、データ整合性を満たした単位から切り出す。

1. **Edge/BFF**：匿名／認証 API、correlationId、rate limit、content eligibility、端別レスポンス組み立て。
2. **Identity**：認証トークン検証、LINE `id_token` audience/issuer/expiry、セッションと失効、権限最小化。
3. **Catalog**：公開商品、カテゴリ、SKU/price/version、可視性、日英中 localizations、検索ページング。
4. **Cart**：ユーザー／匿名 cart、version による optimistic concurrency、idempotent mutation、expiry。
5. **Inventory**：available/reserved/committed、reservation TTL、timeout、二重予約防止。
6. **Order**：Order FSM、Saga orchestration、注文一意性、返金/取消に使う ledger references。
7. **Payment Adapter**：tokenized provider reference、authorize/capture/refund/webhook verification、reconcile。カード情報、API secret を client へ返さない。

イベント／信頼境界：OrderCreated／StockReserved／PaymentAuthorized／PaymentUnknown／OrderConfirmed／OrderRejected／CompensationRequested／ReconciliationCompleted。Outbox + idempotent consumer + dead-letter queue + trace ID。プロデューサーとコンシューマーで schema version を固定。Outbox は **at-least-once** 配信であり、end-to-end exactly-once を宣言しない。

## C. 外部 API (OpenAPI 3.1 を正とする)

| Operation ID | Method URL | Auth | Request / Response | Error |
|---|---|---|---|---|
| catalog.list | GET `/v1/products?cursor=&limit=` | optional | cursor、items[{id,sku,price,currency,availability}] | 400,429,503 |
| catalog.detail | GET `/v1/products/{productId}` | optional | productId → product + version | 404,410 |
| cart.update | PATCH `/v1/carts/{cartId}/items/{sku}` | session | `{quantity,expectedVersion}` → `{cartId,version,items}` | 401,409,422 |
| checkout.submit | POST `/v1/checkouts` | auth + `Idempotency-Key` | `{cartId,expectedVersion,paymentMethodRef}` → `{checkoutId,orderId,status}` | 401,409,422,429,503 |
| checkout.status | GET `/v1/checkouts/{id}` | owner | authoritative status | 403,404,503 |
| orders.list | GET `/v1/orders?cursor=` | owner | order summaries | 401,403,429 |
| orders.detail | GET `/v1/orders/{id}` | owner | order timeline + possible next actions | 401,403,404 |
| identity.liff | POST `/v1/sessions/line` | LIFF ID token | `{idToken,nonce}` → short-lived session | 401,403,429 |

- すべての書き込み：`Idempotency-Key`、期限、主体、request fingerprint、衝突の HTTP 409。認証と所有者認可はサーバで判定。
- 失敗 body: `{type,title,status,code,detail,traceId,retryable}`（RFC 9457 Problem Details の拡張設計）。個人情報／秘密／決済情報を error に含めない。
- Money: `{amountMinor:int64,currency:ISO4217}`；浮動小数に業務金額を持たせない。時刻 UTC RFC3339。ページング cursor 方式。可変 enum の未知値に防御的 fallback。

## D. DB/ER (PostgreSQL / data ownership)

| aggregate | key | major columns | constraint |
|---|---|---|---|
| product | product_id | published, category_id, version | version >= 1 |
| sku | sku_id | product_id, currency, amount_minor, stock_policy | amount_minor >= 0 |
| cart | cart_id | owner_id/null, version, expires_at | optimistic lock |
| cart_item | cart_id + sku_id | quantity, unit_amount_snapshot | quantity > 0 |
| inventory_reservation | reservation_id | sku_id, order_id, quantity, status, expires_at | no negative availability |
| checkout | checkout_id | idempotency_key_hash, cart_id, state, trace_id | one active fingerprint per actor/key |
| order | order_id | buyer_id, currency, amount_minor, status, version | order status FSM |
| payment_attempt | payment_attempt_id | provider_ref, order_id, status, captured_minor | secrets not stored here |
| outbox | event_id | aggregate_id, topic, schema_version, payload, published_at | durable event, retryable |

各サービスは自身の schema のみ所有。クロスサービスの FK は付けず ID/event によって整合。保持期間、暗号化、削除請求・監査の対象は別途リスク評価で決定。

## E. 状態機と Saga invariants

`New → Reserving → AwaitingPayment → Confirming → Confirmed`。
- 在庫無 → `Rejected`、支払拒否 → reservation release（結果記録）。
- 決済 timeout → `PaymentUnknown` → provider reconcile。**照会結果不明では再課金しない**。
- 支払確定後に Order DB が失敗 → durable recovery/reconciliation; 安易に Success としない。
- 返金・二重通知／重複 webhook は idempotency key と provider event ID で処理。
- 期限切れ予約、同時購入、失敗した補償は retry → DLQ → human-review。操作者／時刻／trace を残す。

## F. Flutter / LINE / Scene Adapter

Dart packages: `app_shell`, `design_system`, `domain_models`, `usecases`, `api_contract`, `platform_adapters`, `graph_core`, `scene_3d`, `event_timeline`, `causal_engine`, `workflow_semantics`。各 package は buildable とし、依存は Presentation → UseCase → Domain ← Adapter の方向。LINE は Flutter Web + JS LIFF Bridge、iOS/Android は platform plugins、Web は本番 HTTPS 上で稼働。

`graph_core` は browser/Flutter 共通 JSON の stable ID・Edge structure に従う。`scene_3d` は WebGL/OpenGL 等 runtime adapter、**LHC (+X right,+Y up,+Z forward)** 入出力だけを外部公開。2D/3D/4D/5D は同一 immutable graph revision を使用。4D は timestamped scene changes、5D は Guards/causal intervention/compensation により 4D trace を生成。演出のみの代替不可。

Flutter Web の検索／SEO／初期読込は実機で評価。LINE MINI App は WebView と外部ブラウザで動作しない API があるため capability detection／fallback。商品可否／年齢・広告表現／規約適合は法務判定までは「要審査」とする。

## G. 開発環境・運用設計

`infra/compose.yaml`：BFF、サービス、Postgres、broker、OTel collector、mock provider。`infra/terraform`：network、TLS、secret manager、private DB、backup、rate limit、logs、alerting。マイグレーションは expand → dual-read/write where required → contract（compat window と rollback 手順必須）。MCP/agent tool は最小権限、実決済や通知は環境フラグで禁じる。CI：schema-lint → backend UT → Flutter analyze/test → contract IT → E2E → SAST/DAST/secret scan → packaging → signed artifact → staging → canary → rollback verification。環境差異は config で解決し、コード分岐に埋め込まない。

### 品質閾値（設計目標。未計測）

- コア domain / state / graph / contract コードの statement + branch coverage **90%以上**。除外は明示。
- 同一 Case 入力・Graph Revision で trace checksum 一致。Graph ID/Edge ID 参照は 100% 一意・存在。
- API p95、初期描画、3D fps、バッテリー、サーバ費用の閾値は機種／規模別に要件定義で決定（デフォルトの架空数字を合格値にしない）。
- Web+Mobile+LINE の主要 Case は本体アプリ画面を含めた end-to-end テスト。認可／冪等／不明決済／DLQ／restore を毎回のリリース Gate に含める。

## H. AI 実装フロー（この順序を固定）

1. 公開証拠を証拠等級に分類し、未確認事項一覧・想定業務を明示。未確定の支払／規約／認証／物流条件は default-off。
2. FR/NFR/SEC と ID を確定し、各 Requirement に BD/DD/API/Test/Owner/Status を接続。
3. Graph／Scene／Event／Causal の versioned schemas と invariants を確定。左手系 round-trip fixture を unit test。
4. OpenAPI、AsyncAPI、SQL Migration、Statecharts、責任マトリクスを作成。consumer-driven contract test を作成。
5. Backend domain + mock PSP／LINE、identity/security boundary、outbox/saga と IT/chaos を実装。
6. Flutter shared packages + Web/Android/iOS shell + LIFF adapter を作成。Unit/Widget/Golden/Integration を実施。
7. 3D scene adapter → 4D event bind → 5D counterfactual engine → 144 model cases + full-stack cases を実装・検証。
8. Docker compose + IaC + CI/CD + monitoring + backup/restore + SBOM + release gate を準備。
9. 実機／E2E／policy／security／回復検証の evidence を添付し、**未実行を PASS と書かない**。

### 最終納品ファイルの最低構成

```text
root/
  apps/flutter_app/    apps/line_web/      apps/admin_web/
  packages/domain/     packages/graph_core/  packages/scene_contract/
  packages/timeline/   packages/causal_engine/ packages/workflow/
  services/bff/  services/identity/ services/catalog/ services/cart/
  services/inventory/ services/order/ services/payment_adapter/
  contracts/openapi.yaml contracts/asyncapi.yaml contracts/scene.schema.json
  db/migrations/ infra/compose.yaml infra/terraform/
  docs/requirements/ docs/basic/ docs/detailed/ docs/security/ docs/operations/
  tests/unit/ tests/integration/ tests/contract/ tests/e2e/ tests/chaos/
  .github/workflows/quality.yml  README.md  LICENSE  QUALITY_REPORT.md
```

完成判定：上記ファイルが存在するだけでなく、各環境で実行可能かつ自動化テスト evidence が保存された状態。設計アトラスの動作は製品構築の代替証拠ではない。
'''
md['API_EVENT_DATA_CONTRACTS.md']='''# API／イベント／ER 詳細設計（サービス境界）

> 全て目標設計。公開サイトから確証のない内部構成を推定しない。機械可読ファイル：`reference-models/commerce_openapi.yaml` と `reference-models/engineering_cases.json`。

## OpenAPI セマンティクス

- Idempotency-Key = UUID、ユーザーと operation にスコープした有効期限付き重複制御。異なる payload を同じキーで送れば 409。
- POST checkout の `202 Accepted` は処理中、`201 Created` は生成確定。失敗や不明支払を 200 success としない。
- `GET /v1/checkouts/{id}` は authoritative status。客户端自动重试 POST checkout 禁止、先 GET または same key replay。
- `Authorization` は server-only permission 判定、LIFF token client 端の権限の代替ではない。
- エラーレスポンスは problem+json。traceId、code、retryable により分岐させる。

## Event / Outbox

| Event | producer | consumer | once strategy | compensation |
|---|---|---|---|---|
| OrderRequested | Order | Inventory | event ID dedup | release pending reserve |
| StockReserved | Inventory | Order | reservation ID uniqueness | stock release |
| PaymentAuthorized | Payment Adapter | Order | provider ref dedup | reverse/refund per provider |
| PaymentUnknown | Payment Adapter | Reconciliation | reconcile polling | never charge twice |
| OrderConfirmed | Order | Notification | outbox+consumer dedup | notification retry only |
| PaymentReconciled | Payment Adapter | Order | version check | settle/compensate accordingly |

Event header `{eventId, schemaVersion, correlationId, causationId, traceparent, occurredAt, aggregateId}`。PII を Broker payload に流さない。Producer は order aggregate と Outbox を同一 DB transaction で commit。Consumer は Inbox key にて重複処理を除去し、順序保証が必要な aggregate は version で制御。

## ER（概念、サービスごとの schema 所有）

```mermaid
erDiagram
  PRODUCT ||--o{ SKU : has
  CART ||--o{ CART_ITEM : contains
  SKU ||--o{ CART_ITEM : references
  ORDER ||--o{ PAYMENT_ATTEMPT : attempts
  ORDER ||--o{ INVENTORY_RESERVATION : reserves
  ORDER ||--o{ OUTBOX_EVENT : produces
```

## Threat model

- spoofed LIFF ID token → issuer/audience/signature/nonce verify。
- webhook forgery/replay → HMAC verify、timestamp window、unique provider event ID。
- cross-user order lookup → owner authorization, deny & log。
- price tampering → authoritative server price, not client subtotal。
- double-commit race → optimistic version / unique key / serializable constraints as needed。
- payment unknown → nonterminal result with reconcile, no blind retry。
- secrets and card data → secret manager, provider tokenization, never logs/UI/cases。
'''
md['CASE_COVERAGE_AND_TRACEABILITY.md']='''# フルスタック正常／異常・組合せ Case 設計

> Model-level 144 Case は Guard/Transition の独立検証用。実決済、サービス障害、Flutter 実機を検証した証拠ではない。追加の E2E Case 群は `reference-models/engineering_cases.json` を正とする。

## 期待する Case レベル

| Test level | target | must assert |
|---|---|---|
| UT | Dart DTO/guards, service domain, scene LHC transform | deterministic, boundary, typed errors |
| Contract | OpenAPI and events schema | backwards compatibility, schema evolution |
| IT | Catalog/Order/Inventory/Payments with mock | transactional invariants, idempotence |
| ST | BFF + services + Flutter Web | E2E states, error hints, fraud policy |
| Mobile UAT | Android/iOS | lifecycle, offline, deep link, accessibility |
| LINE UAT | LIFF WebView + external browser | login cancel, capability fallback, policy |
| Chaos | broker DB payment fault and recovery | no duplicate charge, outbox/reconcile, DLQ |
| 3D–5D | same scene/graph across layers | stable IDs, LHC, trace diff and replay |

## ケースの判断原則

正常、異常、境界、復旧、複合（複数原因）、セキュリティ、ユーザー行動、データ同期を分離。各 Case に `preconditions → injection → expectedState → invariants → testLevel → graphRefs → requirementId → evidence` を記入。CausalRun が success でも server transaction と client UI が不整合なら合格としない。
'''
for name,content in md.items(): (out/name).write_text(content,encoding='utf8')
# Machine-readable minimum API design: keeps security / payment constraints concrete
openapi='''openapi: 3.1.0
info:
  title: Feliture Commerce Target API (Design Only)
  version: 0.1.0
  description: Hypothetical contract for AI engineering; not a reverse-engineered API.
servers:
  - url: https://api.example.invalid
paths:
  /v1/products:
    get:
      operationId: catalog.list
      parameters:
        - in: query
          name: cursor
          schema: {type: string}
        - in: query
          name: limit
          schema: {type: integer, minimum: 1, maximum: 100, default: 20}
      responses:
        '200': {description: Paginated products}
        '429': {$ref: '#/components/responses/Problem'}
  /v1/products/{productId}:
    get:
      operationId: catalog.detail
      parameters:
        - in: path
          name: productId
          required: true
          schema: {type: string}
      responses:
        '200': {description: Product detail}
        '404': {$ref: '#/components/responses/Problem'}
  /v1/checkouts:
    post:
      operationId: checkout.submit
      security: [{bearerAuth: []}]
      parameters:
        - in: header
          name: Idempotency-Key
          required: true
          schema: {type: string, format: uuid}
      requestBody:
        required: true
        content:
          application/json:
            schema: {$ref: '#/components/schemas/CheckoutRequest'}
      responses:
        '201': {description: Confirmed checkout, if synchronous}
        '202': {description: Accepted and pending reconciliation}
        '409': {$ref: '#/components/responses/Problem'}
        '422': {$ref: '#/components/responses/Problem'}
  /v1/checkouts/{checkoutId}:
    get:
      operationId: checkout.status
      security: [{bearerAuth: []}]
      parameters:
        - in: path
          name: checkoutId
          required: true
          schema: {type: string}
      responses:
        '200': {description: Authoritative checkout status}
        '403': {$ref: '#/components/responses/Problem'}
components:
  securitySchemes:
    bearerAuth: {type: http, scheme: bearer}
  schemas:
    CheckoutRequest:
      type: object
      required: [cartId, expectedVersion, paymentMethodRef]
      additionalProperties: false
      properties:
        cartId: {type: string}
        expectedVersion: {type: integer, minimum: 1}
        paymentMethodRef: {type: string, description: Provider token only; never raw card details}
  responses:
    Problem:
      description: RFC 9457 compatible problem response
      content:
        application/problem+json:
          schema:
            type: object
            required: [type, title, status, code, traceId]
            properties:
              type: {type: string}
              title: {type: string}
              status: {type: integer}
              code: {type: string}
              detail: {type: string}
              traceId: {type: string}
              retryable: {type: boolean}
'''
(p/'reference-models/commerce_openapi.yaml').write_text(openapi,encoding='utf8')
# Explicit full-stack design cases beyond original generic 144 guard paths
scenarios=[
 ('checkout','正常提交并生成订单','normal','stock reserved, payment authorized, outbox delivered','Confirmed','no duplicate charge','IT/E2E','FR-APP-04','f-checkout-state'),
 ('checkout','库存不足','abnormal','inventory rejects reservation','Rejected','no payment request','IT/E2E','FR-APP-04','f-checkout-state'),
 ('checkout','并发库存仅余一件','composite','two concurrent buyers reserve last unit','OneConfirmedOneRejected','availability never negative','IT/Chaos','FR-APP-04','f-checkout-state'),
 ('checkout','支付响应丢失但实际上授权成功','composite','provider authorized, HTTP response timeout','PaymentUnknown','never retry new capture until reconcile','IT/Chaos','FR-APP-04','f-payment-boundary'),
 ('checkout','支付拒绝','abnormal','provider decline','Rejected','reservation released','IT/E2E','FR-APP-04','f-payment-boundary'),
 ('checkout','幂等键完全相同重放','boundary','repeat request with same key & fingerprint','OriginalResult','single checkout and payment','Contract/IT','FR-APP-04','f-transport'),
 ('checkout','幂等键复用不同请求','abnormal','same key, changed cart or payment token','Conflict409','no new side effects','Contract/IT','FR-APP-04','f-transport'),
 ('checkout','订单写入失败且已经授权','recovery','payment authorized then order database unavailable','ReconciliationPending','no false Confirmed','Chaos','FR-APP-04','f-orders-ui'),
 ('checkout','补偿调用失败进入死信','composite','stock release fails after payment rejected','DLQHumanReview','no fake recovered state','Chaos','FR-APP-04','f-checkout-state'),
 ('checkout','Provider webhook 重放','security','duplicate signed webhook same provider event ID','ProcessedOnce','idempotent consumer','Security/IT','FR-APP-04','f-payment-boundary'),
 ('checkout','伪造支付回调','security','invalid webhook signature','Unauthorized','no state change','Security','FR-APP-04','f-payment-boundary'),
 ('catalog','分页浏览','normal','valid cursor','Listed','stable pagination','E2E','FR-APP-02','f-catalog-repo'),
 ('catalog','未知 Cursor 与分页越界','boundary','invalid cursor, limit=101','ValidationError','no full-table scan','Contract','FR-APP-02','f-catalog-repo'),
 ('catalog','价格篡改','security','client sends altered subtotal','ServerReprice','server price authoritative','Security/IT','FR-APP-04','f-commerce-backend'),
 ('catalog','被限制内容跨渠道展示','security','LINE channel filters disallowed content','PolicyBlocked','no LINE publish until approved','Policy/E2E','FR-APP-05','f-line'),
 ('cart','离线增加商品并重连','recovery','cached cart edits conflict with server version','ConflictResolution','no silent overwrite','Mobile/E2E','FR-APP-03','f-storage'),
 ('cart','数量零/负/超限','boundary','quantity -1 or 0 or 100000','ValidationError','no negative inventory','UT/Contract','FR-APP-03','f-cart-ui'),
 ('identity','跨用户查询订单','security','session A accesses order B','Forbidden403','no leaked data','Security/E2E','FR-APP-04','f-session'),
 ('identity','LIFF ID token 签名错误','security','forged token','Unauthorized401','no session creation','Security/IT','FR-APP-05','f-line'),
 ('identity','LIFF 取消登录','abnormal','user cancels authorization','GuestFallback','no redirect loop','LINE UAT','FR-APP-05','f-line'),
 ('identity','外部浏览器缺少 LINE API','boundary','liff feature unavailable','Fallback','no unsupported method call','LINE UAT','FR-APP-05','f-line'),
 ('identity','过期 Token 自动刷新','recovery','token expiration on checkout','Reauthenticated','idempotent checkout preserved','Mobile/LINE','FR-APP-05','f-session'),
 ('events','消息至少一次重复投递','boundary','broker redelivers identical event','ProcessedOnce','inbox dedup','IT/Chaos','FR-APP-04','f-commerce-backend'),
 ('events','消费者处理乱序事件','composite','OrderConfirmed arrives before StockReserved','VersionGated','no regression to older status','IT/Chaos','FR-APP-04','f-commerce-backend'),
 ('events','Broker 离线后恢复','recovery','outbox backlog then broker recovers','EventuallyDelivered','no lost committed outbox','Chaos','FR-APP-04','f-commerce-backend'),
 ('mobile','iOS 切后台后支付结果未知','recovery','app suspended mid-authorization','AuthoritativeStatus','does not repeat charge','Mobile E2E','FR-APP-01','f-mobile'),
 ('mobile','Android 旋转／重建组件','boundary','activity recreation','PreservedState','no duplicate submission','Mobile E2E','FR-APP-01','f-mobile'),
 ('mobile','深链到已停售商品','abnormal','product removed after link shared','NotFound','safe catalog redirect','Mobile/LINE','FR-APP-02','f-catalog-ui'),
 ('mobile','无障碍放大字体','boundary','200 percent accessibility font scale','Readable','no clipped critical controls','Accessibility','FR-APP-01','f-shell'),
 ('scene','三维 Node ID 一致','normal','select graph node in 3D then 2D','SameNodeId','graph revision unchanged','Web E2E','FR-5D-03','f-catalog-ui'),
 ('scene','多重 Edge 独立拾取','boundary','parallel edges same endpoints','CorrectEdgeId','no target node auto-select','Web E2E','FR-5D-03','f-transport'),
 ('scene','左手坐标系轴方向','normal','increase +X and +Z positions','CorrectLHC','camera forward positive Z','UT/Web E2E','FR-5D-03','f-app'),
 ('scene','WebGL 不支持','recovery','context creation fails','AccessibleFallback','no data loss','Web/LINE','FR-5D-03','f-app'),
 ('timeline','4D 播放与 seek 同态','normal','play then seek to same logical time','IdenticalSnapshot','same graphRevision','Web E2E','FR-5D-04','f-checkout-state'),
 ('timeline','页面停用/恢复','boundary','document visibility changes during replay','PausedOrRecovered','no time-dependent semantic divergence','Web E2E','FR-5D-04','f-app'),
 ('causal','5D 两种 Guard 因果分支','normal','toggle recoverable flag','DifferentPaths','same scene coordinates','Web E2E','FR-5D-05','f-checkout-state'),
 ('causal','5D 多故障同时注入','composite','stock false and payment timeout','EarliestSafeStop','do not skip compensation','Web E2E','FR-5D-05','f-checkout-state'),
 ('causal','5D 补偿失败','abnormal','recoverable false after failed event','Failed','never mark success','Web E2E','FR-5D-05','f-checkout-state'),
 ('causal','5D 未绑定事件目标','boundary','flowNodeId not in mapping','UnboundWarning','no invented graph link','UT/Web','FR-5D-05','f-app'),
 ('release','备份恢复读写一致','recovery','restore database and replay outbox','Restored','no duplicate fulfillment','Ops/Chaos','FR-APP-04','f-commerce-backend'),
 ('release','Secret 误入仓库','security','secret scanner detects committed token','PipelineBlocked','artifact cannot deploy','CI/Security','FR-APP-01','f-transport'),
 ('release','灰度版本 schema 不兼容','composite','old clients and new API version','RolledBack','no data loss','ST/Ops','FR-APP-01','f-commerce-backend'),
]
cases=[]
for i,(area,title,category,inject,expected,inv,level,req,node) in enumerate(scenarios,1):
 cases.append({'id':f'E2E-{i:03}', 'area':area,'title':title,'category':category,'preconditions':'staging only; sanitized fixtures and mocked external PSP/LINE', 'faultInjection':inject,'expectedState':expected,'invariants':[inv], 'testLevel':level,'requirementId':req,'graphNodeIds':[node],'evidenceGrade':'Designed','verified':False})
(p/'reference-models/engineering_cases.json').write_text(json.dumps({'schema':'gemarc.engineering-cases/v1','cases':cases},ensure_ascii=False,indent=2),encoding='utf8')
print('docs',len(md),'engineering_cases',len(cases),'machines',list(bindings['flowNodeToGraphNode']))
# Update data.js: embed new documents + E2E cases in same offline index; preserve baseline model as-is.
datajs=p/'data.js'
s=datajs.read_text(encoding='utf8');idx=s.index('window.GEMARC_DESIGN_DATA = ');data=json.loads(s[idx+len('window.GEMARC_DESIGN_DATA = '):].rsplit(';',1)[0])
for file in md:
 path='architecture/'+file
 data['docs']=[d for d in data['docs'] if d['id']!=path]
 body=(out/file).read_text(encoding='utf8');data['docs'].append({'id':path,'title':file.replace('.md','').replace('_',' '),'body':body,'size':len(body),'group':'architecture'})
data['engineeringCases']=cases
data['sceneContract']=contract
data['sceneBindings']=bindings
data['meta']['version']='5.3'
data['meta']['name']='Unified 3D/4D/5D · Flutter + Full-Stack Design Atlas'
datajs.write_text('/* Immutable offline design source; Flutter product is NOT compiled by this index. */\nwindow.GEMARC_DESIGN_DATA = '+json.dumps(data,ensure_ascii=False,separators=(',',':'))+';\n',encoding='utf8')
