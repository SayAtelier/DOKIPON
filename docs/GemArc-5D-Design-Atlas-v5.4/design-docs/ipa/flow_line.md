> 文書種別: IPA 共通フレーム2013・非機能要求グレード2018 を参照した独自成果物。IPA の審査・認証済みではない。
> 対象: GemArc 5D Flutter Workbench。Feliture の公開ページはサンプル入力であり、非公開システム構成の証拠ではない。
> 基準日: 2026-10-17。仕様の実装状態は `../quality/VERIFICATION.md` を正とする。

# LINE 身份接入 — 基本設計・詳細設計対応（line）

- Requirement: `FR-LINE-01` / `NFR-LINE-01`
- Design: `BD-LINE-01`, `DD-LINE-01`
- Model: Directed guarded finite state machine / Max Steps 24 / Verification `Hypothetical`

| Rule ID | Graph Node ID | 検証条件 / 動作 | Next YES | Next NO | Animation |
|---|---|---|---|---|---|
| begin | webclient | 接收LINE 身份接入请求 / patch/terminal | check-a | — | signal |
| check-a | gateway | LIFF 初始化完成 / aOk | check-b | abort-a | branch |
| abort-a | gateway | LIFF 初始化失败 / patch/terminal | end-failed | — | isolate |
| check-b | webclient | 已明确取得授权 / bOk | check-c | compensate | branch |
| compensate | webclient | 执行补偿 / 降级为访客可读模式 / patch/terminal | can-recover | — | reverse |
| can-recover | webclient | 验证恢复前置条件 / recoverable | recovered | end-failed | branch |
| recovered | webclient | 降级为访客可读模式 / patch/terminal | end-recovered | — | rejoin |
| check-c | account | 外部浏览器上下文可识别 / cOk | check-threshold | abort-c | branch |
| check-threshold | account | 可用性/负荷阈值 ≤ 100% / thresholdPercent | commit | abort-threshold | branch |
| abort-threshold | account | 超过设计阈值，拒绝提交 / patch/terminal | end-failed | — | isolate |
| abort-c | account | 外部浏览器功能受限 / patch/terminal | end-failed | — | isolate |
| commit | webclient | 确认业务不变量并完成 / patch/terminal | end-success | — | flow |
| end-success | webclient | 正常完成 / patch/terminal | End | — | settle |
| end-recovered | webclient | 降级恢复完成 / patch/terminal | End | — | settle |
| end-failed | webclient | 安全终止并保留审计 / patch/terminal | End | — | rollback |

### ケース別の条件／期待結果

| ID | 種別 | Trigger | Inputs | Terminal |
|---|---|---|---|---|
| CASE-LINE-001 | normal | 所有守卫成立 | `aOk=True bOk=True cOk=True recoverable=False thresholdPercent=99` | success |
| CASE-LINE-002 | abnormal | LIFF 初始化失败 | `aOk=False bOk=True cOk=True recoverable=False thresholdPercent=99` | failed |
| CASE-LINE-003 | abnormal | 拒绝授权或登录超时 | `aOk=True bOk=False cOk=True recoverable=False thresholdPercent=99` | failed |
| CASE-LINE-004 | abnormal | 外部浏览器功能受限 | `aOk=True bOk=True cOk=False recoverable=False thresholdPercent=99` | failed |
| CASE-LINE-005 | recovery | 降级为访客可读模式 | `aOk=True bOk=False cOk=True recoverable=True thresholdPercent=99` | recovered |
| CASE-LINE-006 | boundary | 补偿已尝试但不得伪称恢复 | `aOk=True bOk=False cOk=True recoverable=False thresholdPercent=99` | failed |
| CASE-LINE-007 | composite | 优先在最早失败守卫安全停止，不越级补偿 | `aOk=False bOk=False cOk=False recoverable=True thresholdPercent=99` | failed |
| CASE-LINE-008 | boundary | 在设计阈值上限内，恪守闭区间约定 | `aOk=True bOk=True cOk=True recoverable=False thresholdPercent=100` | success |
| CASE-LINE-009 | boundary | 100% 为闭区间上限，101% 必须走独立失败路径 | `aOk=True bOk=True cOk=True recoverable=False thresholdPercent=101` | failed |

#### 不変条件
状態の成功は `end-success` 経由のみ。異常 B は補償を記録すること。仕様上の Node ID を正とし、ワイヤの座標で推論しない。各 Rule は純粋であり、外部 API 実行は行わない。
