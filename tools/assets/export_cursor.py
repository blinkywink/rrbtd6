"""Export the game's hardware cursor textures (Assets.Cursor: HandCursor / HandCursorDown 64x64)
to local_extracted/ui. Usage: tools/.venv/Scripts/python.exe -I tools/assets/export_cursor.py <game install>
"""
import UnityPy, os, sys, glob
G=sys.argv[1]; AA=os.path.join(G,"BloonsTD6_Data","StreamingAssets","aa","StandaloneWindows64","Full")
paths=glob.glob(os.path.join(G,"BloonsTD6_Data","*.assets"))+glob.glob(os.path.join(G,"BloonsTD6_Data","level*"))+glob.glob(os.path.join(AA,"*scene*.bundle"))+glob.glob(os.path.join(AA,"asset_ref*.bundle"))
for p in paths:
    try: env=UnityPy.load(p)
    except Exception: continue
    for o in env.objects:
        if o.type.name!="MonoBehaviour": continue
        try: t=o.read_typetree()
        except Exception: continue
        if "softwareXLarge" in t:
            print(os.path.basename(p), [e.path for e in o.assets_file.externals][:3])
            ext=o.assets_file.externals[1].path.split("/")[-1]
            G2=glob.glob(os.path.join(G,"BloonsTD6_Data",ext))+glob.glob(os.path.join(G,"BloonsTD6_Data","**",ext),recursive=True)
            print("ext",ext,G2[:1])
            e2=env
            objs={}
            if e2:
                for f in e2.files.values():
                    if hasattr(f,"objects"): objs.update(f.objects)
                    for nm,sub in getattr(f,"files",{}).items():
                        if hasattr(sub,"objects") and nm.lower()==ext.lower(): objs.update(sub.objects)
            for k in ("hardware","softwareLarge","softwareXLarge","hardwareLarge","hardwareXLarge"):
                v=t.get(k); 
                if not v: continue
                out={}
                for f in ("textureUp","textureDown"):
                    pp=v[f]
                    try:
                        tex=objs[pp["m_PathID"]].read()
                        out[f]=(tex.m_Name, tex.m_Width, tex.m_Height) if tex else pp
                        if k=="hardware": tex.image.save(os.path.join("local_extracted","ui",tex.m_Name+".png"))
                    except Exception as e: out[f]=str(e)[:40]
                print(" ",k,out,v.get("hotspot"))
            sys.exit()
