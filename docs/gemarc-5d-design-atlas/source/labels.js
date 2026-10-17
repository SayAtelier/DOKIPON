/* Geometry-independent, screen-space label placement for 2D / 3D graph views.
   All layout decisions are cosmetic. They never mutate canonical Graph IDs. */
(function(root,make){const api=make();if(typeof module!=='undefined'&&module.exports)module.exports=api;root.GemLabels=api;})(typeof globalThis!=='undefined'?globalThis:this,function(){
 'use strict';
 const nameNode = n => String(n?.displayName||n?.name||n?.id||'Unknown Node');
 const nameEdge = (e,lookup) => {
   const src=lookup?.(e?.sourceId)?.name,dst=lookup?.(e?.targetId)?.name,derived=src&&dst?src+' → '+dst:'';
   const main=String(e?.displayName||e?.label||e?.name||e?.type||derived||e?.id||'Unknown Edge');
   return /\.([AB])$/.test(e?.id||'') ? main+' · '+RegExp.$1 : main;
 };
 const countWidth=(value)=>Array.from(String(value||'')).reduce((p,ch)=>p+(/[\u2E80-\u9FFF\uAC00-\uD7FF\uF900-\uFAFF]/u.test(ch)?12:/[ijl1 .]/.test(ch)?4.1:/[MW@]/.test(ch)?10.5:7),0);
 function fitText(value,maxWidth){const text=String(value??'');if(countWidth(text)<=maxWidth)return text;let output='';for(const ch of text){if(countWidth(output+ch+'…')>maxWidth)break;output+=ch;}return output+'…';}
 function wrapText(value,firstWidth=115,nextWidth=132){const text=String(value??'');if(countWidth(text)<=firstWidth)return [text];let first='';let i=0;for(const ch of text){if(countWidth(first+ch)>firstWidth)break;first+=ch;i++;}return [first,fitText(Array.from(text).slice(i).join(''),nextWidth)];}
 function rectsHit(a,b,pad=4){return a.x < b.x+b.w+pad && a.x+a.w+pad>b.x && a.y < b.y+b.h+pad && a.y+a.h+pad>b.y;}
 function valid(r,width,height,inset=7){return r.x>=inset&&r.y>=inset&&r.x+r.w<=width-inset&&r.y+r.h<=height-inset;}
 function anchorCandidates(item){
   const {x,y,w,h,kind,from,to,radius=13}=item;
   if(kind==='edge'&&from&&to){
     const dx=to.x-from.x,dy=to.y-from.y,len=Math.hypot(dx,dy)||1,nx=-dy/len,ny=dx/len;
     const options=[];
     for(const offset of [16,30,46,62,82,105])for(const t of [.5,.36,.64,.23,.77,.12,.88])for(const side of [1,-1]){
       const px=from.x+dx*t+nx*side*offset,py=from.y+dy*t+ny*side*offset;
       options.push({x:px-w/2,y:py-h/2,w,h});
     }
     options.push({x:x-w/2,y:y-h/2,w,h});return options;
   }
   const out=[];
   for(const offset of [radius+7,radius+22,radius+40,radius+65,radius+95]){
     out.push({x:x-w/2,y:y-offset-h,w,h},{x:x+offset,y:y-h/2,w,h},{x:x-w/2,y:y+offset,w,h},{x:x-offset-w,y:y-h/2,w,h});
     out.push({x:x+offset*.7,y:y-offset-h,w,h},{x:x-offset*.7-w,y:y+offset,w,h});
   }
   return out;
 }
 function placeLabels(items,width,height,mode='auto',reserved=[]){
   if(!Number.isFinite(width)||!Number.isFinite(height)||width<=0||height<=0)return [];
   const ordered=[...items].sort((a,b)=>(b.priority||0)-(a.priority||0)||String(a.id).localeCompare(String(b.id)));
   const placed=reserved.map(v=>({...v})),out=[];
   for(const item of ordered){
     const inView=Number.isFinite(item.x)&&Number.isFinite(item.y)&&item.x>=-5&&item.x<=width+5&&item.y>=-5&&item.y<=height+5;
     if(!inView||item.hidden){out.push({...item,visible:false,reason:'out-of-view'});continue;}
     const candidates=anchorCandidates(item).filter(r=>valid(r,width,height));
     if(!candidates.length){out.push({...item,visible:false,reason:'offscreen'});continue;}
     const clear=candidates.find(r=>!placed.some(other=>rectsHit(r,other)));
     let chosen=clear;
     if(!chosen&&mode==='all'){
       chosen=candidates.map(r=>({r,overlap:placed.reduce((sum,other)=>sum+(rectsHit(r,other)?1:0),0)})).sort((a,b)=>a.overlap-b.overlap)[0]?.r;
     }
     if(!chosen){out.push({...item,visible:false,reason:'collision'});continue;}
     placed.push(chosen);
     out.push({...item,visible:true,left:Math.round(chosen.x),top:Math.round(chosen.y)});
   }
   return out;
 }
 return {nameNode,nameEdge,countWidth,fitText,wrapText,rectsHit,valid,anchorCandidates,placeLabels};
});
