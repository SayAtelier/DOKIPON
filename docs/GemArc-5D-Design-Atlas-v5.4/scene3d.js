/* GemArc 5D / Offline unified LHC scene. One canonical graph, one spatial model,
 * three modes: 3D static, 4D timeline, 5D causal counterfactual. All geometry uses
 * +X right, +Y up, +Z forward, camera initially at negative Z looking toward +Z.
 * Native WebGL draws faces and routed edges; canvas overlay adds labels / hit help. */
(function(){'use strict';
const clamp=(v,a,b)=>Math.max(a,Math.min(b,v));
const V={add:(a,b)=>a.map((x,i)=>x+b[i]),sub:(a,b)=>a.map((x,i)=>x-b[i]),scale:(a,t)=>a.map(x=>x*t),dot:(a,b)=>a.reduce((s,x,i)=>s+x*b[i],0),cross:(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]],unit:a=>V.scale(a,1/(Math.hypot(...a)||1))};
const COLORS={floor:'#15414e',room:'#214b50',roomLine:'#6bb3a5',edge:'#6786b3',green:'#77efd3',blue:'#88b2ff',gold:'#f3cc7f',red:'#f38c95',muted:'#455366',background:'#0b1424'};
const hex=(h,a=1)=>{h=h.replace('#','');return [parseInt(h.slice(0,2),16)/255,parseInt(h.slice(2,4),16)/255,parseInt(h.slice(4,6),16)/255,a]};
const rgb=(h,light=1)=>{let c=hex(h);return c.slice(0,3).map(x=>clamp(x*light,0,1)).concat(1)};
const eventMap={gateway:'f-transport',webclient:'f-web',catalog:'f-catalog-repo',inventory:'f-checkout-state',payment:'f-payment-boundary',orders:'f-orders-ui',notifier:'f-orders-ui',account:'f-session',store:'f-storage'};
const specs=[
 {id:'experience',label:'F3 · Flutter 体验 / 渠道',y:390,nodeIds:['f-app','f-shell','f-design','f-home-ui','f-catalog-ui','f-fav-ui','f-cart-ui','f-orders-ui','f-mobile','f-web','f-line']},
 {id:'domain',label:'F2 · Application / Domain',y:200,nodeIds:['f-session','f-catalog-repo','f-checkout-state']},
 {id:'infrastructure',label:'F1 · Platform / Proposed APIs',y:10,nodeIds:['f-storage','f-transport','f-identity-backend','f-commerce-backend','f-payment-boundary']}
];
const layout=()=>{const nodePositions={},rooms=[];for(let f=0;f<specs.length;f++){const floor=specs[f];floor.nodeIds.forEach((id,i)=>{let roomIndex=Math.floor(i/4),slot=i%4;let x=-320+roomIndex*320 + (slot%2?76:-76),z=(Math.floor(slot/2)?78:-78);nodePositions[id]=[x,floor.y+26,z];});for(let roomIndex=0;roomIndex<Math.ceil(floor.nodeIds.length/4);roomIndex++){rooms.push({id:`${floor.id}:r${roomIndex}`,floor:floor.id,center:[-320+roomIndex*320,floor.y+28,0],ids:floor.nodeIds.slice(roomIndex*4,roomIndex*4+4)})}}return {nodePositions,rooms,floors:specs}};
const world=layout();
const state={yaw:-25,pitch:24,zoom:1400,exploded:true,visibleFloor:'all',focus:null};
function camera(w,h){const target=[0,215,0],a=state.yaw*Math.PI/180,b=state.pitch*Math.PI/180,dist=state.zoom;
 const eye=V.add(target,[Math.sin(a)*Math.cos(b)*dist,Math.sin(b)*dist,-Math.cos(a)*Math.cos(b)*dist]);
 const forward=V.unit(V.sub(target,eye)),right=V.unit(V.cross([0,1,0],forward)),up=V.unit(V.cross(forward,right));
 const focal=(h*.5)/Math.tan(44*Math.PI/360);return {eye,forward,right,up,focal,w,h};}
function project(p,c){const v=V.sub(p,c.eye),z=V.dot(v,c.forward);if(z<=10)return null;return {x:c.w*.5+c.focal*V.dot(v,c.right)/z,y:c.h*.5-c.focal*V.dot(v,c.up)/z,z};}
function exposed(p,floor){return [p[0],p[1]+(state.exploded?({experience:80,domain:35,infrastructure:0}[floor]||0):0),p[2]];}
function viewPositions(){const r={};for(const floor of specs){for(const id of floor.nodeIds)r[id]=exposed(world.nodePositions[id],floor.id);}return r;}
function makeGL(canvas){const gl=canvas.getContext('webgl',{alpha:false,antialias:true,preserveDrawingBuffer:true});if(!gl)return null;
 const vs='attribute vec4 aPosition; attribute vec4 aColor; varying vec4 vColor; void main(){gl_Position=aPosition;vColor=aColor;}';
 const fs='precision mediump float; varying vec4 vColor;void main(){gl_FragColor=vColor;}';
 function shader(type,s){let sh=gl.createShader(type);gl.shaderSource(sh,s);gl.compileShader(sh);if(!gl.getShaderParameter(sh,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(sh));return sh}
 const program=gl.createProgram();gl.attachShader(program,shader(gl.VERTEX_SHADER,vs));gl.attachShader(program,shader(gl.FRAGMENT_SHADER,fs));gl.linkProgram(program);
 if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));gl.useProgram(program);
 let buf=gl.createBuffer(),pos=gl.getAttribLocation(program,'aPosition'),color=gl.getAttribLocation(program,'aColor');gl.bindBuffer(gl.ARRAY_BUFFER,buf);gl.enableVertexAttribArray(pos);gl.enableVertexAttribArray(color);gl.vertexAttribPointer(pos,4,gl.FLOAT,false,32,0);gl.vertexAttribPointer(color,4,gl.FLOAT,false,32,16);gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);return gl;
}
function draw3D(canvas,overlay,options={}){if(!canvas||!overlay)return;const W=canvas.clientWidth||950,H=canvas.clientHeight||510,dpr=Math.min(window.devicePixelRatio||1,2);canvas.width=Math.round(W*dpr);canvas.height=Math.round(H*dpr);overlay.width=canvas.width;overlay.height=canvas.height;
 const gl=canvas._gemGl||(canvas._gemGl=makeGL(canvas));if(!gl){drawFallback2d(canvas,overlay,options);return}
 gl.viewport(0,0,canvas.width,canvas.height);gl.clearColor(.035,.061,.105,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);
 const cam=camera(W,H),vertices=[],lineVerts=[],poses=viewPositions(),nodes=window.GEMARC_DESIGN_DATA.designGraph.nodes,edges=window.GEMARC_DESIGN_DATA.designGraph.edges;
 const focus=state.focus,frame=options.frame||null,mode=options.mode||'3d',active=frame?eventMap[frame.nodeId]||null:null;
 const nodeVisible=id=>{let f=specs.find(fl=>fl.nodeIds.includes(id));return state.visibleFloor==='all'||f?.id===state.visibleFloor};
 const related=new Set();if(focus){if(focus.kind==='node')related.add(focus.id);let ed=focus.kind==='edge'?edges.find(e=>e.id===focus.id):null;if(ed){related.add(ed.sourceId);related.add(ed.targetId)}edges.forEach(e=>{if(related.has(e.sourceId)||related.has(e.targetId)){related.add(e.sourceId);related.add(e.targetId)}})}
 const fade=id=>focus&&!related.has(id)?.19:!nodeVisible(id)?.14:1;
 function packed(v,c,out){let p=project(v,cam);if(!p)return null;out.push(2*p.x/W-1,1-2*p.y/H,clamp((p.z-10)/4000*2-1,-1,1),1,...c);return p}
 function tri(a,b,c,co,shade=1){[a,b,c].forEach(v=>packed(v,rgb(co,shade),vertices))}
 function segment(a,b,col,width=2){const p=project(a,cam),q=project(b,cam);if(!p||!q)return;let dx=q.x-p.x,dy=q.y-p.y,len=Math.hypot(dx,dy)||1,nx=-dy/len*width*.5,ny=dx/len*width*.5;
 const z=(p.z+q.z)/2;const arr=[[p.x+nx,p.y+ny,p.z],[p.x-nx,p.y-ny,p.z],[q.x+nx,q.y+ny,q.z],[p.x-nx,p.y-ny,p.z],[q.x-nx,q.y-ny,q.z],[q.x+nx,q.y+ny,q.z]];
 for(const v of arr)vertices.push(2*v[0]/W-1,1-2*v[1]/H,clamp((v[2]-10)/4000*2-1,-1,1),1,...hex(col));}
 function box(center,w,h,d,col){const [x,y,z]=center,pts=[[x-w/2,y-h/2,z-d/2],[x+w/2,y-h/2,z-d/2],[x+w/2,y+h/2,z-d/2],[x-w/2,y+h/2,z-d/2],[x-w/2,y-h/2,z+d/2],[x+w/2,y-h/2,z+d/2],[x+w/2,y+h/2,z+d/2],[x-w/2,y+h/2,z+d/2]];
 const sides=[[0,1,2,3],[4,7,6,5],[0,4,5,1],[3,2,6,7],[1,5,6,2],[0,3,7,4]];
 sides.forEach((s,i)=>{tri(pts[s[0]],pts[s[1]],pts[s[2]],col,[.6,.84,.88,1,.69,.74][i]);tri(pts[s[0]],pts[s[2]],pts[s[3]],col,[.6,.84,.88,1,.69,.74][i])});return pts;}
 // World-space axes are left-handed; RGB = X/Y/Z and Z increases into scene.
 segment([-510,-12,-290],[-400,-12,-290],'#ee7485',3);segment([-510,-12,-290],[-510,98,-290],'#69dea5',3);segment([-510,-12,-290],[-510,-12,-180],'#72a5f8',3);
 const frameY={};for(const floor of specs){const p=exposed([0,floor.y,0],floor.id);frameY[floor.id]=p[1];let opacity=state.visibleFloor==='all'||state.visibleFloor===floor.id?1:.1;
 // floor platform; room prisms are semantic containers, not deployment claims
 if(opacity>.2){box([0,p[1]-14,0],1000,12,440,'#203f4a');for(const room of world.rooms.filter(r=>r.floor===floor.id)){let ctr=exposed(room.center,room.floor),roomCol=state.focus?.id===room.id?'#39695c':'#1a3f49';box([ctr[0],ctr[1]-15,0],285,9,204,roomCol);
 const x=ctr[0],y=ctr[1]+33,z=0;[[-140,-100],[140,-100],[140,100],[-140,100]].forEach(([bx,bz],i,a)=>{let t=a[(i+1)%4];segment([x+bx,y,z+bz],[x+t[0],y,z+t[1]],COLORS.roomLine,1.7);segment([x+bx,y-40,z+bz],[x+bx,y,z+bz],COLORS.roomLine,1)});}}}
 // Edges live in the same world as nodes (not an independent SVG overlay).
 const picks={nodes:[],edges:[]};edges.forEach((edge,i)=>{const p=poses[edge.sourceId],q=poses[edge.targetId];if(!p||!q||(!nodeVisible(edge.sourceId)&&!nodeVisible(edge.targetId)))return;
 let v=V.sub(q,p), len=Math.hypot(...v)||1, offset=(i%3-1)*7,normal=V.unit(V.cross(v,[0,1,0]));if(Math.hypot(...normal)<.1)normal=[1,0,0];let a=V.add(p,V.scale(normal,offset)),b=V.add(q,V.scale(normal,offset));
 let mid=V.add(V.scale(V.add(a,b),.5),[0,16,0]);const highlights=focus?.kind==='edge'&&focus.id===edge.id||(active&&(edge.sourceId===active||edge.targetId===active));let col=highlights?COLORS.gold:focus&&!related.has(edge.sourceId)&&!related.has(edge.targetId)?COLORS.muted:COLORS.edge;
 segment(a,mid,col,highlights?5:2.5);segment(mid,b,col,highlights?5:2.5);
 const u=V.unit(V.sub(b,mid)),arrow=V.sub(b,V.scale(u,22));segment(arrow,b,col,highlights?6:3);const sideways=V.unit(V.cross(u,[0,1,0]));segment(V.add(arrow,V.scale(sideways,8)),b,col,2.5);segment(V.sub(arrow,V.scale(sideways,8)),b,col,2.5);
 picks.edges.push({id:edge.id,seg:[[a,mid],[mid,b]],sourceId:edge.sourceId,targetId:edge.targetId,semantic:edge.type});});
 nodes.forEach((n,i)=>{let p=poses[n.id];if(!p||!nodeVisible(n.id))return;
 let high=active===n.id,sel=focus?.kind==='node'&&focus.id===n.id,dim=fade(n.id),sz=high?34:sel?31:22;
 let co=(n.verification==='Hypothetical'? '#e9ba81' : n.kind==='Platform'?'#9fafff':n.kind==='Service'?'#ffd38b':COLORS.green);if(high&&mode==='5d'&&['rollback','isolate'].includes(frame.animation))co=COLORS.red;
 if(dim<.3)co='#354758';let t=[p[0],p[1]+sz,p[2]],b=[p[0],p[1]-sz,p[2]],l=[p[0]-sz,p[1],p[2]],r=[p[0]+sz,p[1],p[2]],f=[p[0],p[1],p[2]-sz],back=[p[0],p[1],p[2]+sz];
 // Octahedral gemstone encodes graph nodes; 3D semantic objects only
 [[t,l,f],[t,f,r],[t,r,back],[t,back,l],[b,f,l],[b,r,f],[b,back,r],[b,l,back]].forEach((v,j)=>tri(v[0],v[1],v[2],co,[1,.81,.74,.66,.6,.76,.58,.5][j]));
 if(high){let radius=38+Math.sin((Date.now()/400))*7,yy=p[1]-sz;for(let k=0;k<24;k++){let a=k*Math.PI/12,q=(k+1)*Math.PI/12;segment([p[0]+Math.cos(a)*radius,yy,p[2]+Math.sin(a)*radius],[p[0]+Math.cos(q)*radius,yy,p[2]+Math.sin(q)*radius],['rollback','isolate'].includes(frame.animation)?COLORS.red:COLORS.gold,3);}}
 picks.nodes.push({id:n.id,point:p,name:n.name,kind:n.kind,verification:n.verification});});
 // Depth-tested WebGL triangles, no external library or CDN.
 const data=new Float32Array(vertices);const pBuffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,pBuffer);gl.bufferData(gl.ARRAY_BUFFER,data,gl.STREAM_DRAW);const program=gl.getParameter(gl.CURRENT_PROGRAM),pAttr=gl.getAttribLocation(program,'aPosition'),cAttr=gl.getAttribLocation(program,'aColor');gl.vertexAttribPointer(pAttr,4,gl.FLOAT,false,32,0);gl.vertexAttribPointer(cAttr,4,gl.FLOAT,false,32,16);gl.drawArrays(gl.TRIANGLES,0,data.length/8);gl.deleteBuffer(pBuffer);
 const ctx=overlay.getContext('2d');ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,W,H);ctx.fillStyle='#94c5c8';ctx.font='11px system-ui';ctx.fillText('LHC +X →   +Y ↑   +Z forward · WebGL 3D / Canonical Graph',16,20);
 for(const fl of specs){let p=project([0,frameY[fl.id]+16,-228],cam);if(!p)continue;ctx.fillStyle=state.visibleFloor==='all'||state.visibleFloor===fl.id?'#9cdacb':'#4c6673';ctx.font='12px system-ui';ctx.fillText(fl.label,p.x-150,p.y-10)}
 drawSparseLabels(ctx,relevantLabels(picks.nodes.map(n=>{let p=project(n.point,cam);return {id:n.id,name:n.name,x:p?.x??-1e5,y:p?.y??-1e5}}),mode,active||state.focus?.id,W),W,H,active||state.focus?.id);ctx.textAlign='left';
 if(frame&&active){let n=picks.nodes.find(n=>n.id===active);if(n){let p=project(n.point,cam);if(p){ctx.fillStyle=mode==='5d'?'#f6c986':'#8ae9d4';ctx.font='bold 13px system-ui';ctx.fillText(`${mode} ${frame.ruleId} · ${frame.animation}`,clamp(p.x+20,20,W-245),clamp(p.y-25,32,H-20));}}}
 canvas._gemPicks=picks;canvas._gemCam=cam;canvas._gemRenderOptions=options;
}
function drawFallback2d(canvas,overlay,options={}){
 const W=canvas.clientWidth||950,H=canvas.clientHeight||510,dpr=Math.min(window.devicePixelRatio||1,2);canvas.width=W*dpr;canvas.height=H*dpr;const ctx=canvas.getContext('2d');if(!ctx)return;ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,W,H);ctx.fillStyle='#0a1625';ctx.fillRect(0,0,W,H);
 const cameraState=camera(W,H),positions=viewPositions(),d=window.GEMARC_DESIGN_DATA.designGraph,mode=options.mode||'3d',frame=options.frame||null,active=frame?eventMap[frame.nodeId]:null;
 const visible=id=>{const floor=specs.find(f=>f.nodeIds.includes(id));return state.visibleFloor==='all'||floor?.id===state.visibleFloor;};
 const selected=state.focus,related=new Set();if(selected){if(selected.kind==='node')related.add(selected.id);if(selected.kind==='edge'){let e=d.edges.find(e=>e.id===selected.id);if(e){related.add(e.sourceId);related.add(e.targetId)}}d.edges.forEach(e=>{if(related.has(e.sourceId)||related.has(e.targetId)){related.add(e.sourceId);related.add(e.targetId)}})}
 const fade=id=>selected&&!related.has(id)?.24:visible(id)?1:.12;
 const moveTo=(p)=>{const q=project(p,cameraState);if(q)ctx.lineTo(q.x,q.y);return q};
 function poly(points,fill,stroke='#4b837d',alpha=1){const projected=points.map(p=>project(p,cameraState));if(projected.some(p=>!p))return;ctx.save();ctx.globalAlpha=alpha;ctx.beginPath();ctx.moveTo(projected[0].x,projected[0].y);projected.slice(1).forEach(p=>ctx.lineTo(p.x,p.y));ctx.closePath();ctx.fillStyle=fill;ctx.fill();ctx.strokeStyle=stroke;ctx.lineWidth=1.4;ctx.stroke();ctx.restore()}
 function line(a,b,color,width=2,alpha=1){const p=project(a,cameraState),q=project(b,cameraState);if(!p||!q)return;ctx.save();ctx.globalAlpha=alpha;ctx.lineWidth=width;ctx.strokeStyle=color;ctx.beginPath();ctx.moveTo(p.x,p.y);ctx.lineTo(q.x,q.y);ctx.stroke();ctx.restore()}
 const floorColors=['#174852','#153e50','#16414b'];
 const ordered=specs.slice().sort((a,b)=>project([0,exposed([0,a.y,0],a.id)[1],0],cameraState).z-project([0,exposed([0,b.y,0],b.id)[1],0],cameraState).z).reverse();
 for(const floor of ordered){let y=exposed([0,floor.y,0],floor.id)[1];const alpha=state.visibleFloor==='all'||state.visibleFloor===floor.id?1:.13;
 poly([[-485,y,-215],[485,y,-215],[485,y,215],[-485,y,215]],floorColors[specs.indexOf(floor)],'#5ec8b0',alpha);
 for(const room of world.rooms.filter(r=>r.floor===floor.id)){let r=exposed(room.center,room.floor),x=r[0],z=0;poly([[x-143,y+5,z-110],[x+143,y+5,z-110],[x+143,y+5,z+110],[x-143,y+5,z+110]],'#24555b','#488d86',alpha);
 [[-143,-110],[143,-110],[143,110],[-143,110]].forEach(([dx,dz],i,ar)=>{let end=ar[(i+1)%4];line([x+dx,y+4,z+dz],[x+dx,y+53,z+dz],'#6eacaa',1.1,alpha);line([x+dx,y+53,z+dz],[x+end[0],y+53,z+end[1]],'#6eacaa',1.1,alpha)});}
 let label=project([-465,y+13,-216],cameraState);if(label){ctx.fillStyle='#b8f3df';ctx.font='bold 12px system-ui';ctx.fillText(floor.label,label.x,label.y-9);}}
 const picks={nodes:[],edges:[]};
 d.edges.forEach((e,i)=>{let p=positions[e.sourceId],q=positions[e.targetId];if(!p||!q||(!visible(e.sourceId)&&!visible(e.targetId)))return;let t=V.scale(V.add(p,q),.5);t[1]+=28;const hi=selected?.kind==='edge'&&selected.id===e.id||(active&&(e.sourceId===active||e.targetId===active)),alpha=Math.min(fade(e.sourceId),fade(e.targetId));const col=hi?'#ffda8c':'#7d9ac7';line(p,t,col,hi?4:2,alpha);line(t,q,col,hi?4:2,alpha);
 let pr=project(t,cameraState),qr=project(q,cameraState);if(pr&&qr){let ang=Math.atan2(qr.y-pr.y,qr.x-pr.x);ctx.save();ctx.globalAlpha=alpha;ctx.strokeStyle=col;ctx.lineWidth=2;ctx.beginPath();ctx.moveTo(qr.x-13*Math.cos(ang-.44),qr.y-13*Math.sin(ang-.44));ctx.lineTo(qr.x,qr.y);ctx.lineTo(qr.x-13*Math.cos(ang+.44),qr.y-13*Math.sin(ang+.44));ctx.stroke();ctx.restore()}
 picks.edges.push({id:e.id,seg:[[p,t],[t,q]],semantic:e.type});});
 for(const n of d.nodes){let point=positions[n.id];if(!point||!visible(n.id))continue;const p=project(point,cameraState);if(!p)continue;let high=active===n.id,sel=selected?.id===n.id,alpha=fade(n.id),r=high?25:sel?23:17;
 let col=n.verification==='Hypothetical'?'#ffcc8a':n.kind==='Platform'?'#a6b4ff':'#70e4cc';if(high&&mode==='5d'&&['rollback','isolate'].includes(frame.animation))col='#f58fa0';
 ctx.save();ctx.globalAlpha=alpha;ctx.shadowBlur=high?18:0;ctx.shadowColor=col;ctx.fillStyle=col;ctx.strokeStyle='#dcfff6';ctx.lineWidth=1.4;ctx.beginPath();ctx.moveTo(p.x,p.y-r);ctx.lineTo(p.x+r*.9,p.y);ctx.lineTo(p.x,p.y+r);ctx.lineTo(p.x-r*.9,p.y);ctx.closePath();ctx.fill();ctx.stroke();ctx.shadowBlur=0;
 ctx.textAlign='center';if(high){ctx.beginPath();ctx.arc(p.x,p.y,32,0,Math.PI*2);ctx.strokeStyle=col;ctx.lineWidth=2;ctx.stroke()}ctx.restore();
 picks.nodes.push({id:n.id,point,name:n.name});}
 drawSparseLabels(ctx,relevantLabels(picks.nodes.map(n=>{let p=project(n.point,cameraState);return {id:n.id,name:d.nodes.find(x=>x.id===n.id)?.name||n.id,x:p?.x??-1e5,y:p?.y??-1e5}}),mode,active||selected?.id,W),W,H,active||selected?.id);ctx.textAlign='left';ctx.fillStyle='#84b4b8';ctx.font='11px system-ui';ctx.fillText('LHC +X →   +Y ↑   +Z forward · 3D Projection fallback (WebGL disabled)',16,20);
 if(active){let p=project(positions[active],cameraState);if(p){ctx.fillStyle='#f9d799';ctx.font='bold 12px system-ui';ctx.fillText(`${mode} ${frame?.ruleId||''} ${frame?.animation||''}`,Math.min(p.x+20,W-220),Math.max(40,p.y-20));}}
 canvas._gemCam=cameraState;canvas._gemPicks=picks;canvas._gemRenderOptions=options;
 const ol=overlay.getContext('2d');ol.clearRect(0,0,overlay.width,overlay.height);
}
function relevantLabels(items,mode,focusId,W){
  // Render a small, task-relevant label set without dropping any graph entity or hit region.
  // The inspector always retains complete names, IDs, ports, evidence and relationships.
  const limit=mode==='3d'?(W<680?5:10):(W<680?3:6);
  const priority=['f-app','f-transport','f-session','f-catalog-repo','f-checkout-state','f-payment-boundary','f-commerce-backend','f-orders-ui','f-mobile','f-line','f-storage'];
  return items.slice().sort((a,b)=>{
    if(a.id===focusId)return -1;if(b.id===focusId)return 1;
    const ai=priority.indexOf(a.id),bi=priority.indexOf(b.id);
    return (ai<0?999:ai)-(bi<0?999:bi)||a.id.localeCompare(b.id);
  }).slice(0,limit);
}
function drawSparseLabels(ctx,items,W,H,focusId){
 const busy=[];const sorted=items.slice().sort((a,b)=>(a.id===focusId?0:1)-(b.id===focusId?0:1));
 for(const n of sorted){const name=n.name.length>18?n.name.slice(0,17)+'…':n.name;ctx.font=n.id===focusId?'bold 12px system-ui':'11px system-ui';const width=ctx.measureText(name).width+12;const x=clamp(n.x,W?width/2+7:0,W-width/2-7);let location=null;
 for(const dy of [36,52,-37,68,-52]){let y=n.y+dy,box={x:x-width/2,y:y-13,w:width,h:19};if(box.y<28||box.y+box.h>H-6)continue;if(!busy.some(t=>!(box.x+box.w<t.x||t.x+t.w<box.x||box.y+box.h<t.y||t.y+t.h<box.y))){location={x,y,box};break}}
 if(!location)continue;busy.push(location.box);ctx.save();ctx.fillStyle='rgba(5,18,29,.78)';ctx.fillRect(location.box.x,location.box.y,location.box.w,location.box.h);ctx.textAlign='center';ctx.font=n.id===focusId?'bold 12px system-ui':'11px system-ui';ctx.fillStyle=n.id===focusId?'#fff1bb':'#d7e9ed';ctx.fillText(name,location.x,location.y);ctx.restore();}
}
function distancePointSeg(p,a,b){let dx=b.x-a.x,dy=b.y-a.y,t=clamp(((p.x-a.x)*dx+(p.y-a.y)*dy)/(dx*dx+dy*dy||1),0,1);return Math.hypot(p.x-(a.x+dx*t),p.y-(a.y+dy*t));}
function pick(canvas,x,y){let cam=canvas._gemCam,ps=canvas._gemPicks;if(!cam||!ps)return null;let candidates=[];for(const n of ps.nodes){let p=project(n.point,cam);if(p&&Math.hypot(p.x-x,p.y-y)<24)candidates.push({kind:'node',id:n.id,dist:Math.hypot(p.x-x,p.y-y)})}
 if(candidates.length)return candidates.sort((a,b)=>a.dist-b.dist)[0];
 for(const e of ps.edges){let d=Math.min(...e.seg.map(s=>{let a=project(s[0],cam),b=project(s[1],cam);return a&&b?distancePointSeg({x,y},a,b):99999}));if(d<14)candidates.push({kind:'edge',id:e.id,dist:d})}return candidates.sort((a,b)=>a.dist-b.dist)[0]||null;}
