from pathlib import Path
import re
p=Path(__file__).parent
html=(p/'index.template.html').read_text(encoding='utf8')
html=html.replace('FLUTTER DESIGN ATLAS</small>', 'UNIFIED 3D DESIGN ATLAS</small>').replace('V5.2','V5.3')
html=html.replace('<button class="nav" data-view="library">','<button class="nav" data-view="blueprint"><span class="nav-icon">⚙</span>AI 全栈工程蓝图<span class="nav-index">AI</span></button>\n <button class="nav" data-view="library">')
html=html.replace('3D · 楼宇空间','3D · Graph 空间').replace('4D · 时间与事件','4D · 动态 3D').replace('5D · 因果分歧','5D · 因果 4D')
html=html.replace('<button class="button outline" data-jump="causal">播放因果推演 ↗</button>','<button class="button outline" data-jump="spatial">打开统一 3D 场景 ↗</button>')
start=html.index(' <section class="view" id="view-spatial">')
end=html.index(' <section class="view" id="view-timeline">',start)
html=html[:start]+''' <section class="view" id="view-spatial"><div class="page-heading"><div class="eyebrow">3D · LHC WEBGL · CANONICAL NODE–EDGE WORLD</div><h1>统一三维图论架构</h1><p>真正的 WebGL 三维模型。图论 Node 和 Edge 共享唯一 ID 与世界坐标，和 4D / 5D 使用完全相同的场景；楼宇、楼层与房间仅用于有语义的工程组织。</p></div>
 <div class="panel"><div class="toolbar wrap-toolbar"><div class="segmented" id="floorSelect"><button class="selected" data-floor="all">全部楼层</button><button data-floor="experience">F3 · 体验</button><button data-floor="domain">F2 · 领域</button><button data-floor="infrastructure">F1 · 平台</button></div><label class="slider-label">左手系 Yaw <input type="range" id="cameraRotate" min="-170" max="170" value="-25"/></label><label class="slider-label">Pitch <input type="range" id="cameraPitch" min="-70" max="70" value="24"/></label><button class="button subtle small" id="explodeToggle">⇅ 收起楼层</button><button class="button subtle small" id="spatialReset">↺ 复位</button></div>
 <div class="spatial-layout"><div class="spatial-view" id="spatialView"><div id="spatialScene" class="gem-scene" role="application" aria-label="左手坐标系 3D 节点边建筑模型"></div><div class="scene-stamp">NATIVE WEBGL · LHC +X RIGHT · +Y UP · +Z FORWARD</div></div><aside class="explorer-inspector" id="spatialInspector"></aside></div>
 <div class="canvas-footer"><span>鼠标拖动 = Yaw/Pitch · 滚轮 = Zoom · 点击宝石节点或关系线 = 独立检查 · 与 4D/5D 同模型</span><span id="spatialStats">—</span></div></div></section>
''' +html[end:]
html=html.replace('每一步由状态转移规则求值得到。切换 Case、拖动时间或选择节点，观察同一架构下行为如何变化。','4D = 同一个 3D 世界 + 可计算的事件、时间、状态与运动。切换 Case 或播放时间线，3D 节点与 Edge 同步响应。')
html=html.replace('正常基线、异常注入、Guard 分支、状态差异和补偿操作均由规则计算；参数改变时重新推演，而不是替换预制动画。','5D = 可注入正常/异常/边界/组合条件的 4D 三维场景。所有 Guard、补偿、反事实对比均重新计算后在同一 WebGL 场景中呈现。')
blue=''' <section class="view" id="view-blueprint"><div class="page-heading"><div class="eyebrow">AI EXECUTABLE SPEC / IPA DESIGN-TO-CODE</div><h1>从需求到全栈微服务工程</h1><p>设计对象为 Flutter Web、Android、iOS、LINE MINI App + BFF/API/Domain Services。所有服务合同是目标设计，非 Feliture 已发现的生产架构。每一个开发工件均有稳定 ID、接口输入输出与可验证 Case。</p></div>
 <div class="metrics"><div class="metric"><span>逻辑拓扑</span><b>统一 Scene</b><small>3D / 4D / 5D 同源</small></div><div class="metric"><span>客户端</span><b>4 渠道</b><small>Web、Android、iOS、LINE</small></div><div class="metric"><span>后端设计</span><b>7 边界</b><small>BFF、Identity、Catalog、Cart、Inventory、Order、Payment Adapter</small></div><div class="metric"><span>工程质量</span><b>Gates</b><small>可追踪 API、数据、测试与恢复</small></div></div>
 <div class="panel block-margin"><div class="panel-title">可交付的微服务分层设计 · Designed / Hypothetical</div>
 <div class="arch-stack"><div class="arch-tier"><div class="arch-tier-label">CLIENTS</div><div class="arch-box" data-jump="platforms">Flutter Web</div><div class="arch-box" data-jump="platforms">Flutter Android/iOS</div><div class="arch-box" data-jump="platforms">LINE MINI App / LIFF</div></div>
 <div class="arch-connector">HTTPS · OAuth/OIDC · Contract-first · Correlation-ID · Rate Limit</div>
 <div class="arch-tier highlight-tier"><div class="arch-tier-label">EDGE</div><div class="arch-box">API Gateway / BFF</div><div class="arch-box">Identity Session</div><div class="arch-box">Policy / Eligibility</div></div>
 <div class="arch-connector">内部 REST/gRPC 可选 · 可靠事件 Outbox · Saga 状态机 · 禁止跨库共享表</div>
 <div class="arch-tier"><div class="arch-tier-label">DOMAIN</div><div class="arch-box">Catalog</div><div class="arch-box">Cart</div><div class="arch-box">Inventory</div><div class="arch-box">Order</div><div class="arch-box">Payment Adapter</div></div>
 <div class="arch-connector">Per-service Storage · Idempotency · Event Schema · Reconciliation</div>
 <div class="arch-tier dashed-tier"><div class="arch-tier-label">INFRA</div><div class="arch-box">PostgreSQL 逻辑库</div><div class="arch-box">Message Broker</div><div class="arch-box">Observability</div><div class="arch-box">Secrets / CI-CD</div></div></div></div>
 <div class="two-panels"><div class="panel"><div class="panel-title">AI 实现输入 · 文档导航</div><div class="info-list"><div><b>01 · Requirements + NFR</b><p>需求 ID、非功能定量指标、角色权限与数据分类</p></div><div><b>02 · Basic / Detailed Design</b><p>服务责任边界、C4、UML、ER、状态图、Dart 类/接口、REST/事件模型</p></div><div><b>03 · 3D / 4D / 5D Contract</b><p>统一左手系、图/场景映射、时间帧与因果可重复执行</p></div><div><b>04 · Cases & Release Gates</b><p>UT → IT → E2E → Contract → Chaos → Security → LINE 平台核验</p></div></div><button class="button primary small" data-doc="architecture/AI_IMPLEMENTATION_BLUEPRINT.md">查看全栈详细蓝图 →</button></div>
 <div class="panel"><div class="panel-title">生成 AI 必须遵守的门禁</div><div class="info-list"><div><b>禁止无证据声称完成</b><p>Designed/Hypothetical 不可冒充 Implemented/Observed；编译、部署、平台审核不可互相替代。</p></div><div><b>契约先行</b><p>先冻结 API、Event、SQL Migration 与状态机契约，再生成前后端实现。</p></div><div><b>错误优先</b><p>幂等、延迟、失败、撤销、重放、并发与支付未知终态必须测试。</p></div><div><b>逐层可替换</b><p>Flutter Client / LIFF Bridge / Backend / Graph 3D Adapter / Causal Engine 必须解耦。</p></div></div><button class="button subtle small" data-doc="architecture/UNIFIED_3D_4D_5D_CONTRACT.md">查看 3D–5D 数据契约 →</button></div></div>
 </section>
'''
html=html.replace(' <section class="view" id="view-library">',blue+' <section class="view" id="view-library">')
# script initialization sequence (currently at EOF)
html=html.replace('<script src="./data.js"></script><script src="./app.js"></script>', '<script src="./data.js"></script><script src="./scene3d.js"></script><script src="./app.js"></script>')
(p/'index.template.html').write_text(html,encoding='utf8')
app=(p/'app.js').read_text(encoding='utf8')
app=app.replace("platforms:'跨端设计',library:'IPA 文库'", "platforms:'跨端设计',blueprint:'AI 全栈工程蓝图',library:'IPA 文库'")
app=app.replace("if(view==='platforms')renderDevice();}", "if(view==='platforms')renderDevice();}")
# Remove existing faux-3D projection and reuse the native WebGL 3D scene everywhere
app=re.sub(r'^function renderSpatial\(\).*$', '''function renderSpatial(){const S=window.GemArcScene;S.attach($('spatialScene'),{mode:'3d'});S.renderAll();const focus=S.state.focus,d=D.designGraph,node=focus?.kind==='node'?d.nodes.find(n=>n.id===focus.id):null,edge=focus?.kind==='edge'?d.edges.find(e=>e.id===focus.id):null;let html='<div class="inspector-label">3D · CANONICAL GRAPH</div>';if(node)html+=`<h3>${E(node.name)}</h3>${badge(node.verification)}<dl class="keyvals"><dt>Node ID</dt><dd>${E(node.id)}</dd><dt>Kind</dt><dd>${E(node.kind)}</dd><dt>World Space</dt><dd>${E(JSON.stringify(S.viewPositions()[node.id]))}</dd><dt>Incoming Edge</dt><dd>${E(d.edges.filter(e=>e.targetId===node.id).map(e=>e.id).join(', ')||'—')}</dd><dt>Outgoing Edge</dt><dd>${E(d.edges.filter(e=>e.sourceId===node.id).map(e=>e.id).join(', ')||'—')}</dd><dt>Definition</dt><dd>${E(node.description)}</dd></dl>`;else if(edge)html+=`<h3>EDGE ${E(edge.id)}</h3>${badge(edge.verification)}<dl class="keyvals"><dt>Source</dt><dd>${E(edge.sourceId)}</dd><dt>Target</dt><dd>${E(edge.targetId)}</dd><dt>Type</dt><dd>${E(edge.type)}</dd><dt>Directed</dt><dd>${edge.directed?'true':'false'}</dd><dt>Evidence</dt><dd>${E(edge.evidenceIds?.join(', ')||'设计关系')}</dd></dl>`;else html+='<h3>Node/Edge 都在 3D 世界中</h3><p>拖动旋转、滚轮缩放、点击宝石或连接线选择，所选对象的上下游按语义高亮。4D/5D 复用同一世界场景。</p><p>本次 WebGL 场景用于设计审阅，未来 Flutter 产品需按同一 scene schema 重现，不能将其误认作 Flutter 已编译。</p>';html+='<button class="button subtle small" id="spatialClear">清除聚焦</button>';$('spatialInspector').innerHTML=html;$('spatialClear').onclick=()=>{S.state.focus=null;renderSpatial()};$('spatialStats').textContent=d.nodes.length+' Nodes · '+d.edges.length+' Edges · LHC';}''',app,flags=re.M)
app=re.sub(r'^function registerSpatial\(\).*$', '''function registerSpatial(){const S=window.GemArcScene;$('spatialScene').addEventListener('gemarc-select',()=>renderSpatial());$('cameraPitch').addEventListener('input',e=>{S.state.pitch=Number(e.target.value);S.renderAll()})}''',app,flags=re.M)
# add lab scene before timeline track
app=app.replace('<div class="lab-graph"><div class="lab-track" id="labTrack"></div></div>','<div class="lab-graph"><div class="gem-scene lab-3d-scene" id="lab3dScene" role="application" aria-label="与 3D 架构共用的 WebGL 4D 5D 场景"></div><div class="lab-scene-caption">同一 3D 场景 · Graph Node/Edge · ${is5?"5D 多因子因果分歧":"4D 动态事件轨迹"}</div><div class="lab-track" id="labTrack"></div></div>')
# renderLabTrace at bottom use replace section before original view causal compare logic
needle="const box=root.querySelector('#labCompare');"
app=app.replace(needle,'''const binding=window.GemArcScene.eventMap[frame.nodeId];window.GemArcScene.attach(root.querySelector('#lab3dScene'),{mode:view==='causal'?'5d':'4d',frame,trace});window.GemArcScene.update(root.querySelector('#lab3dScene'),{mode:view==='causal'?'5d':'4d',frame,trace});if(!binding){root.querySelector('#labBranch').textContent+=' · 未绑定 3D Graph ID: '+frame.nodeId;}''' +needle)
# upgrade init handlers; original app.js has state.yaw & UI handlers in init
app=app.replace("$('cameraRotate').oninput=e=>{state.yaw=Number(e.target.value);renderSpatial()}","$('cameraRotate').oninput=e=>{window.GemArcScene.state.yaw=Number(e.target.value);window.GemArcScene.renderAll()}")
app=app.replace("$('explodeToggle').onclick=()=>{state.exploded=!state.exploded;$('explodeToggle').textContent=state.exploded?'⇅ 收起楼层':'⇅ 展开楼层';renderSpatial()}","$('explodeToggle').onclick=()=>{const s=window.GemArcScene.state;s.exploded=!s.exploded;$('explodeToggle').textContent=s.exploded?'⇅ 收起楼层':'⇅ 展开楼层';renderSpatial()}")
app=app.replace("$('spatialReset').onclick=()=>{state.floor='all';state.yaw=-26;state.exploded=true;state.spatialFocus=null;$$('#floorSelect button').forEach(b=>b.classList.toggle('selected',b.dataset.floor==='all'));$('cameraRotate').value=-26;$('explodeToggle').textContent='⇅ 收起楼层';renderSpatial()}","$('spatialReset').onclick=()=>{const s=window.GemArcScene.state;Object.assign(s,{visibleFloor:'all',yaw:-25,pitch:24,zoom:1400,exploded:true,focus:null});$$('#floorSelect button').forEach(b=>b.classList.toggle('selected',b.dataset.floor==='all'));$('cameraRotate').value=-25;$('cameraPitch').value=24;$('explodeToggle').textContent='⇅ 收起楼层';renderSpatial()}")
# modify floor selection event
app=app.replace("state.floor=e.target.dataset.floor;$$('#floorSelect button')", "window.GemArcScene.state.visibleFloor=e.target.dataset.floor;$$('#floorSelect button')")
# any stale state floor check handler display inspect
(p/'app.js').write_text(app,encoding='utf8')
css=(p/'styles.css').read_text(encoding='utf8')
css+='''
/* Unified native WebGL left-handed 3D scene / re-used in 4D and 5D */
.gem-scene{width:100%;height:500px;position:relative;overflow:hidden;touch-action:none;background:#081321;border-radius:12px;border:1px solid #345161;cursor:grab}
.gem-scene:active{cursor:grabbing}
.gem-scene canvas{position:absolute;inset:0;width:100%;height:100%;display:block}
.gem-scene canvas.gem-overlay{pointer-events:none}
.spatial-view #spatialScene{min-height:520px}
.lab-3d-scene{height:335px;min-height:335px;margin:8px 0 2px}
.lab-scene-caption{padding:6px 10px;font-size:11px;color:#9bbdc7;letter-spacing:.03em;border-bottom:1px solid #243747}
@media(max-width:850px){.gem-scene{height:400px}.lab-3d-scene{height:285px;min-height:285px}}
'''
(p/'styles.css').write_text(css,encoding='utf8')
print('upgraded html',len(html),'app',len(app),'css',len(css))
