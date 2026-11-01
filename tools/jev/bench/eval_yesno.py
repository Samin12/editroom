import json, sys, statistics as st
def load(p, qid):
    rows={}
    for l in open(p):
        r=json.loads(l)
        if r["id"]==qid: rows[r["key"]]=r
    return rows
def report(name, rows, labels, price, tok_override=None):
    tp=fp=fn=tn=nul=0; lat=[]; toks=[]
    scored=[]
    for k,lab in labels.items():
        if lab is None or k not in rows: continue
        r=rows[k]
        if r.get("error"): continue
        lat.append(r["latency_ms"] or 0); toks.append(tok_override or r.get("input_tokens") or 0)
        a=r["answer"]; p=r.get("p")
        scored.append((p if p is not None else 0.5, lab))
        if a is None: nul+=1; a="yes"   # route unsure to the agent = counts as flagged
        if a=="yes" and lab: tp+=1
        elif a=="yes": fp+=1
        elif lab: fn+=1
        else: tn+=1
    n=tp+fp+fn+tn
    prec=tp/(tp+fp) if tp+fp else 0; rec=tp/(tp+fn) if tp+fn else 0
    # best threshold that keeps recall 1.0
    pos=sorted(p for p,l in scored if l); thr=min(pos) if pos else None
    flagged_at_full_recall=sum(1 for p,l in scored if p>=thr) if thr is not None else None
    cost=st.mean(toks)*price/1e6*1000 if toks else 0
    print(f"{name}: n={n} TP={tp} FP={fp} FN={fn} TN={tn} unsure={nul} precision={prec:.2f} recall={rec:.2f} "
          f"| full-recall threshold p>={thr:.3g} flags {flagged_at_full_recall}/{n} "
          f"| latency median {st.median(lat):.0f} ms p95 {sorted(lat)[int(len(lat)*.95)]:.0f} | ~{st.mean(toks):.0f} tok/call ${cost:.3f}/1k calls")
if __name__=="__main__":
    lab=json.load(open(sys.argv[1])); qid=sys.argv[2]
    for spec in sys.argv[3:]:
        name,path,price,*tok=spec.split(",")
        report(name, load(path,qid), lab, float(price), int(tok[0]) if tok else None)
