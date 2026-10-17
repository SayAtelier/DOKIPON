> 文書種別: IPA 共通フレーム2013・非機能要求グレード2018 を参照した独自成果物。IPA の審査・認証済みではない。
> 対象: GemArc 5D Flutter Workbench。Feliture の公開ページはサンプル入力であり、非公開システム構成の証拠ではない。
> 基準日: 2026-10-17。仕様の実装状態は `../quality/VERIFICATION.md` を正とする。

# UI / UX 詳細

- 情報設計: 2D、3D、Workflow、5D Lab、Evidence を NavRail／Mobile BottomNav に展開。
- 認知負荷: Node ID, Edge ID, 実装/仮説マークは必要箇所にのみ表示。ヒントと操作ラベルで意味の重複を避ける。
- 退避: Selected 100%、Directly Related 100%、Unrelated 8〜20% を基準とする View transform。Color のみで状態を区別しない。
- 2D: パン、拡大、Node と Edge の独立拾取。11px の Edge 拾取許容。
- 3D: フロアと部屋による読解、段階的爆発表示、回転、ノードとエッジ選択。非工学的な装飾物禁止。
- Gem 規則: 3D 工学オブジェクト専用。System は五角形紫、Service は六角形金、Page は四角形青緑、Entity は四角形緑、Hypothetical は橙の目立つ色で事実から区別。形と色を両方使用。
- Timeline: 再生・停止・前後・スクラブ・Rule ID・Guard 結果・差分を一組として提示。
- アクセシビリティ: 文字拡大 200%、コントラスト WCAG AA を設計目標、キーボード操作・Semantics 対応は未検証。
- モバイル: 850px 以下で BottomNav、Case list を上下へ再配置。LINE Safe Area・バックナビは端末試験が必要。
- ローカライズ: zh-CN を初期実装、ja-JP/en-US は Flutter locale と辞書投入・全画面検証をリリースゲートとして必須。
