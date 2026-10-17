/* GemArc scoped, deterministic discrete-event model. Not a production trace. */
(function (root, factory) {
  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  root.GemCore = api;
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';
  const deepCopy = x => JSON.parse(JSON.stringify(x));
  const types = new Set(['Validator','Cache','Proxy']);
  function validateModel(model) {
    const n = new Set(model.nodes.map(v => v.id));
    const e = new Map(model.edges.map(v=>[v.id,v]));
    if (n.size !== model.nodes.length || e.size !== model.edges.length) throw new Error('Duplicate graph ID');
    for(const edge of model.edges) if(!n.has(edge.sourceId) || !n.has(edge.targetId)) throw new Error('Dangling edge '+edge.id);
    for(const id of model.workflow) if(!e.has(id)) throw new Error('Missing workflow edge '+id);
    for(let i=1;i<model.workflow.length;i++){
      if(e.get(model.workflow[i-1]).targetId !== e.get(model.workflow[i]).sourceId) throw new Error('Disconnected workflow '+i);
    }
    return true;
  }
  function applyScenario(base, intervention={}) {
    validateModel(base);
    const model=deepCopy(base), delta={disabledNodes:[...(intervention.disabledNodes||[])],disabledEdges:[...(intervention.disabledEdges||[])],addedNodes:[],addedEdges:[],rewrittenEdges:[]};
    const inj=intervention.injected;
    if(inj){
      if(!types.has(inj.kind)) throw new Error('Element template not allowed');
      if(!['pass','reject'].includes(inj.behavior)) throw new Error('Invalid behavior');
      if(inj.kind!=='Validator' && inj.behavior==='reject') throw new Error('Only Validator can reject');
      if(inj.portType!=='CommerceMessage') throw new Error('Incompatible ports');
      if(inj.stateMachine!=='finite') throw new Error('State machine / termination missing');
      const old=model.edges.find(x=>x.id===inj.edgeId);
      if(!old || !model.workflow.includes(old.id)) throw new Error('Injection target must be in active workflow');
      const oldSrc=model.nodes.find(n=>n.id===old.sourceId), oldDst=model.nodes.find(n=>n.id===old.targetId);
      const id='n.scenario.'+inj.kind.toLowerCase();
      if(model.nodes.some(n=>n.id===id)) throw new Error('Duplicate injected ID');
      const mid={id,name:inj.kind+' · 实验',building:oldSrc.building,floor:oldSrc.floor,room:'Scenario',role:'Decision',kind:'Experiment',verification:'Hypothetical',description:'仅存在于场景差分中，非正式架构',ports:[{id:id+'.in',type:inj.portType,direction:'in'},{id:id+'.out',type:inj.portType,direction:'out'}],behavior:inj.behavior,sourcePath:null,evidenceIds:[]};
      model.nodes.push(mid);
      const ea={...old,id:old.id+'.A',targetId:id,targetPortId:id+'.in',verification:'Hypothetical',evidenceIds:[]};
      const eb={...old,id:old.id+'.B',sourceId:id,sourcePortId:id+'.out',verification:'Hypothetical',evidenceIds:[]};
      model.edges=model.edges.filter(e=>e.id!==old.id).concat([ea,eb]);
      model.workflow=model.workflow.flatMap(x=> x===old.id?[ea.id,eb.id]:[x]);
      delta.addedNodes.push(id);delta.addedEdges.push(ea.id,eb.id);delta.rewrittenEdges.push({old:old.id,replacement:[ea.id,eb.id]});
    }
    validateModel(model);return {model,delta};
  }
  function simulate(base, caseDef, intervention={}) {
    const {model,delta}=applyScenario(base,intervention), emap=new Map(model.edges.map(x=>[x.id,x]));
    const badN=new Set(intervention.disabledNodes||[]),badE=new Set(intervention.disabledEdges||[]);
    const trace=[], states=Object.fromEntries(model.nodes.map(n=>[n.id,'Idle']));
    let t=0,sequence=0,attempts={},terminal='COMPLETED',reason='';
    const add=(type,nodeId,edgeId,detail)=>{trace.push({eventId:'evt.'+String(++sequence).padStart(3,'0'),caseId:caseDef.id,logicalTime:t,sequence,type,nodeId,edgeId:edgeId||null,detail:detail||''});};
    const firstEdge=emap.get(model.workflow[0]);
    states[firstEdge.sourceId]='Running';add('Start',firstEdge.sourceId,null,'synthetic model input');
    for(const edgeId of model.workflow){
      const edge=emap.get(edgeId), dst=model.nodes.find(x=>x.id===edge.targetId);
      if(badE.has(edgeId) || badE.has(edgeId.split('.')[0]) || badN.has(edge.sourceId) || badN.has(edge.targetId)) {
        t+=1;states[edge.sourceId]='Blocked';terminal='BLOCKED';reason='禁用模块/关系阻断 '+edgeId;add('Blocked',edge.sourceId,edgeId,reason);break;
      }
      t+=2;states[edge.sourceId]='Completed';states[edge.targetId]='Running';add('Transit',edge.targetId,edgeId,edge.label||'event');
      if(dst.id.startsWith('n.scenario.')){
        t+=1;add('Rule',dst.id,null,dst.name+': '+dst.behavior);
        if(dst.behavior==='reject'){states[dst.id]='Failed';terminal='REJECTED';reason='Validator Guard rejected';add('Failure',dst.id,null,reason);break;}
      }
      if(caseDef.fault && dst.id==='n.'+caseDef.fault){
        const count=attempts[dst.id]||0;attempts[dst.id]=count+1;
        t+=1;add('Failure',dst.id,edgeId,caseDef.fault==='payment'?'payment_unknown':'guard rejected');
        if(count<caseDef.retry){ t+=3;add('Retry',dst.id,edgeId,'controlled test retry '+(count+1));t+=2;add('Recovery',dst.id,edgeId,'simulated successful retry');}
        else{states[dst.id]='Failed';terminal=dst.id==='n.payment'?'UNKNOWN':'FAILED';reason=caseDef.fault==='payment'?'payment_unknown：必须对账，不可重新扣款':'模型中的守卫条件拒绝';break;}
      }
    }
    if(terminal==='COMPLETED'){t+=1;add('Completed',model.workflow.length?emap.get(model.workflow.at(-1)).targetId:null,null,'logical terminal success');}
    return {model,delta,trace,states,outcome:{status:terminal,reason,logicalTime:t,eventCount:trace.length},scenarioCase:caseDef.id};
  }
  function compare(a,b){
    const canon=e=>JSON.stringify([e.logicalTime,e.type,e.nodeId,e.edgeId,e.detail]);
    let first=null;const m=Math.max(a.trace.length,b.trace.length);
    for(let i=0;i<m;i++) if(canon(a.trace[i])!==canon(b.trace[i])){first={index:i,baseline:a.trace[i]||null,intervention:b.trace[i]||null};break;}
    const impacted=first?[...a.trace.slice(first.index),...b.trace.slice(first.index)]:[];
    const affectedNodes=[...new Set(impacted.map(x=>x.nodeId).filter(Boolean))];
    const affectedEdges=[...new Set(impacted.map(x=>x.edgeId).filter(Boolean))];
    return {hasDifference:first!==null||a.outcome.status!==b.outcome.status,firstDivergence:first,affectedNodes,affectedEdges,baseline:a.outcome,intervention:b.outcome};
  }
  return {validateModel,applyScenario,simulate,compare};
});
