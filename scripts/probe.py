import json, urllib.request, urllib.error
def dl(u, h=None):
    req = urllib.request.Request(u, headers={"User-Agent": "OrdisNexus-probe", "Accept": "application/json", "Language": "en", "Platform": "pc", **(h or {})})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()
def show(label, obj, n=900):
    print("=====", label); print(json.dumps(obj, ensure_ascii=False)[:n])
d = json.loads(dl("https://raw.githubusercontent.com/WFCD/warframe-drop-data/gh-pages/data/all.slim.json"))
show("drop sample", d[:3], 600); print("drop count", len(d))
show("drop ferrite", [x for x in d if x.get("item") == "Ferrite"][:4], 700)
show("drop wisp", [x for x in d if "Wisp Prime" in x.get("item","")][:3], 600)
items = json.loads(dl("https://api.warframe.market/v2/items"))["data"]
print("market items", len(items))
w = [i for i in items if "wisp_prime" in i["slug"]]
show("wisp market items", [{"slug": i["slug"], "gameRef": i.get("gameRef")} for i in w], 900)
slug = next((i["slug"] for i in w if "systems" in i["slug"]), w[0]["slug"])
for u in [f"https://api.warframe.market/v2/orders/item/{slug}/top", f"https://api.warframe.market/v2/orders/item/{slug}", f"https://api.warframe.market/v2/item/{slug}", f"https://api.warframe.market/v1/items/{slug}/statistics"]:
    try: show("OK "+u, json.loads(dl(u)), 1300)
    except urllib.error.HTTPError as e: print("FAIL", u, e.code, e.read()[:200])
    except Exception as e: print("FAIL", u, e)
