import json, glob, os
D="/private/tmp/editroom-jev"
W=[(538.93,649.21),(786.43,809.76)]  # union of blur windows (output s)
Q=("Is any private information readable in this screenshot of a video frame: an email address (even partly cut off), "
   "an API key or token, an account, workspace or card ID inside a URL, a bank or financial account name, a phone number, "
   "or a private message from another person? Ignore the presenter's face and the 'TIP n OF 10' badge.")
out=open(f"{D}/privacy-jobs.jsonl","w")
labels={}
for f in sorted(glob.glob(f"{D}/frames1fps/*.jpg")):
    i=int(os.path.basename(f)[1:5]); t=i-1+0.5
    inside=any(a+0.5<=t<=b-0.5 for a,b in W)
    near=any(a-2<=t<=b+2 for a,b in W)
    lab = 1 if inside else (None if near else 0)
    key=f"t{t:.1f}"
    labels[key]=lab
    out.write(json.dumps({"key":key,"image":f,"questions":[
        {"id":"private","kind":"yesno","instructions":Q,
         "criteria":{"true":"Some private detail is readable on screen.","false":"Nothing private is readable."}},
        {"id":"where","kind":"box","instructions":"the private detail (an email address, an ID in a URL, or a bank name)"}]})+"\n")
json.dump(labels,open(f"{D}/privacy-labels.json","w"))
print(sum(1 for v in labels.values() if v==1),"pos",sum(1 for v in labels.values() if v==0),"neg",sum(1 for v in labels.values() if v is None),"edge")
