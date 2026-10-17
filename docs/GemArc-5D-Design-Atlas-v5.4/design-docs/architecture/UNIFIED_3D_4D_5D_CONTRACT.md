# 統合 3D／4D／5D アーキテクチャ詳細設計書

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
