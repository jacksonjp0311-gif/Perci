#!/usr/bin/env python3
from __future__ import annotations
import argparse, json, mmap, random, struct, time
from array import array
from pathlib import Path
import numpy as np
import build_weights as bw

FIXED = struct.Struct('<8sIIIIQQQ32s')
LABEL_ENTRY = 16 + 16 + bw.WORDS*8

def priors(text):
    t=' '+bw.normalize(text)+' '
    keys={
      'greeting':[' hello ',' hi ',' hey ','good morning','good evening'],
      'identity':['who are you','what exactly is perci','your limitations','your limits','what can you do'],
      'english':[' grammar ',' adjective ',' noun ',' verb ','rewrite','polish','english'],
      'logic':[' logically ','what follows','contradiction','assumption','infer ','reason step'],
      'math':[' calculate ',' compute ',' divided ',' multiply ',' plus ',' minus ',' equation ',' fraction ',' percent '],
      'geometry':[' triangle ',' circle ',' geometry ',' pythagorean ',' angle ',' circumference '],
      'memory':[' remember ',' recall ',' memory ',' store this ','what do you remember'],
      'code':[' rust ',' powershell ',' code ',' debug ',' parser ',' cli ',' repository '],
      'governance':[' permission ',' authority ',' authorized ',' durable ',' mutation ',' ledger ',' sandbox ',' origin alignment '],
      'planning':[' plan ',' milestones ',' roadmap ',' acceptance tests ',' dependencies ',' build first '],
      'explanation':[' explain ',' teach ',' simple terms ',' example ',' how does ',' why does '],
      'systems':[' lumen ',' cortex ',' bitwork ',' nemo ',' rhp ',' perci '],
      'science':[' momentum ',' energy ',' force ',' pressure ',' experiment ',' scientific ',' atom ',' cells '],
      'creativity':[' invent ',' brainstorm ',' story ',' creative ',' original ',' design a futuristic '],
      'comparison':[' compare ',' contrast ',' tradeoffs ',' versus ',' vs '],
    }
    scores={k:sum(24 for x in xs if x in t) for k,xs in keys.items()}
    scores['general']=24 if not any(scores.values()) else 0
    return scores

class Model:
    def __init__(self,path:Path):
        self.f=path.open('rb'); self.mm=mmap.mmap(self.f.fileno(),0,access=mmap.ACCESS_READ)
        magic,version,bits,words,nlabels,total,header,target,corpus=FIXED.unpack_from(self.mm,0)
        assert magic==bw.MAGIC and version==bw.VERSION and bits==bw.BITS and words==bw.WORDS
        self.header=header; self.labels=[]
        off=FIXED.size
        for _ in range(nlabels):
            name=self.mm[off:off+16].split(b'\0',1)[0].decode(); off+=16
            label_id,start,count,_=struct.unpack_from('<IIII',self.mm,off); off+=16
            pos=np.frombuffer(self.mm,dtype='<u8',count=bw.WORDS,offset=off); off+=bw.WORDS*8
            self.labels.append((name,start,count,pos.copy()))
    def classify(self,text:str,nearest=True):
        bits,pop=bw.encode(text); q=np.asarray(bits,dtype=np.uint64)
        boosts=priors(text)
        coarse=[]
        for i,(name,start,count,pos) in enumerate(self.labels):
            score=int(np.bitwise_count(np.bitwise_and(pos,q)).sum()) + boosts.get(name,0)
            coarse.append((score,i))
        coarse.sort(reverse=True)
        best=None
        for _,i in coarse[:3 if nearest else 1]:
            name,start,count,pos=self.labels[i]
            offset=self.header+start*bw.RECORD_SIZE
            rows=np.ndarray((count,bw.WORDS),dtype='<u8',buffer=self.mm,offset=offset+8,strides=(bw.RECORD_SIZE,8))
            overlaps=np.bitwise_count(np.bitwise_and(rows,q)).sum(axis=1,dtype=np.uint16)
            pops=np.ndarray((count,),dtype='<u2',buffer=self.mm,offset=offset+4,strides=(bw.RECORD_SIZE,))
            scores=overlaps.astype(np.int32)*2-pops.astype(np.int32)
            j=int(scores.argmax()); score=int(scores[j])
            variant=struct.unpack_from('<H',self.mm,offset+j*bw.RECORD_SIZE)[0]
            candidate=(score + coarse[[x[1] for x in coarse].index(i)][0]*2,i,variant,int(overlaps[j]))
            if best is None or candidate>best: best=candidate
        score,i,variant,overlap=best
        return self.labels[i][0],variant,score,overlap,coarse[:3]

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--model',type=Path,default=Path('models/perci-cognitive-v0.1.pwgt')); ns=ap.parse_args()
    m=Model(ns.model)
    tests=[
      ('greeting','hello friend, ready to begin?'),
      ('identity','what exactly is Perci and where are your limits'),
      ('english','could you polish my grammar and explain the adjective'),
      ('logic','all ravens are birds and this is a raven, what follows'),
      ('math','compute 812 divided by 7'),
      ('geometry','find the area of a triangle whose base is 14 and height is 9'),
      ('memory','please remember this architectural decision'),
      ('code','help debug this Rust command line parser'),
      ('governance','do we have authority to write this durable mutation'),
      ('planning','make milestones and acceptance tests for the project'),
      ('explanation','teach the concept in simple terms with an example'),
      ('systems','how should Lumen Cortex and Bitwork interconnect'),
      ('science','describe momentum and how to measure it experimentally'),
      ('creativity','invent an original cybernetic interface concept'),
      ('comparison','compare deterministic solvers against neural prediction'),
      ('general','give me your careful thoughts on this unusual idea'),
    ]
    ok=0; started=time.time()
    for expected,text in tests:
        got,variant,score,overlap,coarse=m.classify(text)
        passed=got==expected; ok+=passed
        print(f"{'PASS' if passed else 'FAIL'} expected={expected:11s} got={got:11s} variant={variant:2d} score={score:4d} overlap={overlap:3d} :: {text}")
    elapsed=time.time()-started
    print(json.dumps({'passed':ok,'total':len(tests),'accuracy':ok/len(tests),'seconds':elapsed,'queries_per_second':len(tests)/elapsed},indent=2))
    raise SystemExit(0 if ok>=14 else 1)
if __name__=='__main__': main()
