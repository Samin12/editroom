import json, statistics as st, sys
lab=json.load(open("qa-labels.json"))
for p,price in (("jev",0.042),("sage",0.05)):
    R={}
    for l in open(f"qa-{p}.jsonl"):
        r=json.loads(l); R[(r["key"],r["id"])]=r
    print(f"== {p}")
    lat=[];tok=[]
    for q in ["clipped","overlap","empty","camera","tiny"]:
        tp=fp=fn=tn=nul=0; probs=[]
        for k,l in lab.items():
            r=R.get((k,q))
            if not r or r.get("error"): continue
            if not r.get("cached"): pass
            lat.append(r["latency_ms"]); tok.append(r.get("input_tokens") or 0)
            a=r["answer"]; y=l[q]
            probs.append((r.get("p") or 0, y))
            if a is None: nul+=1; a="yes"
            if a=="yes" and y: tp+=1
            elif a=="yes": fp+=1
            elif y: fn+=1
            else: tn+=1
        pos=[pp for pp,y in probs if y]; thr=min(pos)
        fl=sum(1 for pp,y in probs if pp>=thr)
        print(f"  {q:8s} TP={tp:2d} FN={fn:2d} FP={fp:3d} TN={tn:3d} unsure={nul:2d} recall={tp/(tp+fn):.2f} precision={tp/max(1,tp+fp):.2f} | full-recall flags {fl}/{len(probs)}")
    print(f"  latency median {st.median(lat):.0f} ms, ~{st.mean(tok):.0f} tok/question, ${st.mean(tok)*price/1e6*1000:.3f}/1k questions")
