import glob, os, json
from PIL import Image, ImageChops
D="/private/tmp/editroom-jev/qa"
canvas=Image.open(f"{D}/canvas.png").convert("RGB")
base=sorted(glob.glob(f"{D}/good/*-55.jpg"))
lab={}
for f in sorted(glob.glob(f"{D}/good/*.jpg")):
    lab[f]={"clipped":0,"overlap":0,"empty":0,"camera":0,"tiny":0}
for f in base:
    im=Image.open(f).convert("RGB"); W,H=im.size; cv=canvas.resize((W,H))
    name=os.path.basename(f)[:-7]
    a=cv.copy(); a.paste(im.crop((int(.33*W),0,W,H)),(0,0)); a.save(f"{D}/bad/{name}-clipped.jpg",quality=90)
    sh=Image.new("RGB",(W,H)); sh.paste(im,(int(.05*W),int(.045*H))); b=ImageChops.lighter(im,sh); b.save(f"{D}/bad/{name}-overlap.jpg",quality=90)
    cv.save(f"{D}/bad/{name}-empty.jpg",quality=90)
    c=im.copy(); c.paste(im.crop((int(.18*W),int(.3*H),int(.5*W),int(.62*H))).resize((int(.2*W),int(.2*H))),(int(.79*W),int(.35*H))); c.save(f"{D}/bad/{name}-camera.jpg",quality=90)
    t=cv.copy(); t.paste(im.crop((0,0,int(.78*W),H)).resize((int(.78*W*.28),int(H*.28))),(int(.2*W),int(.36*H))); t.save(f"{D}/bad/{name}-tiny.jpg",quality=90)
    for k in ["clipped","overlap","empty","camera","tiny"]:
        l={"clipped":0,"overlap":0,"empty":0,"camera":0,"tiny":0}; l[k]=1
        lab[f"{D}/bad/{name}-{k}.jpg"]=l
Q={
 "clipped":"Is any text, card or picture cut off at the left or right edge of the image, so part of it is missing?",
 "overlap":"Do two texts or elements overlap or ghost over each other so that some text becomes hard to read?",
 "empty":"Is the image essentially empty: only a dark dotted background, with no text, card, picture or diagram?",
 "camera":"Is there any text, card or graphic in the right-hand fifth of the image (the area right of 78% of the width), between 25% and 75% of the height?",
 "tiny":"Is the main text in this image too small to read comfortably on a phone screen?",
}
with open(f"{D}/qa-jobs.jsonl","w") as o:
    for f in lab:
        o.write(json.dumps({"key":f,"image":f,"questions":[{"id":k,"kind":"yesno","instructions":q} for k,q in Q.items()]})+"\n")
json.dump(lab,open(f"{D}/qa-labels.json","w"))
print(len(lab),"frames")
