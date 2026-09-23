from pathlib import Path
import hashlib, shutil, subprocess, sys
name,kind=sys.argv[1:]
root=Path.cwd(); source=root/'target/m2'/('transclusion-'+name)
copy=root/'target/m2'/('verify-transclusion-'+name)
assert not copy.exists(),copy
run=lambda args: subprocess.run(args,cwd=(copy if "--reproduce-to" in args else root),text=True,capture_output=True)
def ok(args):
 p=run(args); print('COMMAND', ' '.join(map(str,args)));print(p.stdout,end='');print(p.stderr,end='');print('EXIT',p.returncode);assert p.returncode==0
shutil.copytree(source,copy)
try:
 ok(['python3','-B','target/m2/audit-semantic-package.py',str(copy),kind])
 ok(['python3','-B',str(copy/'reference.py')])
 ok(['python3','-B',str(copy/'reference.py'),'--reproduce-to','reproduced'])
 child=copy/'reproduced'
 original={str(f.relative_to(source)):f.read_bytes() for f in source.rglob('*') if f.is_file()}
 produced={str(f.relative_to(child)):f.read_bytes() for f in child.rglob('*') if f.is_file()}
 assert original==produced,{'missing':sorted(set(original)-set(produced)),'extra':sorted(set(produced)-set(original)),'changed':[f for f in set(original)&set(produced) if original[f]!=produced[f]]}
 print('BYTE_IDENTICAL_FILES',len(original))
 ok(['python3','-B','target/m2/audit-semantic-package.py',str(child),kind])
 f=child/'CASES.json';f.write_bytes(f.read_bytes()+b'\n')
 p=run(['python3','-B','target/m2/audit-semantic-package.py',str(child),kind]);print('CORRUPTED_COPY_EXIT',p.returncode);print(p.stderr,end='');assert p.returncode!=0 and 'CASES.json' in p.stderr
 print('VALID_COPY_ACCEPTED_AND_CASE_CORRUPTION_REJECTED')
finally: shutil.rmtree(copy)
