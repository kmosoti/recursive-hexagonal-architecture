from pathlib import Path
import collections,hashlib,json,sys,tomllib,unicodedata
p=Path(sys.argv[1]);kind=sys.argv[2];r=tomllib.loads((p/'registration.toml').read_text())
h=lambda b:hashlib.sha256(b).hexdigest()
sums=(p/'SHA256SUMS').read_bytes();assert h(sums)==r['content_sha256']
files={}
for line in sums.decode().splitlines():
 digest,name=line.split('  ',1)
 assert name not in files and not Path(name).is_absolute() and '..' not in Path(name).parts
 file=p/name;assert file.is_file() and not file.is_symlink();assert h(file.read_bytes())==digest,name
 files[name]=digest
actual={str(x.relative_to(p)) for x in p.rglob('*') if x.is_file()};assert actual==set(files)|{'registration.toml','SHA256SUMS'},actual-set(files)
assert len(files)==r.get('payload_count',r.get('counts',{}).get('payloads'))
cs=json.loads((p/'CASES.json').read_text());assert isinstance(cs,list)
assert len(cs)==r.get('case_count',r.get('counts',{}).get('cases')) and len({c['id'] for c in cs})==len(cs)
counts=dict(collections.Counter(c['kind'] for c in cs));assert counts==r.get('kind_counts',r.get('case_counts',r.get('kinds')))
if 'source_paths' in r:
 for name,src in r['source_paths'].items():
  rel=Path(src).relative_to(r['eventual_package']) if 'eventual_package' in r else Path(src);assert h((p/rel).read_bytes())==r['source_digests'].get(name+'_sha256',r['source_digests'].get(name))
  assert (p/rel).read_bytes()==Path(r['original_paths'][name]).read_bytes()
for source in r.get('sources',[]):
 path=source.get('path',source.get('source_path'))
 rel=Path(path).relative_to('xtask/tests/corpus/transclusion/'+{'region_graph':'regions','section':'sections','uri':'uris','site':'sites'}[kind])
 assert h((p/rel).read_bytes())==source.get('sha256',source.get('source_sha256'))
 assert (p/rel).read_bytes()==Path(source['original_path']).read_bytes()
if 'source_snapshots' in r:
 for name in ['contract','bdr','prompt']:
  rel=Path(r['source_snapshots'][name+'_path']).relative_to(r['eventual_path'])
  assert h((p/rel).read_bytes())==r['source_snapshots'][name+'_sha256']
  assert (p/rel).read_bytes()==Path(r['original_paths'][name]).read_bytes()
if kind=='uri':
 for c in cs:
  assert set(c)=={'id','kind','host','origin','destination','reference_kind','anchors','expected'}
  assert c['kind']=='uri' and c['reference_kind'] in ['link','image']
  assert all(isinstance(c[k],str) for k in ['host','origin','destination','expected'])
  assert all(unicodedata.normalize('NFC',c[k])==c[k] for k in ['host','origin'])
  assert isinstance(c['anchors'],dict) and all(isinstance(k,str) and isinstance(v,str) for k,v in c['anchors'].items())
print(json.dumps({'package':str(p),'cases':len(cs),'kind_counts':counts,'payload_count':len(files),'content_sha256':h(sums),'cases_sha256':h((p/'CASES.json').read_bytes())},sort_keys=True))
