import json, lzma, urllib.request
def dl(u):
    req = urllib.request.Request(u, headers={"User-Agent": "OrdisNexus-probe", "Accept": "application/json", "Language": "en"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()
def show(label, obj, n=900):
    print("=====", label); print(json.dumps(obj, ensure_ascii=False)[:n])
dec = lzma.LZMADecompressor(format=lzma.FORMAT_ALONE)
raw = dl("https://content.warframe.com/PublicExport/index_en.txt.lzma")
try: text = dec.decompress(raw).decode()
except lzma.LZMAError: text = b"".join(dec.decompress(raw[i:i+1]) for i in range(len(raw))).decode()
names = [l.strip() for l in text.splitlines() if l.strip()]
def get(base):
    for n in names:
        if n.startswith(base + "_en.json") or n.startswith(base + ".json"):
            return json.loads(dl("https://content.warframe.com/PublicExport/Manifest/" + n))
rel = get("ExportRelicArcane")["ExportRelicArcane"]
rs = [r for r in rel if r.get("relicRewards")]
print("relics with rewards:", len(rs))
for r in rs[:3] + [x for x in rs if "Lith A1" in x.get("name","") or "Axi A1" in x.get("name","")][:6]:
    show("relic", {k: (v if k != "relicRewards" else v[:2]) for k, v in r.items()}, 700)
print("relic name samples", [r["name"] for r in rs[:12]])
print("relic unique samples", [r["uniqueName"] for r in rs[100:106]])
wp = [x for x in get("ExportWeapons")["ExportWeapons"] if x["name"] in ("Braton Prime", "Soma Prime", "Nami Skyla Prime")]
show("weapon samples", [{k: x.get(k) for k in ("name","uniqueName","productCategory","masteryReq")} for x in wp], 800)
for u in ["https://raw.githubusercontent.com/WFCD/warframe-drop-data/gh-pages/data/all.slim.json"]:
    try:
        d = json.loads(dl(u)); print("DROPDATA keys", list(d.keys()) if isinstance(d, dict) else type(d))
        for k, v in (d.items() if isinstance(d, dict) else []):
            show("drop."+k, v[:2] if isinstance(v, list) else v, 500)
    except Exception as e: print("drop fail", e)
for u in ["https://api.warframe.market/v2/orders/item/wisp_prime_systems/top", "https://api.warframe.market/v2/orders/item/wisp_prime_systems", "https://api.warframe.market/v1/items/wisp_prime_systems/orders", "https://api.warframe.market/v2/item/wisp_prime_systems"]:
    try: show("market "+u, json.loads(dl(u)), 900)
    except Exception as e: print("market fail", u, e)
