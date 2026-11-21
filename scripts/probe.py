import json, lzma, urllib.request, sys
def dl(u):
    req = urllib.request.Request(u, headers={"User-Agent": "OrdisNexus-probe"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()
def show(label, obj, n=900):
    print("=====", label); print(json.dumps(obj, ensure_ascii=False)[:n])
dec = lzma.LZMADecompressor(format=lzma.FORMAT_ALONE)
raw = dl("https://content.warframe.com/PublicExport/index_en.txt.lzma")
try: text = dec.decompress(raw).decode()
except lzma.LZMAError: text = b"".join(dec.decompress(raw[i:i+1]) for i in range(len(raw))).decode()
names = [l.strip() for l in text.splitlines() if l.strip()]
print("INDEX", [n.split("!")[0] for n in names])
def get(base):
    for n in names:
        if n.startswith(base + "_en.json") or n.startswith(base + ".json"):
            return json.loads(dl("https://content.warframe.com/PublicExport/Manifest/" + n))
for base in ["ExportRecipes","ExportRelicArcane","ExportResources","ExportWarframes","ExportWeapons","ExportUpgrades","ExportRegions","ExportNightwave","ExportFusionBundles","ExportSortieRewards","ExportCustoms"]:
    d = get(base)
    if d is None: print("MISSING", base); continue
    for k, v in d.items():
        show(f"{base}.{k} (len {len(v) if hasattr(v,'__len__') else '-'})", v[:2] if isinstance(v, list) else v)
# a prime recipe and a relic
rec = get("ExportRecipes")["ExportRecipes"]
for r in rec:
    if "WispPrime" in r.get("resultType","") or "WispPrime" in r.get("uniqueName",""):
        show("wisp recipe", r, 1500)
rel = get("ExportRelicArcane")["ExportRelicArcane"]
for r in rel:
    if r.get("relicRewards"):
        show("a relic", r, 1500); break
w = get("ExportWarframes")["ExportWarframes"]
show("frame", [x for x in w if "Wisp" in x.get("name","")][:1], 1500)
try:
    dt = json.loads(dl("https://www.warframe.com/droptables/data/droptables.json"))
    print("DROPTABLE KEYS", {k: type(v).__name__ for k, v in dt.items()})
    for k, v in dt.items():
        s = json.dumps(v, ensure_ascii=False)[:500]
        print("--", k, s)
except Exception as e: print("droptables fail", e)
try:
    ws = json.loads(dl("https://api.warframestat.us/pc"))
    print("WS KEYS", list(ws.keys()))
    for k in ["fissures","invasions","sortie","cetusCycle","vallisCycle","cambionCycle","arbitration","voidTrader","nightwave"]:
        v = ws.get(k); show("ws."+k, v[:1] if isinstance(v, list) else v, 700)
except Exception as e: print("ws fail", e)
try:
    st = json.loads(dl("https://api.warframe.market/v2/items"))
    show("market stats", st, 600)
except Exception as e: print("market fail", e)
