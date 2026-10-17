/* 1D Intent Studio — prompts bound to the same canonical requirement and flow IDs. */
(() => {
  'use strict';
  const D=window.GEMARC_DESIGN_DATA;
  const $=id=>document.getElementById(id);
  if (!$('promptStage') || !D) return;
  const stageName={requirements:'要件定義',basic:'基本設計',detailed:'詳細設計',simulation:'4D/5D 状态及因果设计',implementation:'全栈工程实现',review:'设计审查'};
  $('promptFeature').innerHTML=D.features.map(f=>`<option value="${f.id}">${f.id} · ${f.name.replace(/</g,'&lt;')}</option>`).join('');
  $('promptFlow').innerHTML=D.flows.map(f=>`<option value="${f.id}">${f.id} · ${f.title.replace(/</g,'&lt;')}</option>`).join('');
  function compose() {
    const f=D.features.find(x=>x.id===$('promptFeature').value)||D.features[0];
    const flow=D.flows.find(x=>x.id===$('promptFlow').value)||D.flows[0];
    const stage=$('promptStage').value;
    const cases=D.cases.filter(c=>c.flowId===flow.id);
    const related=D.engineeringCases.filter(c=>c.requirementId===f.id).slice(0,8);
    const strict=$('promptEvidence').value==='strict';
    const stageDuty={
      requirements:'编写业务场景、参与者、系统边界、FR/NFR、验收条件、反例和待确认事项。不要产生不存在的技术结构。',
      basic:'定义 Logical/Process/Data/Deployment/Security 视图和服务职责、Flutter/LINE 端边界、跨层 Node–Edge 语义、ADR 和非功能权衡。',
      detailed:'定义 API/事件 Schema、字段及验证、幂等、状态机 Guard、异常码、时序、事务一致性、补偿和具体测试断言。',
      simulation:'设计 3D 基础场景、4D 可重放事件序列、5D 基线/干预/反事实分叉，对照模型事件、状态快照及影响边。',
      implementation:'依据已经确认的设计生成 Flutter Web/Android/iOS 与 LINE LIFF 适配、Rust 微服务、OpenAPI、迁移、IaC 及 UT/IT/ST/UAT；报告未验证项。',
      review:'检查需求→Node/Edge→接口→状态机→Case→测试是否闭环，并按证据不足、约束冲突、可访问性和部署风险输出缺口。'
    };
    const constraint=strict?'没有源码证据或可信公开页面依据的内容必须保持 Designed/Hypothetical；不得声称 Implemented/Observed。':'可以完善 Proposed 结构，但每一条新增事实必须标明 Designed 或 Hypothetical。';
    const prompt=`# GEMARC / 1D DESIGN INTENT
阶段：${stageName[stage]}
设计对象：Feliture Flutter Web + Android/iOS + LINE MINI App（LIFF）
设计源版本：${D.meta.version||'5.4'}；维度定义：0D 为不可见存在性门禁，1D 为提示词。

## 01 / 范围与证据
需求 ID：${f.id}
需求标题：${f.name}
范围：${f.scope}
需求内容：${f.description}
优先级：${f.priority}
事实规则：${constraint}
任何业务推测都需注明来源与不确定性，不得伪造支付、库存、数据库或生产环境。

## 02 / 必须满足的设计追踪
基本设计：${f.design}
详细设计：${f.detail}
代表性 Case：${f.case}
既有机器可读架构模型：reference-models/architecture.json
关联场景 Flow：${flow.id} · ${flow.title}
关联正常/异常/边界/恢复 Case：${cases.map(c=>c.id).join('、')}
全栈设计 Case：${related.map(c=>c.id).join('、')||'待按需求建立映射'}

## 03 / 本阶段任务
${stageDuty[stage]}

## 04 / 强制可交互表现
2D：按照对应 UML/ER/状态/时序标准表达并可单独选择 Node 和 Edge。
3D：单一 GraphRevision，Building/Floor/Room、左手坐标系 (+X 向右，+Y 向上，+Z 向前)，真实三维节点及可拾取边。
4D：同一个 3D Scene + 单调逻辑时钟 + Guard 状态转移 + 事件 Trace；暂停/seek/回放不得改变事件结果。
5D：在相同 3D Scene/初态/模型版本下执行 baseline 和 intervention；识别首次实际差异、传播、阻断、补偿、收敛与无差异情景。
3D 工程节点可使用标准宝石切型；普通 UI、文字、按钮、时间轴不得宝石化。不得以动画装饰替代模拟计算。

## 05 / 验收与非功能约束
既有验收条件：${f.accept}
Case 流程事实等级：${flow.sourceVerification}
工程验证：ID 稳定性、关系端点、类型正确、状态机终态、因果轨迹一致、异常补偿幂等性、并发/超时/网络分区恢复。
视觉验证：语义色不依赖颜色单通道，WCAG 2.2 AA 文字对比目标，焦点可见；减少动效；1440/1024/768/390px 下不重叠、不超框。
文档：遵循 IPA 共通框架参考，日文默认要件/基本/详细/测试资料只写最终有效状态；追踪标注未解决事项。

## 06 / 必须交付及证据
可机器读取的契约、可检查的动态设计内容、追踪矩阵、正常和异常 Case、重现实测的验证命令和结果。
不存在的事实→Unknown；未执行的测试→NOT_VERIFIED；不允许伪造通过记录。
按需求-设计-实现-Case-Test 链路输出最终状态，不得将设计文档浏览器的实现结构误作目标商业系统架构。`;
    $('promptText').value=prompt;
    $('promptMeta').innerHTML=`<span class="trace-chip">${f.id}</span><span class="trace-chip">${f.design}</span><span class="trace-chip">${f.detail}</span><span class="trace-chip">${flow.id}</span><span class="trace-chip">${cases.length} Case</span>`;
    $('promptLength').textContent=`${prompt.length.toLocaleString()} 字符 · ${stageName[stage]}`;
    return prompt;
  }
  ['promptStage','promptFeature','promptFlow','promptEvidence'].forEach(id=>$(id).addEventListener('change',compose));
  $('promptCopy').addEventListener('click',async()=>{
    const text=compose();
    try { await navigator.clipboard.writeText(text);$('promptCopy').textContent='已复制 ✓'; }
    catch { $('promptText').focus();$('promptText').select();const ok=document.execCommand('copy');$('promptCopy').textContent=ok?'已复制 ✓':'请按 Ctrl+C'; }
    setTimeout(()=>$('promptCopy').textContent='复制',2100);
  });
  $('promptSave').addEventListener('click',()=>{
    const blob=new Blob([compose()],{type:'text/markdown;charset=utf-8'}),url=URL.createObjectURL(blob),a=document.createElement('a');
    a.href=url;a.download='GemArc_1D_'+$('promptFeature').value+'_'+$('promptStage').value+'.md';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
  });
  window.GemArcPromptAPI=Object.freeze({generate:()=>compose(),model:{dimension1:'Prompt',dimension0:'Existence gate',dimension2:'2D diagrams',dimension3:'Semantic 3D graph',dimension4:'Time+3D',dimension5:'Causal+4D'}});
  compose();
})();
