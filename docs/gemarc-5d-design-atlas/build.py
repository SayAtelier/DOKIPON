#!/usr/bin/env python3
"""Create a dependency-free portable entry for file:// and static HTTP."""
from pathlib import Path
import json, html
ROOT=Path(__file__).parent
model=json.loads((ROOT/'models/architecture.json').read_text())
cases=json.loads((ROOT/'models/cases.json').read_text())
s=(ROOT/'source/template.html').read_text()
def encode(o): return json.dumps(o,ensure_ascii=False,separators=(',',':')).replace('<','\\u003c').replace('>','\\u003e').replace('&','\\u0026')
s=s.replace('__MODEL__',encode(model)).replace('__CASES__',encode(cases))
s=s.replace('__ENGINE__',(ROOT/'source/engine.js').read_text()).replace('__VIEWER__',(ROOT/'source/viewer.js').read_text()).replace('__LABELS__',(ROOT/'source/labels.js').read_text())
s=s.replace('__STYLE__',(ROOT/'source/style.css').read_text())
s=s.replace('__SHA_SHORT__',model['revision'][:12])
s=s.replace('__CASE_OPTIONS__',''.join(f'<option value="{html.escape(c["id"])}">{html.escape(c["label"])}</option>' for c in cases))
s=s.replace('__REQ_OPTIONS__',''.join(f'<option value="{html.escape(r["id"])}">{html.escape(r["id"]+" / "+r["name"])}</option>' for r in model['requirements']))
(ROOT/'index.html').write_text(s,encoding='utf8');(ROOT/'dist').mkdir(exist_ok=True);(ROOT/'dist/index.html').write_text(s,encoding='utf8');print(f'Built standalone HTML: {len(s):,} chars')
