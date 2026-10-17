const test=require('node:test'),assert=require('node:assert/strict');
const L=require('../source/labels.js');
const m=require('../models/architecture.json');
test('every node and edge has a meaningful name',()=>{
 const nodes=new Map(m.nodes.map(n=>[n.id,n]));
 for(const n of nodes.values())assert.ok(L.nameNode(n).trim().length>2,n.id);
 for(const e of m.edges)assert.ok(L.nameEdge(e,id=>nodes.get(id)).trim().length>2,e.id);
});
test('rewritten graph edges retain distinct readable names',()=>{
 assert.equal(L.nameEdge({id:'e04.A',label:'write cart'}),'write cart · A');
 assert.equal(L.nameEdge({id:'e04.B',label:'write cart'}),'write cart · B');
});
test('labels avoid overlapping and staying outside viewport',()=>{
 const items=[{id:'n1',kind:'node',x:100,y:90,w:90,h:24,priority:10},{id:'e1',kind:'edge',x:100,y:90,from:{x:50,y:50},to:{x:170,y:130},w:110,h:22,priority:20},{id:'n2',kind:'node',x:130,y:90,w:80,h:24,priority:30}];
 const list=L.placeLabels(items,350,200),vis=list.filter(v=>v.visible);
 assert.ok(vis.length>=2);
 for(const r of vis){assert.equal(L.valid({x:r.left,y:r.top,w:r.w,h:r.h},350,200),true)}
 for(let i=0;i<vis.length;i++)for(let j=i+1;j<vis.length;j++)assert.equal(L.rectsHit({x:vis[i].left,y:vis[i].top,w:vis[i].w,h:vis[i].h},{x:vis[j].left,y:vis[j].top,w:vis[j].w,h:vis[j].h}),false);
});
test('all mode reveals colliding names rather than making up identifiers',()=>{
 const base=Array.from({length:3},(_,i)=>({id:'edge'+i,kind:'node',x:65,y:50,w:100,h:26,priority:i}));
 const auto=L.placeLabels(base,135,140,'auto');const all=L.placeLabels(base,135,140,'all');assert.ok(all.filter(x=>x.visible).length>=auto.filter(x=>x.visible).length);
});
test('out-of-camera labels are hidden instead of pinned to wrong object',()=>{
 const off=[{id:'off',kind:'node',x:-100,y:-100,w:80,h:20}];assert.equal(L.placeLabels(off,400,300)[0].visible,false);
});

test('long CJK node names wrap without duplicated prefixes or box overflow',()=>{
 const name='库存预留校验服务与支付回滚处理协调模块';const lines=L.wrapText(name,70,84);
 assert.equal(lines.length,2);assert.ok(L.countWidth(lines[0])<=70);assert.ok(L.countWidth(lines[1])<=84);
 assert.ok(!lines[1].startsWith(lines[0]));assert.ok(L.fitText('長い日本語表示名',48).endsWith('…'));
});
