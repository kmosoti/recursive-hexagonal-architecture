from pathlib import Path
import collections,hashlib,json,shutil,subprocess,tomllib
source=Path('target/m2/module-test-boundary').resolve();copy=Path('target/m2/verify-module-test-boundary').resolve();assert not copy.exists()
h=lambda b:hashlib.sha256(b).hexdigest()
def audit(p,live=False):
 reg=tomllib.loads((p/'registration.toml').read_text());sums=(p/'SHA256SUMS').read_bytes();assert h(sums)==reg['content_sha256'];inventory={}
 for line in sums.decode().splitlines():
  digest,name=line.split('  ',1);assert name not in inventory and not Path(name).is_absolute() and '..' not in Path(name).parts
  f=p/name;assert f.is_file() and not f.is_symlink();assert h(f.read_bytes())==digest,name;inventory[name]=digest
 files={str(f.relative_to(p)) for f in p.rglob('*') if f.is_file()};assert files==set(inventory)|{'registration.toml','SHA256SUMS'}
 assert len(inventory)==reg['payloads']==7 and len(files)==reg['files']==9
 cases=json.loads((p/'CASES.json').read_text());assert len(cases)==reg['actual_case_count']==12;assert len({c['id'] for c in cases})==12;assert dict(collections.Counter(c['kind'] for c in cases))==reg['kind_counts']
 for s in reg['source_snapshots']:
  b=(p/s['relative_path']).read_bytes();assert h(b)==s['sha256']
  if live:assert b==Path(s['original_documentary_path']).read_bytes()
 for c in cases:
  for path,digest in c['expected']['source_files'].items():assert h(c['files'][path].encode())==digest
  if c['mutation']:
   m=c['mutation'];assert h(m['text'].encode())==m['expected_sha256'];assert m['expected_sha256']!=h(c['files'][m['path']].encode())
 print(json.dumps({'package':str(p),'cases':len(cases),'outcomes':dict(collections.Counter(c['expected']['outcome'] for c in cases)),'payloads':len(inventory),'files':len(files),'content_sha256':h(sums),'cases_sha256':h((p/'CASES.json').read_bytes())}),flush=True)
audit(source,True);shutil.copytree(source,copy)
try:
 audit(copy)
 for cmd in [['python3','-B',str(copy/'reference.py')],['python3','-B',str(copy/'reference.py'),'--reproduce','reproduced']]:
  p=subprocess.run(cmd,cwd=copy,text=True,capture_output=True);print('COMMAND',repr(cmd),'EXIT',p.returncode);print(p.stdout,end='');print(p.stderr,end='');assert p.returncode==0
 child=copy/'reproduced';original={str(p.relative_to(source)):p.read_bytes() for p in source.rglob('*') if p.is_file()};reproduced={str(p.relative_to(child)):p.read_bytes() for p in child.rglob('*') if p.is_file()};assert original==reproduced;print('BYTE_IDENTICAL_FILES',len(original));audit(child)
 f=child/'CASES.json';f.write_bytes(f.read_bytes()+b'\n');r=subprocess.run(['python3','-B',str(child/'reference.py')],cwd=child,text=True,capture_output=True);print('CORRUPTED_COPY_EXIT',r.returncode);print(r.stdout,end='');print(r.stderr,end='');assert r.returncode!=0 and ('CASES.json' in r.stderr or 'CASES.json' in r.stdout)
 print('VALID_COPY_ACCEPTED_AND_CASE_TAMPERING_REFUSED')
finally:shutil.rmtree(copy)
