#!/usr/bin/env python3
"""Verify Perci's bundled cognitive weight file using only the standard library."""
from __future__ import annotations
import argparse, hashlib, json, struct
from pathlib import Path

MAGIC=b'PERCIW01'
FIXED=struct.Struct('<8sIIIIQQQ32s')

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--model',type=Path,default=Path('models/perci-cognitive-v0.1.pwgt'))
    ns=ap.parse_args()
    manifest_path=ns.model.with_suffix(ns.model.suffix+'.json')
    manifest=json.loads(manifest_path.read_text(encoding='utf-8'))
    size=ns.model.stat().st_size
    h=hashlib.sha256()
    with ns.model.open('rb') as fh:
        header=fh.read(FIXED.size)
        values=FIXED.unpack(header)
        for chunk in iter(lambda:fh.read(4*1024*1024),b''): h.update(chunk)
    # The header was consumed before hashing; hash the whole file accurately.
    h=hashlib.sha256()
    with ns.model.open('rb') as fh:
        for chunk in iter(lambda:fh.read(4*1024*1024),b''): h.update(chunk)
    magic,version,bits,words,labels,records,header_size,target_size,corpus=values
    checks={
      'magic':magic==MAGIC,
      'version':version==manifest['version'],
      'bits':bits==manifest['bits_per_activation'],
      'words':words==manifest['words_per_activation'],
      'label_count':labels==len(manifest['labels']),
      'records':records==manifest['prototype_count'],
      'size':size==manifest['size_bytes']==target_size,
      'sha256':h.hexdigest()==manifest['sha256'],
    }
    print(json.dumps({'model':str(ns.model),'checks':checks,'sha256':h.hexdigest(),'size_bytes':size},indent=2))
    raise SystemExit(0 if all(checks.values()) else 1)
if __name__=='__main__': main()
