from pathlib import Path
import json, tempfile,threading,http.server,functools,urllib.request,hashlib
from playwright.sync_api import sync_playwright
root=Path(__file__).resolve().parents[1]
html=(root/'index.html').read_text(encoding='utf8')
assert '<script>window.ATLAS_MODEL=' in html
assert not any(x in html for x in ['<script src=','<link href="https:','@import url('])
assert len(html)>60000
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(http.server.SimpleHTTPRequestHandler,directory=str(root)))
threading.Thread(target=server.serve_forever,daemon=True).start()
with urllib.request.urlopen(f'http://127.0.0.1:{server.server_port}/index.html') as f: b=f.read(); assert b==html.encode()
server.shutdown()
with sync_playwright() as pw:
 browser=pw.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox','--enable-unsafe-swiftshader'])
 mobile=browser.new_page(viewport={'width':390,'height':844})
 errors=[];mobile.on('pageerror',lambda e:errors.append(str(e)))
 mobile.set_content(html,wait_until='load');mobile.wait_for_timeout(350)
 mobile.locator('[data-dim="1"]').click()
 assert mobile.locator('#promptText').is_visible()
 mobile.locator('[data-dim="5"]').click()
 assert mobile.locator('#kindPick').is_visible()
 assert mobile.locator('#space3d').is_visible()
 assert not errors,errors
 out={'standaloneHtmlBytes':len(b),'externalRuntimeAssets':0,'staticHttpServedBytes':len(b),'mobileWidth':390,'mobileInteractive':True,'errors':errors,'sha256':hashlib.sha256(b).hexdigest()}
 mobile.screenshot(path=str(root/'tests/mobile_screenshot.png'))
 browser.close()
(root/'tests/release_results.json').write_text(json.dumps(out,indent=2,ensure_ascii=False))
print(json.dumps(out,indent=2))
