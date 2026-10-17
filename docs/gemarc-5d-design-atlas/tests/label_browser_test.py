import json,pathlib
from playwright.sync_api import sync_playwright
root=pathlib.Path(__file__).resolve().parents[1]
out={};errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox','--enable-unsafe-swiftshader'])
 page=browser.new_page(viewport={'width':1440,'height':900},device_scale_factor=1)
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.set_content((root/'index.html').read_text(),wait_until='load');page.wait_for_timeout(500)
 out['initial_labels_total']=page.locator('#spatialLabels .spatial-label').count()
 out['initial_nodes']=page.locator('#spatialLabels .spatial-label.node').count()
 out['initial_edges']=page.locator('#spatialLabels .spatial-label.edge').count()
 out['visible_names']=[a['title'] for a in page.locator('#spatialLabels .spatial-label').evaluate_all('(els)=>els.map(x=>({title:x.title}))')]
 page.screenshot(path=str(root/'tests/label_desktop.png'))
 positions=page.locator('#spatialLabels .spatial-label').evaluate_all('(els)=>Object.fromEntries(els.map(x=>[x.dataset.labelId, [parseFloat(x.style.left),parseFloat(x.style.top)]]))')
 box=page.locator('#space3d').bounding_box()
 page.mouse.move(box['x']+box['width']*.45,box['y']+box['height']*.45)
 page.mouse.down();page.mouse.move(box['x']+box['width']*.67,box['y']+box['height']*.55,steps=14);page.mouse.up();page.wait_for_timeout(400)
 positions2=page.locator('#spatialLabels .spatial-label').evaluate_all('(els)=>Object.fromEntries(els.map(x=>[x.dataset.labelId, [parseFloat(x.style.left),parseFloat(x.style.top)]]))')
 out['labels_move_with_orbit']=any(v!=positions2.get(k) for k,v in positions.items() if k in positions2)
 out['labels_upright']=all(not a.get('transform') or a['transform']=='none' for a in page.locator('.spatial-label').evaluate_all('(els)=>els.map(x=>({transform:getComputedStyle(x).transform}))'))
 # Exercise additional right/left/top viewing directions using the actual pointer orbit interaction.
 angle_checks=[]
 for dx,dy in [(220,-90),(-235,120),(160,140),(-220,-125)]:
  box=page.locator('#space3d').bounding_box()
  x=box['x']+45;y=box['y']+44
  page.mouse.move(x,y);page.mouse.down();page.mouse.move(x+dx,y+dy,steps=12);page.mouse.up()
  page.wait_for_timeout(160)
  arr=page.locator('#spatialLabels .spatial-label.node, #spatialLabels .spatial-label.edge').evaluate_all('(els)=>els.map(e=>({id:e.dataset.labelId,x:parseFloat(e.style.left),y:parseFloat(e.style.top),transform:getComputedStyle(e).transform}))')
  angle_checks.append({'rendered':len(arr),'readable':all(e['transform']=='none' for e in arr)})
 out['multi_angle_checks']=angle_checks
 page.locator('#labelMode').select_option('all');page.wait_for_timeout(900)
 out['all_nodes']=page.locator('#spatialLabels .spatial-label.node').count()
 out['all_edges']=page.locator('#spatialLabels .spatial-label.edge').count()
 edge_label=page.locator('#spatialLabels .spatial-label.edge').first
 edge_label.click(force=True, timeout=4000)
 out['3d_select_by_label']=page.locator('#insType').inner_text().startswith('Graph Edge')
 page.locator('[data-dim="2"]').click()
 out['2d_nodes']=page.locator('#graphSvg [data-id^="n."]').count()
 out['2d_edges']=page.locator('#graphSvg [data-id^="e"]').count()
 out['2d_edge_names']=page.locator('#graphSvg .graph-edge-label').count()
 if out['2d_edge_names']:
  page.locator('#graphSvg .graph-edge-label').first.click(force=True)
  out['select_by_label']=page.locator('#insType').inner_text().startswith('Graph Edge')
 page.locator('[data-dim="5"]').click()
 page.locator('#kindPick').select_option('Validator');page.locator('#disableEdge').select_option('e04');page.get_by_role('button',name='插入并重算').click();page.wait_for_timeout(400)
 out['injected_node_named']=page.locator('#spatialLabels .spatial-label[data-label-id="n.scenario.validator"]').count()
 out['injected_edges_named']=page.locator('#spatialLabels .spatial-label[data-label-id="e04.A"], #spatialLabels .spatial-label[data-label-id="e04.B"]').count()
 page.screenshot(path=str(root/'tests/label_causal.png'))
 mobile=browser.new_page(viewport={'width':390,'height':844},device_scale_factor=1)
 mobile.on('pageerror',lambda e:errors.append(str(e)))
 mobile.set_content((root/'index.html').read_text(),wait_until='load');mobile.wait_for_timeout(400)
 out['mobile_labels']=mobile.locator('#spatialLabels .spatial-label').count()
 out['mobile_horizontal_overflow']=mobile.evaluate('document.documentElement.scrollWidth>innerWidth')
 mobile.screenshot(path=str(root/'tests/label_mobile.png'))
 out['js_errors']=errors
 browser.close()
(root/'tests/label_results.json').write_text(json.dumps(out,ensure_ascii=False,indent=2))
print(json.dumps(out,ensure_ascii=False,indent=2))
