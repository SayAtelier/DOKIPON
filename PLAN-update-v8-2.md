# v8.2 适配与并行子代理方案

> 状态：等待你点头
> 目标：在 D:/DOKIPON 现有工作区上把 v5.4 → v8.2 的改造收尾，按 3 路并行

## 0. 你在此时已经知道的事实

- `D:/DOKIPON` 当前分支 `main` @ `f3625c2`。
- 3 个新提交 `main` 领先 `UlyssesLeoLee/dev`（`f3625c2 8.2版文档更新`、`1756173 前端开发`、`9296765 电商版本设计资料`）。
- 我之前所有电商改造（8 份 `docs/ecommerce/*.md`、8 个 Rust crate、3 个 Flutter app、6 个 mock、`tools/e2e_checkout.sh`、`contracts/openapi.yaml`）都还在工作区里**没 commit**。
- `docs/gemarc-5d-design-atlas/` 是新交付物（与 v5.4 不同的项目：浏览器里跑的 Atlas app + 4 份日语设计书）。`QA_FINAL.md` 判定 **DEGRADED / NOT FULLY ACCEPTED**。
- 旧 `docs/GemArc-5D-Design-Atlas-v5.4` 已经不存在。
- `clarify` 工具在我环境里坏掉，没法用结构化问题收答案。
- **修正（2026-10-18）**：写 plan 时我以为前几轮的工作（8 份文档、8 个 Rust crate、6 个 mock、3 个 Flutter app）还在工作区里没 commit——错的。`git status` 是干净的，所有产物**早就入库了**（`docs/ecommerce`、`services/crates`、`apps/`、`packages/`、`contracts/openapi.yaml` 都在 `f3625c2`/`1756173` 里）。所以 Step 1 改成"看看这堆东西和 v8.2 之间到底差什么"，而不是把它们当未提交改动打包。

## 1. 你在上一轮已经决定的事

| 决定 | 含义 |
|---|---|
| 按推荐方案做 | 走我上一条消息里的 4 步 |
| dev = `UlyssesLeoLee/dev` | rebase 目标是 `D:/orcaWork/DOKIPON/dev` 这个 worktree 上的 dev 分支 |
| 非破坏性 rebase | 失败停下来问你；不 force-push、不 reset、不 `--no-ff` |

## 2. 我打算怎么走（按顺序）

### Step 1 — 我自己（不开子代理）

**先确认 git 是干净的。** 之前几轮的工作已入库，工作区只有 `PLAN-update-v8-2.md` 自身。

```bash
cd D:/DOKIPON
git status --short
# 应该只有 ?? PLAN-update-v8-2.md
```

接下来做**扫描，不做打包提交**。我跑：

- `grep -r "docs/GemArc-5D-Design-Atlas-v5.4" --include="*.rs" --include="*.dart" --include="*.md" --include="*.yaml" --include="*.sh" .`——列所有还指向 v5.4 的位置
- 同样扫 `commerce_atlas` / `Feliture` 等 Atlas 旧名
- 把 v5.4 路径在哪些文件里出现几次列出来给 Step 2 的 agent B 用

这**不会**改任何文件。Step 1 的产物是一份扫描报告，附在 plan 同目录的 `PLAN-update-v8-2-scan.md` 里。

### Step 2 — 3 个子代理并行

它们都跑在 `D:/DOKIPON`（已经 commit 后的状态）上，每个拿到一段明确范围：

