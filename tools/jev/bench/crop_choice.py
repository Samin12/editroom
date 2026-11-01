import json, random, sys, base64, uuid, time, concurrent.futures as cf
sys.path.insert(0, "/Users/saminyasar/Projects/editroom-wt/s-jev/tools/jev")
import decide
items=json.load(open("crop-jobs.json"))
random.seed(3)
def jev_multi(it):
    c=list(it["cands"]); random.shuffle(c)
    names="ABCD"[:len(c)]
    state={"claim":it["claim"]}; assets={}
    for n,pth in zip(names,c):
        raw,fmt=decide.prepared_image(pth,"jev"); ref=f"img-{uuid.uuid4().hex[:10]}"
        assets[ref]={"type":"image","format":fmt,"data_base64":base64.b64encode(raw).decode()}; state[f"screenshot_{n}"]=ref
    q={"pick":{"type":"choice","instructions":"Which screenshot is the best on-screen proof of the claim: it shows the exact words or number he says, readable at a glance, without clutter?",
               "criteria":{n:f"screenshot_{n}" for n in names}}}
    body={"model":decide.JEV_MODEL,"state":state,"assets":assets,"questions":q}
    resp,ms=decide.post(decide.JEV_URL,body,{"Authorization":f"Bearer {decide.jev_key()}","TypeSafe-Danger-Zone-Images":"true"})
    ch=resp["answers"]["pick"]["choice"]; return c[names.index(ch)]==it["best"], ms, resp["usage"]["input_tokens"]
def per_cand(it,prov):
    q={"id":"fit","kind":"score","instructions":"How well does the screenshot in the image work as on-screen proof of what the creator says: the exact words or number readable at a glance, without clutter?",
       "levels":["unusable","weak","ok","good","ideal"]}
    sc=[]
    for pth in it["cands"]:
        r=decide.ask({"key":it["key"],"image":pth,"text":f"What the creator says: \"{it['claim']}\"","questions":[q]},prov)[0]
        sc.append(r["answer"] if r["answer"] is not None else -1)
    return it["cands"][sc.index(max(sc))]==it["best"]
with cf.ThreadPoolExecutor(8) as ex:
    m=list(ex.map(lambda it: (lambda: jev_multi(it))() if True else None, items))
acc_multi=sum(x[0] for x in m)/len(m)
base=sum(1/len(it["cands"]) for it in items)/len(items)
with cf.ThreadPoolExecutor(8) as ex:
    pj=list(ex.map(lambda it: per_cand(it,"jev"), items))
    ps=list(ex.map(lambda it: per_cand(it,"sage"), items))
import statistics as st
print(f"n={len(items)} chance {base:.2f} | jev one-request choice {acc_multi:.2f} (median {st.median(x[1] for x in m):.0f} ms, ~{st.mean(x[2] for x in m):.0f} tok) | jev per-crop score {sum(pj)/len(pj):.2f} | sage per-crop score {sum(ps)/len(ps):.2f}")
