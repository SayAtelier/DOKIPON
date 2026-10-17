# v8.2 适配 — Step 1 扫描报告

> 写在 `PLAN-update-v8-2.md` 旁边。运行日期 2026-10-18，main @ f3625c2。
> 这不是一份要 review 的文档——它是给 Step 2 的 3 个子代理的工作输入。

## 0. 工作区状态

`git status` 干净。除了 `PLAN-update-v8-2.md` 自身之外没有任何未跟踪或未提交的文件。

## 1. v5.4 路径残留

命令：
```
grep -rn "docs/GemArc-5D-Design-Atlas-v5.4" --include="*.rs" --include="*.dart" --include="*.md" --include="*.yaml" --include="*.sh" --include="*.json" --include="*.toml" .
```

去掉本 plan 自身的 3 处，结果 5 处：

| 文件 | 行 | 上下文 |
|---|---|---|
| `contracts/openapi.yaml` | 9 | "Converted from `docs/GemArc-5D-Design-Atlas-v5.4/reference-models/commerce_openapi.yaml`" |
| `docs/ecommerce/01-requirements/要件定義書.md` | 2 | frontmatter 標 |
| `docs/ecommerce/07-构建与运行.md` | 3 | 介绍段 "由 `docs/GemArc-5D-Design-Atlas-v5.4` 改造而来" |
| `docs/ecommerce/README.md` | 3 | 介绍段 "本文件由 `docs/GemArc-5D-Design-Atlas-v5.4` 改造而来" |

**Rust 代码（`services/crates/`）零引用**。**Dart 代码（`apps/`、`packages/`）零引用**。Agent B 的工作量很小。

## 2. 旧名 / 商品名残留

命令：
```
grep -rn "commerce_atlas\|Feliture" --include="*.rs" --include="*.dart" --include="*.md" --include="*.yaml" .
```

**结果：零**。`Feliture` 是 Atlas 设计里假想的目标商品站名，v5.4 文档里出现过但代码里没用过。

## 3. v8.2 已经有的引用

```
grep -rln "docs/gemarc-5d-design-atlas" --include="*.rs" --include="*.dart" --include="*.md" --include="*.yaml" .
```

**结果：只有 `PLAN-update-v8-2.md` 自己**。新目录完全没被现有代码引用——这是预期的，因为代码本就是按 v5.4 不变量写的（这些不变量 v8.2 也不要求改）。

## 4. 结论

- v5.4 残留都在 4 份 markdown + 1 份 yaml 里。Agent B 可以一次性清完。
- Agent C（`tests/` 目录）没有需要清扫的引用——纯创建工作。
- Agent A（文档改写）除了改 4 处 v5.4 字面引用，更大的工作量是按 `docs/gemarc-5d-design-atlas/docs/ja-JP/01-04` 的结构重写。
- 任何 Rust/Dart 代码不需要改。这跟之前几轮的"代码实现的不变量 v8.2 也承认"判断一致。

## 5. 注意事项给 Step 2 的 agent

- `contracts/openapi.yaml` 里的 v5.4 引用是 description 字段里"convert from"的叙述，不是路径定位。改成 "convert from v8.2 atlas" 之类。
- 4 处 markdown 里的引用有的是设计依据（要件書 frontmatter），有的是历史叙述（README、07）。Agent A 自己判断是"换成新路径"还是"改写成'v5.4 → v8.2 重写'的叙述"。
- 替换时**别**碰文件本身的章节标题和 ID（FR-CHECKOUT-01 / CASE-ORDER-001 等）。这些 ID 在代码里直接引用。
- `docs/gemarc-5d-design-atlas/QA_FINAL.md` 明确说**DEGRADED / NOT FULLY ACCEPTED**。新文档里如果引用 v8.2 路径，要带这个标签，别让人觉得新文档等同于"v5.4 改进版"。
