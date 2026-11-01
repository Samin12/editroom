import glob, json, os, subprocess, random
P="/Users/saminyasar/Borumi Projects/tips Agent/proof"
C=os.path.expanduser("~/.editroom/bin/cutroom")
imgs=[f for f in sorted(glob.glob(f"{P}/0*/*.png")) if not f.endswith("-page.png") and "full" not in f and "highlight" not in f]
random.seed(7); random.shuffle(imgs)
jobs=[]; gt={}
for f in imgs:
    if len(jobs)>=25: break
    tsv=subprocess.run(["tesseract",f,"-","tsv"],capture_output=True,text=True).stdout.splitlines()[1:]
    lines={}
    for row in tsv:
        c=row.split("\t")
        if len(c)<12 or not c[11].strip() or float(c[10])<88: continue
        lines.setdefault((c[2],c[3],c[4]),[]).append(c[11])
    cands=[w for w in lines.values() if len(w)>=4]
    if not cands: continue
    ws=cands[len(cands)//2]; phrase=" ".join(ws[1:4])
    if len(phrase)<10 or not any(ch.isalpha() for ch in phrase): continue
    r=subprocess.run([C,"boxes",f,"--phrase",phrase,"--json"],capture_output=True,text=True)
    try: m=json.loads(r.stdout)["matches"]
    except Exception: continue
    if not m or not m[0].get("exact"): continue
    k=os.path.basename(f); gt[k]={"phrase":phrase,"norm":m[0]["norm"],"lines":len(m[0].get("lines",[]))}
    jobs.append({"key":k,"image":f,"questions":[{"id":"box","kind":"box","instructions":f"the words \"{phrase}\" in the image"}]})
open(f"/private/tmp/editroom-jev/box/box-jobs.jsonl","w").write("\n".join(json.dumps(j) for j in jobs)+"\n")
json.dump(gt,open("/private/tmp/editroom-jev/box/box-gt.json","w"),indent=1)
print(len(jobs),"phrases")
