# GemArc 5D / UI·UX Design Specification

## 主设计原则

工程主信息优先、语义与表现分离、渐进披露、一次只强调一个焦点、低强度动画、无色彩单点依赖。设计网站首先是给研发、设计审查和 AI 读取的工程规格书，而不是品牌宣传页。

## 核心 Token（深色阅读模式）

| Token | 值 | 用途 |
|---|---|---|
| Background | `#0D1420` | 大面积低饱和背景 |
| Sidebar | `#101A28` | 稳定导航，不与画布争夺焦点 |
| Surface | `#152232` | 设计文档与检查面板 |
| Elevated | `#203246` | 选择、悬浮及弹出表面 |
| Divider | `#314254` | 弱分隔，不使用过亮边框 |
| Text primary | `#EDF2F7` | 主文案 |
| Text secondary | `#BDCbd9` | 正文辅助文字 |
| Selected/interactive | `#76D7C4` | 焦点、选中、操作 |
| Informational | `#8CB5E6` | 非故障信息与关系 |
| Warning | Amber / icon + text | 风险/待确认，非普通选中 |
| Failure | Coral / broken shape + label | 失败与警报 |

基本单位 4px，主要间距 8/12/16/24/32，内容区正文 14–16px，分组标题 16–22px，背景噪声不可压过关系线与标签。保持重要文字对比度目标 AA（正文 4.5:1，适用的大字号与非文本 3:1），在 200% 文字缩放下可使用。实际测量对象应包含所有状态图例、次级标签、Edge Tooltips、输入焦点。

## 交互优先级

1. 浏览者选择“想回答的问题”。
2. 画布显示与问题有关的节点和关系。
3. 无关内容按相关性退避，必要时隐藏低优先级标签；选择保持可见。
4. 右侧细节提供可溯源的 Graph/Case/Design IDs。
5. 4D/5D 才出现时间控制；默认暂停，不自动播放重复装饰动画。
6. 一次故障注入只改变 Guard/Input，执行引擎重算路径，三维舞台显示首次真实分歧。
7. 撤销/清除焦点/重置模型操作易于寻找，不以无限画布的当前位置限制回退。

## 视觉语义不冲突

`shape = functional role`、`base color = domain`、`ring shape / glyph = runtime status`、`edge stroke = relation type`、`path effect = event`、`branch overlay = causal difference`。外环故障状态不能给宝石表面染成全红，否则混淆“Domain vs State”。普通导航、表单、时间轴不使用宝石外形。

## UX 验收

宽度 390/768/1024/1440px 分别验证导航、提示词表单、SVG 图、3D 场景与 Case 轨道；键盘 Tab、焦点可见、Escape、Reader Font 200%、中文/日文/英文溢出、Touch/Mouse 3D 操作、防误触 Edge。验证 `prefers-reduced-motion` 时自动关闭仅装饰性动效，保留逻辑时间状态文本。不得只凭屏幕截图认定可访问性合格。
