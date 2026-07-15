#!/usr/bin/env python3
"""Chunked builder for execution environments with short command limits."""
from __future__ import annotations
import argparse, hashlib, json, os, random, struct
from array import array
from pathlib import Path
import build_weights as bw

CHUNK_DIR = Path('training/weight-chunks')

def counts():
    total = (bw.TARGET_SIZE-bw.HEADER_SIZE)//bw.RECORD_SIZE
    base, rem = divmod(total, len(bw.LABELS))
    cs=[base+(i<rem) for i in range(len(bw.LABELS))]
    offsets=[]; x=0
    for c in cs: offsets.append(x); x+=c
    return total, cs, offsets, bw.TARGET_SIZE-bw.HEADER_SIZE-total*bw.RECORD_SIZE

def build_label(label_id:int):
    CHUNK_DIR.mkdir(parents=True, exist_ok=True)
    total, cs, _, _ = counts(); count=cs[label_id]; label=bw.LABELS[label_id]
    out=CHUNK_DIR/f'{label_id:02d}-{label}.bin'
    freq=array('I',[0])*bw.BITS
    digest=hashlib.sha256()
    rng=random.Random(bw.SEED ^ (label_id * 0x9E3779B97F4A7C15))
    with out.open('wb') as fh:
        for i in range(count):
            prompt, variant, quality=bw.prompt_for(label,rng,i)
            bits,pop=bw.encode(prompt)
            digest.update(label.encode('ascii')); digest.update(b'\0'); digest.update(prompt.encode()); digest.update(b'\n')
            for wi,w in enumerate(bits):
                v=int(w)
                while v:
                    low=v & -v; bit=low.bit_length()-1
                    freq[(wi<<6)+bit]+=1; v^=low
            fh.write(struct.pack('<HHHH',variant,quality,pop,0)); bits.tofile(fh)
    with (out.with_suffix('.freq')).open('wb') as fh: freq.tofile(fh)
    (out.with_suffix('.json')).write_text(json.dumps({'id':label_id,'label':label,'count':count,'bytes':out.stat().st_size,'corpus_sha256':digest.hexdigest()},indent=2)+'\n')
    print(json.dumps({'built':label,'count':count,'bytes':out.stat().st_size}))

def assemble(output:Path):
    total, cs, offsets, pad=counts()
    freqs=[]; corpus=hashlib.sha256()
    for i,label in enumerate(bw.LABELS):
        base=CHUNK_DIR/f'{i:02d}-{label}'
        binp=base.with_suffix('.bin'); fp=base.with_suffix('.freq'); jp=base.with_suffix('.json')
        if not binp.exists() or not fp.exists(): raise SystemExit(f'missing chunk {label}')
        a=array('I');
        with fp.open('rb') as fh: a.fromfile(fh,bw.BITS)
        freqs.append(a)
        corpus.update(bytes.fromhex(json.loads(jp.read_text())['corpus_sha256']))
    allf=array('Q',[0])*bw.BITS
    for f in freqs:
        for j,v in enumerate(f): allf[j]+=v
    pos=[]; neg=[]
    for f in freqs:
        others=array('Q',(int(allf[j])-int(f[j]) for j in range(bw.BITS)))
        p,n=bw.top_mask(f,others); pos.append(p); neg.append(n)
    output.parent.mkdir(parents=True,exist_ok=True)
    with output.open('wb+') as out:
        out.write(b'\0'*bw.HEADER_SIZE)
        for i,label in enumerate(bw.LABELS):
            p=CHUNK_DIR/f'{i:02d}-{label}.bin'
            with p.open('rb') as src:
                while True:
                    block=src.read(4*1024*1024)
                    if not block: break
                    out.write(block)
        out.write(b'\0'*pad)
        bw.write_header(out,total,offsets,cs,pos,neg,corpus.digest())
        out.flush(); os.fsync(out.fileno())
    h=hashlib.sha256()
    with output.open('rb') as fh:
        for b in iter(lambda:fh.read(4*1024*1024),b''): h.update(b)
    manifest={'name':'Perci Cognitive Weights','version':bw.VERSION,'format':'PERCIW01','architecture':'4096-bit sparse associative Bitwork network','size_bytes':output.stat().st_size,'size_mib':output.stat().st_size/(1024*1024),'prototype_count':total,'bits_per_activation':bw.BITS,'words_per_activation':bw.WORDS,'labels':bw.LABELS,'record_size':bw.RECORD_SIZE,'sha256':h.hexdigest(),'corpus_sha256':corpus.hexdigest(),'seed':bw.SEED,'limitations':['Not a transformer or general-purpose pretrained language model.','Open-ended language is template and retrieval based.','Exact arithmetic and geometry are delegated to deterministic tools.','Knowledge is bounded by the generated curriculum and local memory.']}
    output.with_suffix(output.suffix+'.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps(manifest,indent=2))

def main():
    ap=argparse.ArgumentParser(); sub=ap.add_subparsers(dest='cmd',required=True)
    a=sub.add_parser('label'); a.add_argument('ids',nargs='+',type=int)
    b=sub.add_parser('assemble'); b.add_argument('--output',type=Path,default=Path('models/perci-cognitive-v0.1.pwgt'))
    ns=ap.parse_args()
    if ns.cmd=='label':
        for i in ns.ids: build_label(i)
    else: assemble(ns.output)
if __name__=='__main__': main()