function attach(container,options={}){if(!container)return null;let canvas=container.querySelector('canvas.gem-webgl'),overlay=container.querySelector('canvas.gem-overlay');if(!canvas){container.innerHTML='<canvas class="gem-webgl" aria-label="WebGL 三维 Node-Edge 语义空间"></canvas><canvas class="gem-overlay" aria-hidden="true"></canvas>';canvas=container.querySelector('canvas.gem-webgl');overlay=container.querySelector('canvas.gem-overlay');
 let drag=null;container.addEventListener('pointerdown',e=>{if(e.target.tagName!=='CANVAS')return;drag={x:e.clientX,y:e.clientY,yaw:state.yaw,pitch:state.pitch,moved:false};container.setPointerCapture?.(e.pointerId)});
 container.addEventListener('pointermove',e=>{if(!drag)return;const dx=e.clientX-drag.x,dy=e.clientY-drag.y;if(Math.hypot(dx,dy)>4)drag.moved=true;state.yaw=clamp(drag.yaw+dx*.35,-170,170);state.pitch=clamp(drag.pitch-dy*.27,-70,70);renderAll()});
 container.addEventListener('pointerup',e=>{if(!drag)return;let dragged=drag.moved;drag=null;if(dragged)return;const rect=container.getBoundingClientRect(),p=pick(canvas,e.clientX-rect.left,e.clientY-rect.top);state.focus=p?{kind:p.kind,id:p.id}:null;renderAll();container.dispatchEvent(new CustomEvent('gemarc-select',{bubbles:true,detail:state.focus}))});
 container.addEventListener('wheel',e=>{e.preventDefault();state.zoom=clamp(state.zoom*(e.deltaY>0?1.08:.92),680,2700);renderAll()},{passive:false});}
 canvas._gemOptions=options;draw3D(canvas,overlay,options);return canvas;}
function renderAll(){document.querySelectorAll('.gem-scene').forEach(el=>{let c=el.querySelector('canvas.gem-webgl'),o=el.querySelector('canvas.gem-overlay');if(c&&o)draw3D(c,o,c._gemOptions||{})})}
function update(el,options){let c=el?.querySelector('canvas.gem-webgl'),o=el?.querySelector('canvas.gem-overlay');if(c&&o){c._gemOptions=options;draw3D(c,o,options)}}
window.GemArcScene={state,world,eventMap,project,camera,attach,update,renderAll,pick,viewPositions};
})();