| agent | 任务 | 范围 | 不许碰 |
|---|---|---|---|
| **A · 文档** | 改写 `docs/ecommerce/` 4 份核心文档（要件 / 基本 / 詳細 / セキュリティ / テスト / 運用）以匹配 `docs/gemarc-5d-design-atlas/docs/ja-JP/01-04`。加 v8.2 影响说明 + 链接到 `UX_DESIGN_SYSTEM.md` + "DEGRADED / NOT FULLY ACCEPTED" 标签 | `docs/ecommerce/*.md` | 任何 `.rs` / `.dart` / `.yaml` |
| **B · 路径与引用** | 全工作区扫描 `docs/GemArc-5D-Design-Atlas-v5.4` 字面串，替换为 `docs/gemarc-5d-design-atlas`。更新 `services/` 注释、`apps/` 注释、`contracts/openapi.yaml` 里的 description、`tools/e2e_checkout.sh` 里的注释 | 所有文本文件 | 文件移动/重命名 |
| **C · tests 目录** | ① 在 `D:/DOKIPON/tests/mocks/` 下建一个 `README.md`，说明：这套 mock 覆盖的是 v5.4 → v8.2 之间仍标 `[Designed]` 的设计用例，对应 `docs/ecommerce/05-test/测试設計書.md` §3 ② 把 `tools/e2e_checkout.sh` 复制成 `tests/integration/e2e_checkout.sh` 并加一个 `--mocks-up` 模式：自动启动 6 个 mock binary 再跑 ③ 加 `tests/README.md` 顶层说明这一层的角色 | `D:/DOKIPON/tests/**` | 任何现有文件 |

并行触发方式：3 个 `delegate_task(goal=..., context=...)`，等全部完成。

### Step 3 — 我自己（不开子代理）

3 个 agent 全部完成后，我做这些：

```bash
cd D:/DOKIPON
git fetch origin UlyssesLeoLee/dev

# 先看 main 是否还能 fast-forward 进 dev —— 不强推
git log --oneline UlyssesLeoLee/dev..main | head
git log --oneline main..UlyssesLeoLee/dev | head

# 优先尝试 rebase，让 main 的提交干净地站在 UlyssesLeoLee/dev 之后
# (注意：是 rebase main on dev ，不是反过来 —— 让我的工作在最新的 dev 之上)
git status            # 应干净
# git rebase 是让 HEAD 变基到指定基。我把 main 上多出来的提交
# 一个一个重放到 UlyssesLeoLee/dev 之上。
# 但 main 当前就是 checkout 的分支，所以：
git rebase UlyssesLeoLee/dev
```

冲突策略：
- **能自动解的（白空、纯增删）**：我解。
- **不能自动解的**（`services/`、`docs/ecommerce/`、`tests/` 都可能冲突）：**停下来问你**，不 commit，不 continue。
- rebase 完成、`cargo test --workspace` + `flutter test` + `bash tools/e2e_checkout.sh` 三件套都过，才算 Step 3 完成。

### Step 4 — 我自己（不开子代理）

rebase 通过后：

```bash
# 验证
git log --oneline -10
cargo test --workspace    # 期望 99 passed
flutter test packages/domain_models    # 期望 24 passed
bash tools/e2e_checkout.sh            # 期望 22 passed

# 不 push，不动 UlyssesLeoLee/dev
git status
```

把结果告诉你，由你决定怎么合到 `UlyssesLeoLee/dev`（PR、命令行、UI 随便）。

## 3. 我不会做的事

- 不 push 任何东西到 origin / upstream
- 不改 UlyssesLeoLee/dev 任何历史（不 reset、不 force-push、不 --no-ff）
- 不动 Cargo.toml / pubspec.yaml 的依赖版本
- 不在 worktree 之间搬动未提交的改动
- 不开 dev 分支的 worktree（你已经有了 `D:/orcaWork/DOKIPON/dev`，不需要再造一个）

## 4. 风险与回退

- **Step 1 commit 后悔了**：`git reset HEAD^ --hard`，回到工作区干净状态
- **Step 2 子代理的 diff 你不满意**：`git reset --hard f3625c2`，重起
- **Step 3 rebase 冲突解不开**：`git rebase --abort`，把冲突列出来给你看
- **Step 3 之后测试挂了**：不 commit rebase 结果，所有改动留在 unstaged，列失败给你

## 5. 启动条件

你的下一条消息说"开始" / "OK" / "go" 我就开始。
说不"开始"我就不动。
