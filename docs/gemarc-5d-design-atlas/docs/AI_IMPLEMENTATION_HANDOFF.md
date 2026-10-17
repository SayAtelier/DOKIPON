# AI 実装引継ぎ契約

正式製品の実装・変更対象は、閲覧用アトラスから分離する。`sourceRevision` を固定し、要求 ID・Node/Edge ID・Case ID と実装ファイルを関連付ける。

要求は `models/architecture.json` の `requirements`、API の起点は DOKIPON リポジトリ内 `contracts/openapi.yaml` とする。次工程で構成・認証・冪等性・支払結果照合・DB 制約・失敗時の補償・運用監視を明細化する。

実装タスクには `taskId`、`requirementIds`、`sourceAndEvidence`、`inputContracts`、`targetModules`、`dependencies`、`normalCases`、`exceptionCases`、`automatedAcceptanceTests`、`rollbackPlan` を必須とする。

**注意**：アトラスのモデル内の「Designed」な通信 Edge や「Hypothetical」な結果は実際に呼び出せる API・永続化スキーマ・サービスの配置を証明しない。テスト環境から本番決済へ送信してはならない。
