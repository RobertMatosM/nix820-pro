#!/usr/bin/env python3
"""Generate distribution notices from the resolved Linux dependencies and bundled fonts."""
import hashlib
import json
import pathlib
import subprocess

ROOT=pathlib.Path(__file__).resolve().parent.parent
host=next(line.split(': ',1)[1] for line in subprocess.check_output(['rustc','-vV'],text=True).splitlines() if line.startswith('host:'))
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--filter-platform',host,'--format-version','1'],cwd=ROOT,text=True))
packages={p['id']:p for p in metadata['packages']}
nodes={n['id']:n for n in metadata['resolve']['nodes']}
pending=[metadata['resolve']['root']]
seen=set()
while pending:
    ident=pending.pop()
    if ident in seen: continue
    seen.add(ident)
    pending.extend(dep['pkg'] for dep in nodes[ident]['deps'])
overrides={name:'egui-MIT.txt' for name in ['ecolor','eframe','egui','egui-wgpu','egui-winit','egui_glow','emath','epaint']}
overrides.update(accesskit='accesskit-MIT.txt',profiling='profiling-MIT.txt',gl_generator='Apache-2.0.txt',khronos_api='Apache-2.0.txt')
texts={}
rows=[]
def notice(text):
    text=text.strip()
    digest=hashlib.sha256(text.encode()).hexdigest()[:12]
    texts[digest]=text
    return digest
for ident in sorted(seen,key=lambda ident:(packages[ident]['name'],packages[ident]['version'])):
    p=packages[ident]
    if p['name']=='keyboard-studio': continue
    directory=pathlib.Path(p['manifest_path']).parent
    files=sorted({f for pattern in ['LICENSE*','LICENCE*','COPYING*','NOTICE*','license*','licence*'] for f in directory.glob(pattern) if f.is_file()})
    if p.get('license_file') and (directory/p['license_file']).is_file(): files.append(directory/p['license_file'])
    if not files:
        if p['name'] not in overrides: raise SystemExit(f"Missing license text: {p['name']} {p['version']}")
        files=[ROOT/'tools/license-overrides'/overrides[p['name']]]
    references=[notice(file.read_text(errors='replace')) for file in files]
    rows.append(f"| {p['name']} {p['version']} | {p.get('license') or 'See text'} | "+', '.join(f'[notice](#notice-{ref})' for ref in references)+' |')
output=['# Third-party notices','',f'Generated for `{host}`. Original Keyboard Studio code is 0BSD. Dependencies and bundled fonts retain the licenses below. The resolved graph can include build or optional packages in addition to linked code. System libraries retain their own licenses.','', '| Package | Declared license | Text |','| --- | --- | --- |']+rows
output+=['','## Bundled fonts','']
for name in ['Interface-LICENSE.txt','Symbols-LICENSE.txt']:
    ref=notice((ROOT/'assets/fonts'/name).read_text())
    output.append(f'- {name}: [notice](#notice-{ref})')
for ident,text in sorted(texts.items()):
    output += ['',f'<a id="notice-{ident}"></a>','',f'## Notice {ident}','','```text',text,'```']
(ROOT/'THIRD_PARTY_NOTICES.md').write_text('\n'.join(output)+'\n')
print(f'{len(rows)} dependency packages; {len(texts)} deduplicated notice texts')
