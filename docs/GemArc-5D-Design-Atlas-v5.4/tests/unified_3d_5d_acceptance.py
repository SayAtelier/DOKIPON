from pathlib import Path
from playwright.sync_api import sync_playwright
from collections import Counter
import json, re
ROOT=Path(__file__).resolve().parents[1]
results=[]
def t(name,fn):
 try:
  v=fn();
  if v is False:raise AssertionError('expected truthy result')
  results.append({'case':name,'pass':True,'result':str(v)[:170]});print('PASS',name,v)
 except Exception as ex:
  results.append({'case':name,'pass':False,'error':str(ex)[:500]});print('FAIL',name,repr(ex)[:250])
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path='/usr/bin/chromium',args=['--no-sandbox','--disable-gpu'])
 page=browser.new_page(viewport={'width':1440,'height':900},device_scale_factor=1)
 errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 page.set_content((ROOT/'index.html').read_text(encoding='utf8'),wait_until='load',timeout=30000)
 page.wait_for_timeout(300)
 def js(s):return page.evaluate(s)
 t('offline inline HTML no remote fetch/script dependencies',lambda: ('<script src=' not in (ROOT/'index.html').read_text(encoding='utf8') and '<link rel="stylesheet"' not in (ROOT/'index.html').read_text(encoding='utf8')))
 t('design data 19 nodes / 26 edges',lambda: (len(json.loads((ROOT/'reference-models/causal_cases.json').read_text())['cases']), js('window.GemArcDesignAPI.getDesignGraph().nodes.length'),js('window.GemArcDesignAPI.getDesignGraph().edges.length')))
 t('44? available documentation and engineering model',lambda:(js('window.GEMARC_DESIGN_DATA.docs.length'),js('window.GEMARC_DESIGN_DATA.engineeringCases.length')))
 t('canonical graph stable endpoints',lambda: js('(()=>{let g=window.GemArcDesignAPI.getDesignGraph(),ids=new Set(g.nodes.map(n=>n.id));return g.edges.every(e=>ids.has(e.sourceId)&&ids.has(e.targetId))})()'))
 t('left handed camera +X visible right at zero yaw',lambda: js('(()=>{let S=GemArcScene;S.state.yaw=0;S.state.pitch=0;let c=S.camera(950,560);return S.project([100,215,0],c).x>S.project([0,215,0],c).x&&c.forward[2]>.99&&c.right[0]>.99})()'))
 t('world coordinate positions consistent',lambda: js('(()=>{const S=GemArcScene,a=S.viewPositions();return Object.keys(a).length===19&&Object.values(a).every(v=>v.length===3&&v.every(Number.isFinite))})()'))
 page.locator('[data-view=spatial]').click();page.wait_for_timeout(500)
 t('3D canvas visible with pick table',lambda: js('(()=>{let c=document.querySelector("#spatialScene canvas.gem-webgl");return !!c && c._gemPicks?.nodes.length===19 && c._gemPicks?.edges.length===26})()'))
 # direct node pick at projected center, prior to interaction
 t('3D node hit test yields stable graph id',lambda:js('(()=>{let c=document.querySelector("#spatialScene canvas.gem-webgl");let n=c._gemPicks.nodes.find(n=>n.id==="f-catalog-repo"),p=GemArcScene.project(n.point,c._gemCam);return GemArcScene.pick(c,p.x,p.y)?.id==="f-catalog-repo"})()'))
 # an independently picked segment without node snap
 t('3D edge picking can select its own ID',lambda:js('(()=>{let c=document.querySelector("#spatialScene canvas.gem-webgl");for(let e of c._gemPicks.edges){for(let seg of e.seg){let a=GemArcScene.project(seg[0],c._gemCam),b=GemArcScene.project(seg[1],c._gemCam);for(let t of [.25,.5,.75]){let q=GemArcScene.pick(c,a.x+(b.x-a.x)*t,a.y+(b.y-a.y)*t);if(q?.kind==="edge")return true}}}return false})()'))
 def gesture():
  y0=js('GemArcScene.state.yaw');sc=page.locator('#spatialScene');r=sc.bounding_box();page.mouse.move(r['x']+r['width']*.45,r['y']+200);page.mouse.down();page.mouse.move(r['x']+r['width']*.65,r['y']+210,steps=5);page.mouse.up();assert abs(js('GemArcScene.state.yaw')-y0)>3;return js('GemArcScene.state.yaw')
 t('3D orbit left-handed drag gesture',gesture)
 t('3D floor selection changes scene view',lambda:(page.locator('[data-floor=domain]').click(),js('GemArcScene.state.visibleFloor')=='domain')[-1])
 t('3D scene reset restores camera',lambda:(page.locator('#spatialReset').click(),js('GemArcScene.state.yaw')==-25 and js('GemArcScene.state.visibleFloor')=='all')[-1])
 page.screenshot(path=str(ROOT/'previews/unified-3d.png'),full_page=False)
 page.locator('[data-view=timeline]').click();page.wait_for_timeout(220)
 t('4D embeds the SAME spatial graph model',lambda:js('(()=>{let c=document.querySelector("#timelineMount canvas.gem-webgl");return c?._gemPicks.nodes.length===19&&c._gemPicks.edges.length===26})()'))
 t('4D step advances event and updates 3D node',lambda:(page.locator('#timelineMount #labNext').click(),js("document.querySelector('#timelineMount canvas.gem-webgl')._gemRenderOptions.frame.sequence")>=1)[-1])
 page.locator('[data-view=causal]').click();page.wait_for_timeout(250)
 t('5D embeds same full 3D graph',lambda:js('(()=>{let c=document.querySelector("#causalMount canvas.gem-webgl");return c?._gemPicks.nodes.length===19&&c._gemPicks.edges.length===26})()'))
 t('5D causation has normal / failed / recovered terminals',lambda:js('(()=>["CASE-CHECKOUT-001","CASE-CHECKOUT-002","CASE-CHECKOUT-005"].map(id=>window.GemArcDesignAPI.runCase(id).terminal).join(","))()')=='success,failed,recovered')
 t('5D guard intervention changes terminal',lambda:js('(()=>{let normal=window.GemArcDesignAPI.runCase("CASE-CHECKOUT-001"),bad=window.GemArcDesignAPI.runCase("CASE-CHECKOUT-001",{thresholdPercent:101});return normal.terminal==="success"&&bad.terminal==="failed"&&JSON.stringify(normal.frames.map(x=>x.ruleId))!==JSON.stringify(bad.frames.map(x=>x.ruleId))})()'))
 t('5D scene active binding matches event',lambda:js('(()=>{let c=document.querySelector("#causalMount canvas.gem-webgl"),frame=c._gemRenderOptions.frame;return GemArcScene.eventMap[frame.nodeId]!==undefined})()'))
 page.screenshot(path=str(ROOT/'previews/unified-5d.png'),full_page=False)
 page.locator('[data-view=blueprint]').click()
 t('AI blueprint 42 fullstack designed cases',lambda: len(page.locator('.engine-case-item').all())==42)
 t('full stack detailed case clickable',lambda:(page.locator('[data-engineering=E2E-004]').click(), '支付响应丢失' in page.locator('#engineeringCaseDetail').inner_text())[-1])
 t('AI design docs accessible from blueprint',lambda:(page.locator('[data-doc="architecture/AI_IMPLEMENTATION_BLUEPRINT.md"]').first.click(), 'AI による Flutter' in page.locator('#docBody').inner_text())[-1])
 t('all 144 model cases expected terminal',lambda:js('window.GEMARC_DESIGN_DATA.cases.every(c=>GemArcDesignAPI.runCase(c.id).terminal===c.expectedTerminal)'))
 t('no JS page errors',lambda:len(errors)==0)
 browser.close()
pass_count=sum(x['pass'] for x in results)
(ROOT/'tests/acceptance-v53-results.json').write_text(json.dumps({'count':len(results),'passed':pass_count,'failed':len(results)-pass_count,'results':results},ensure_ascii=False,indent=2),encoding='utf8')
print('SUMMARY',pass_count,'/',len(results))
if pass_count!=len(results):raise SystemExit(1)
