from pathlib import Path
p=Path(__file__).parent
html=(p/'index.template.html').read_text(encoding='utf8')
css=(p/'styles.css').read_text(encoding='utf8')
html=html.replace('<link rel="stylesheet" href="./styles.css">','<style>\n'+css+'\n</style>')
for source in ['data.js','scene3d.js','app.js','prompt1d.js']:
    code=(p/source).read_text(encoding='utf8').replace('</script','<\\/script')
    html=html.replace('<script src="./'+source+'"></script>','<script>\n'+code+'\n</script>')
(p/'index.html').write_text(html,encoding='utf8')
print('single-file index size',len(html))
