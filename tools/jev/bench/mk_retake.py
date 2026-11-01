import json, re
D="/private/tmp/editroom-jev"
T=json.load(open(f"{D}/truth-test2.json")); words=[w["text"] for w in T]
def lab(a,b):
    js=[T[i]["cut"] for i in range(a,b+1) if T[i]["cut"] is not None]
    if not js: return None
    f=sum(js)/len(js); return 1 if f>=0.6 else (0 if f<=0.4 else None)
def ctx(a,b,n=45):
    return " ".join(words[max(0,a-n):a])+" [["+" ".join(words[a:b+1])+"]] "+" ".join(words[b+1:b+1+n])
items=[]
# detector review items
for line in open(f"{D}/rt/work/review.md"):
    m=re.match(r"r(\d+) (\w+) w(\d+)-w(\d+):",line)
    if m: items.append({"key":f"r{m.group(1)}","src":"review","a":int(m.group(3)),"b":int(m.group(4))})
# agent editorial cuts (Opus judge)
for line in open(f"{D}/rt/cuts-agent.yaml"):
    if '"by":"agent"' in line or "by: agent" in line:
        m=re.search(r'"from":"w(\d+)".*?"to":"w(\d+)"',line) or re.search(r"from: w(\d+).*?to: w(\d+)",line)
        if m: items.append({"key":f"e{m.group(1)}","src":"editorial","a":int(m.group(1)),"b":int(m.group(2))})
# Opus decisions on review items: was the span cut by the agent run (cuts.yaml of the agent test incl. detector accepts)?
agent_cut=set()
import subprocess
Q=("In this transcript of a talking-head video, should the words inside [[ ]] be cut from the final edit because they are a retake, "
   "restart, false start, stumble or repetition that the speaker says again (better) right after? Answer no if cutting them would lose "
   "content that is not repeated.")
jobs=[];labels={}
for it in items:
    l=lab(it["a"],it["b"])
    if l is None: continue
    labels[it["key"]]={"label":l,"src":it["src"],"a":it["a"],"b":it["b"]}
    jobs.append({"key":it["key"],"text":ctx(it["a"],it["b"]),"questions":[{"id":"cut","kind":"yesno","instructions":Q}]})
open(f"{D}/rt/retake-jobs.jsonl","w").write("\n".join(json.dumps(j) for j in jobs)+"\n")
json.dump(labels,open(f"{D}/rt/retake-labels.json","w"),indent=1)
import collections; print(len(jobs), collections.Counter((v["src"],v["label"]) for v in labels.values()))
