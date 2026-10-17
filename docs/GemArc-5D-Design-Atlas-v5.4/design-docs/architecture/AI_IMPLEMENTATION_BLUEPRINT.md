# AI による Flutter + Full-Stack Microservices 開発実行仕様（承認対象）

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
