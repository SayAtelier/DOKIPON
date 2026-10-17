> 文書種別: IPA 共通フレーム2013・非機能要求グレード2018 を参照した独自成果物。IPA の審査・認証済みではない。
> 対象: GemArc 5D Flutter Workbench。Feliture の公開ページはサンプル入力であり、非公開システム構成の証拠ではない。
> 基準日: 2026-10-17。仕様の実装状態は `../quality/VERIFICATION.md` を正とする。

# 源码与证据溯源 — 基本設計・詳細設計対応（evidence）

- Requirement: `FR-EVIDENCE-01` / `NFR-EVIDENCE-01`
- Design: `BD-EVIDENCE-01`, `DD-EVIDENCE-01`
- Model: Directed guarded finite state machine / Max Steps 24 / Verification `Hypothetical`

| Rule ID | Graph Node ID | 検証条件 / 動作 | Next YES | Next NO | Animation |
|---|---|---|---|---|---|
| begin | gateway | 接收源码与证据溯源请求 / patch/terminal | check-a | — | signal |
| check-a | store | 源码 Git SHA 可确认 / aOk | check-b | abort-a | branch |
| abort-a | store | 来源仓库不可访问 / patch/terminal | end-failed | — | isolate |
| check-b | webclient | Node 有证据引用 / bOk | check-c | compensate | branch |
| compensate | webclient | 执行补偿 / 降级验证等级并保留待确认标记 / patch/terminal | can-recover | — | reverse |
| can-recover | webclient | 验证恢复前置条件 / recoverable | recovered | end-failed | branch |
| recovered | webclient | 降级验证等级并保留待确认标记 / patch/terminal | end-recovered | — | rejoin |
| check-c | webclient | 证据级别不被升级 / cOk | check-threshold | abort-c | branch |
| check-threshold | webclient | 可用性/负荷阈值 ≤ 100% / thresholdPercent | commit | abort-threshold | branch |
| abort-threshold | webclient | 超过设计阈值，拒绝提交 / patch/terminal | end-failed | — | isolate |
| abort-c | webclient | 设计假设冒充已实现 / patch/terminal | end-failed | — | isolate |
| commit | store | 确认业务不变量并完成 / patch/terminal | end-success | — | flow |
| end-success | store | 正常完成 / patch/terminal | End | — | settle |
| end-recovered | store | 降级恢复完成 / patch/terminal | End | — | settle |
| end-failed | store | 安全终止并保留审计 / patch/terminal | End | — | rollback |

### ケース別の条件／期待結果

| ID | 種別 | Trigger | Inputs | Terminal |
|---|---|---|---|---|
| CASE-EVIDENCE-001 | normal | 所有守卫成立 | `aOk=True bOk=True cOk=True recoverable=False thresholdPercent=99` | success |
| CASE-EVIDENCE-002 | abnormal | 来源仓库不可访问 | `aOk=False bOk=True cOk=True recoverable=False thresholdPercent=99` | failed |
| CASE-EVIDENCE-003 | abnormal | 边关系缺失证据 | `aOk=True bOk=False cOk=True recoverable=False thresholdPercent=99` | failed |
| CASE-EVIDENCE-004 | abnormal | 设计假设冒充已实现 | `aOk=True bOk=True cOk=False recoverable=False thresholdPercent=99` | failed |
| CASE-EVIDENCE-005 | recovery | 降级验证等级并保留待确认标记 | `aOk=True bOk=False cOk=True recoverable=True thresholdPercent=99` | recovered |
| CASE-EVIDENCE-006 | boundary | 补偿已尝试但不得伪称恢复 | `aOk=True bOk=False cOk=True recoverable=False thresholdPercent=99` | failed |
| CASE-EVIDENCE-007 | composite | 优先在最早失败守卫安全停止，不越级补偿 | `aOk=False bOk=False cOk=False recoverable=True thresholdPercent=99` | failed |
| CASE-EVIDENCE-008 | boundary | 在设计阈值上限内，恪守闭区间约定 | `aOk=True bOk=True cOk=True recoverable=False thresholdPercent=100` | success |
| CASE-EVIDENCE-009 | boundary | 100% 为闭区间上限，101% 必须走独立失败路径 | `aOk=True bOk=True cOk=True recoverable=False thresholdPercent=101` | failed |

#### 不変条件
状態の成功は `end-success` 経由のみ。異常 B は補償を記録すること。仕様上の Node ID を正とし、ワイヤの座標で推論しない。各 Rule は純粋であり、外部 API 実行は行わない。
