from playwright.sync_api import sync_playwright
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
# Browsing harness policy blocks file://; HTTP is used for interaction tests.
# App itself uses only relative local scripts and no fetch() for offline file opening.
import threading, functools, http.server
handler=functools.partial(http.server.SimpleHTTPRequestHandler,directory=str(ROOT))
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
URL='offline inline index.html (browser policy blocks file and localhost navigation)' 
results=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path='/usr/bin/chromium',args=['--no-sandbox','--disable-gpu'])
    page=browser.new_page(viewport={'width':1440,'height':900},device_scale_factor=1)
    errors=[]; page.on('pageerror',lambda e:errors.append(str(e)))
    page.set_content((ROOT/'index.html').read_text(encoding='utf8'),wait_until='load',timeout=30000)
    page.set_default_timeout(5000)
    page.wait_for_timeout(300)
    def test(name,fn):
        try:
            info=fn();results.append({'name':name,'pass':True,'info':str(info)[:120]});print('PASS',name,info)
        except Exception as ex:
            results.append({'name':name,'pass':False,'error':str(ex)[:350]});print('FAIL',name,str(ex)[:350])
    def initial():
        assert page.title().startswith('GemArc 5D')
        assert page.locator('#metricCases').inner_text()=='144'
        assert page.locator('#metricDocs').inner_text()=='33'
        return page.title()
    test('index direct file open',initial)
    def req():
        page.locator('.nav[data-view="requirements"]').click()
        assert page.locator('.req-item').count()==13
        page.locator('.req-item[data-feature="FR-APP-05"]').click()
        assert 'LIFF' in page.locator('#requirementDetail').inner_text()
        return '13 requirements and LINE acceptance'
    test('requirements and trace',req)
    def diagrams():
        page.locator('.nav[data-view="diagrams"]').click()
        assert page.locator('#graphLayer .g-node').count()==19
        assert page.locator('#graphLayer .g-edge').count()==26
        page.locator('#graphLayer [data-node="f-catalog-ui"]').evaluate('(e)=>e.dispatchEvent(new MouseEvent("click",{bubbles:true}))')
        assert 'f-catalog-ui' in page.locator('#graphInspector').inner_text()
        page.locator('#graphLayer [data-edge="fe-08"]').evaluate('(e)=>e.dispatchEvent(new MouseEvent("click",{bubbles:true}))')
        assert 'EDGE INSPECTOR' in page.locator('#graphInspector').inner_text()
        page.locator('button[data-diagram="er"]').click()
        assert 'PaymentIntent' in page.locator('#graphSvg').text_content()
        page.locator('button[data-diagram="uml"]').click()
        assert 'CatalogViewModel' in page.locator('#graphSvg').text_content()
        page.locator('button[data-diagram="evidence"]').click()
        assert 'Evidence Ledger' in page.locator('#graphSvg').text_content()
        return '19 design nodes, 26 edges; UML ER evidence'
    test('graph Node Edge UML ER evidence',diagrams)
    def spatial():
        page.locator('.nav[data-view="spatial"]').click()
        assert page.locator('#spatialSvg [data-room]').count()==9
        page.locator('#spatialSvg [data-room="f-catalog-ui"]').evaluate('(e)=>e.dispatchEvent(new MouseEvent("click",{bubbles:true}))')
        assert 'f-catalog-ui' in page.locator('#spatialInspector').inner_text()
        page.locator('button[data-floor="2"]').click()
        page.locator('#cameraRotate').fill('28')
        return '3 floors / 9 selectable rooms'
    test('spatial focus floor rotation',spatial)
    def workflow():
        page.locator('.nav[data-view="workflow"]').click()
        assert page.locator('#workflowSvg [data-wfnode]').count()==7
        assert page.locator('#workflowSvg [data-wfedge]').count()==8
        page.locator('#workflowValidate').click()
        assert '语义有效' in page.locator('#workflowValidationStatus').inner_text()
        page.locator('#workflowSvg [data-wfnode="w-payment"]').dispatch_event('pointerdown',{'pointerId':2,'clientX':500,'clientY':340})
        page.locator('#workflowSvg [data-wfnode="w-payment"]').dispatch_event('pointerup',{'pointerId':2,'clientX':500,'clientY':340})
        assert '支付授权' in page.locator('#workflowInspector').inner_text()
        return '7 workflow nodes 8 edges validated'
    test('workflow canvas semantic validation',workflow)
    def lab():
        page.locator('.nav[data-view="causal"]').click()
        assert page.locator('#labTrack .lab-step').count()>0
        base=page.evaluate("GemArcDesignAPI.runCase('CASE-CHECKOUT-001')")
        bad=page.evaluate("GemArcDesignAPI.runCase('CASE-CHECKOUT-001',{bOk:false})")
        recovered=page.evaluate("GemArcDesignAPI.runCase('CASE-CHECKOUT-001',{bOk:false,recoverable:true})")
        boundary=page.evaluate("GemArcDesignAPI.runCase('CASE-CHECKOUT-001',{thresholdPercent:101})")
        assert base['terminal']=='success' and bad['terminal']=='failed' and recovered['terminal']=='recovered' and boundary['terminal']=='failed'
        page.locator('#input-bOk').uncheck()
        assert 'failed' in page.locator('#labOutcome').inner_text()
        page.locator('#input-recoverable').check()
        assert 'recovered' in page.locator('#labOutcome').inner_text()
        assert '反事实对比' in page.locator('#labCompare').inner_text()
        page.locator('#labNext').click()
        assert '2 /' in page.locator('#labCounter').inner_text()
        return 'success, failed, recovered, boundary & delta re-computed'
    test('5D causal recalculation 4 branches',lab)
    def testcases():
        page.locator('.nav[data-view="cases"]').click()
        assert page.locator('#caseRows .case-row').count()==144
        page.locator('#caseTypeFilter').select_option('recovery')
        assert page.locator('#caseRows .case-row').count()==16
        page.locator('#caseSearch').fill('CHECKOUT')
        assert page.locator('#caseRows .case-row').count()==1
        page.locator('#caseRows .case-row').first.click()
        assert page.locator('#view-causal').evaluate('(e)=>getComputedStyle(e).display')=='block'
        return '144 categories, 16 recovery and case navigation'
    test('case library filters and 5D jump',testcases)
    def docs():
        page.locator('.nav[data-view="library"]').click()
        assert page.locator('.doc-list-item').count()==33
        page.locator('#docSearch').fill('詳細')
        assert page.locator('.doc-list-item').count()>=1
        page.locator('.doc-list-item').first.click()
        assert len(page.locator('#docBody').inner_text())>500
        return '33 docs embedded offline'
    test('IPA document reading',docs)
    def cross():
        page.locator('.nav[data-view="platforms"]').click()
        assert page.locator('.platform-card').count()==4
        page.locator('button[data-device="line"]').click()
        assert 'LINE MINI APP' in page.locator('#deviceStage').inner_text()
        page.locator('.nav[data-view="trace"]').click()
        assert page.locator('#traceRows tbody tr').count()==13
        return '4 platforms and 13 RTM mappings'
    test('Flutter LINE and RTM',cross)
    def search():
        page.locator('#globalSearchBtn').click()
        page.locator('#universalSearch').fill('CASE-LINE-003')
        assert page.locator('.search-result').count()>0
        page.keyboard.press('Enter')
        assert page.locator('#view-causal').evaluate('(e)=>getComputedStyle(e).display')=='block'
        return 'global search opens case'
    test('global search shortcut',search)
    def full_case_set():
        output=page.evaluate("""() => {
          const cases=GemArcDesignAPI.getCases(), outcomes={},failures=[];
          for(const c of cases){
            const a=GemArcDesignAPI.runCase(c.id);
            const b=GemArcDesignAPI.runCase(c.id);
            outcomes[a.terminal]=(outcomes[a.terminal]||0)+1;
            if(a.terminal!==c.expectedTerminal)failures.push(c.id+': '+a.terminal+' != '+c.expectedTerminal);
            if(JSON.stringify(a.frames)!==JSON.stringify(b.frames))failures.push(c.id+': non deterministic');
            if(!a.frames.length)failures.push(c.id+': empty trace');
            if(a.frames.length>24)failures.push(c.id+': step budget');
          }
          const g=GemArcDesignAPI.getDesignGraph(),nodeIds=new Set(g.nodes.map(n=>n.id));
          for(const e of g.edges) if(!nodeIds.has(e.sourceId)||!nodeIds.has(e.targetId))failures.push('graph endpoint '+e.id);
          return {count:cases.length,outcomes,failures,nodes:g.nodes.length,edges:g.edges.length};
        }""")
        assert output['count']==144 and not output['failures'],output
        return str(output['outcomes'])+'; stable '+str(output['nodes'])+' nodes '+str(output['edges'])+' edges'
    test('all 144 deterministic cases and graph integrity',full_case_set)
    def mobile():
        page.set_viewport_size({'width':390,'height':844})
        page.locator('#menuBtn').click()
        assert 'open' in page.locator('#sidebar').get_attribute('class')
        page.locator('.nav[data-view="overview"]').click()
        assert page.locator('#view-overview').is_visible()
        return '390px responsive navigation'
    test('mobile breakpoint navigation',mobile)
    page.set_viewport_size({'width':1440,'height':900})
    page.locator('.nav[data-view="overview"]').click()
    page.screenshot(path=str(ROOT/'previews'/'design-overview.png'),full_page=True)
    page.locator('.nav[data-view="diagrams"]').click()
    page.locator('button[data-diagram="graph"]').click()
    page.screenshot(path=str(ROOT/'previews'/'architecture-map.png'),full_page=True)
    page.locator('.nav[data-view="causal"]').click()
    page.screenshot(path=str(ROOT/'previews'/'causal-design.png'),full_page=True)
    page.locator('.nav[data-view="spatial"]').click()
    page.locator('#spatialReset').click()
    page.screenshot(path=str(ROOT/'previews'/'spatial-design.png'),full_page=True)
    assert not errors,errors
    browser.close()
(ROOT/'tests'/'acceptance-results.json').write_text(json.dumps({'url':URL,'results':results,'total':len(results),'passed':sum(r['pass'] for r in results),'errors':errors},ensure_ascii=False,indent=2),encoding='utf8')
if any(not r['pass'] for r in results):raise SystemExit('Tests failed')
