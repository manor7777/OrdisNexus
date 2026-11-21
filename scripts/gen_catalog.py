#!/usr/bin/env python3
"""Build assets/catalog.json.gz from Warframe's official Public Export files.

  python scripts/gen_catalog.py --dir <folder with Export*.json files>
  python scripts/gen_catalog.py --fetch        # download the latest Public Export

Files are matched by name (any prefix/suffix), so `ExportRecipes.json`,
`ExportRecipes_en.json` and `abc123-ExportRecipes.json` all work. An optional
`warframe_lang_dict.json` ({lowercase unique name: {"name": ...}}) adds names.

Output: names + icon paths for every item, and every prime item with the
parts (blueprint + recipe components) it needs.
"""
import gzip, json, lzma, re, sys, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "catalog.json.gz"
CONTENT = "https://content.warframe.com/PublicExport/"
INDEX = "https://origin.warframe.com/PublicExport/index_en.txt.lzma"

# productCategory -> XP at which the item is mastered
WEAPON_XP = 450000
FRAME_XP = 900000
XP_BY_CATEGORY = {
    "Suits": FRAME_XP, "SpaceSuits": FRAME_XP, "MechSuits": FRAME_XP,
    "Sentinels": FRAME_XP, "KubrowPets": FRAME_XP, "SpecialItems": FRAME_XP,
    "LongGuns": WEAPON_XP, "Pistols": WEAPON_XP, "Melee": WEAPON_XP,
    "SpaceGuns": WEAPON_XP, "SpaceMelee": WEAPON_XP, "SentinelWeapons": WEAPON_XP,
    "OperatorAmps": WEAPON_XP, "Hoverboards": FRAME_XP,
}
ITEM_FILES = ["ExportWeapons", "ExportWarframes", "ExportSentinels"]


def split_camel(s):
    s = s.replace("_", " ")
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", " ", s)


def derive_part_name(item_name, unique):
    seg = re.sub(r"Component$", "", unique.rsplit("/", 1)[-1])
    base = item_name.replace(" Prime", "").replace(" ", "").replace("&", "")
    rest = seg.replace("Prime", "", 1).replace(base, "", 1).strip()
    return f"{item_name} {split_camel(rest or seg)}"


# ---------------------------------------------------------------- sources
def dir_source(d):
    d = Path(d)

    def get(base):
        hits = sorted(p for p in d.glob("*.json") if base.lower() in p.name.lower()
                      and not (base == "ExportWeapons" and "Railjack" in p.name))
        return json.loads(hits[0].read_text(encoding="utf-8")) if hits else None
    return get


def fetch_source():
    def dl(url):
        req = urllib.request.Request(url, headers={"User-Agent": "InventoryCompare-catalog"})
        with urllib.request.urlopen(req, timeout=120) as r:
            return r.read()

    dec = lzma.LZMADecompressor(format=lzma.FORMAT_ALONE)
    raw = dl(INDEX)
    try:
        text = dec.decompress(raw).decode()
    except lzma.LZMAError:
        text = b"".join(dec.decompress(raw[i:i + 1]) for i in range(len(raw))).decode()
    names = [l.strip() for l in text.splitlines() if l.strip()]

    def get(base):
        for n in names:
            if n.startswith(base + "_en.json") or n.startswith(base + ".json"):
                return json.loads(dl(CONTENT + "Manifest/" + n))
        return None
    return get


# ------------------------------------------------------------------- build
def build(get, lang_path=None):
    names, icons = {}, {}
    # keep what an earlier catalog knew so partial sources never lose data
    if OUT.exists():
        old = json.loads(gzip.decompress(OUT.read_bytes()))
        names.update(old["names"]); icons.update(old["icons"])

    if lang_path and Path(lang_path).exists():
        for k, v in json.loads(Path(lang_path).read_text(encoding="utf-8")).items():
            if v.get("name"):
                names[k.lower()] = v["name"]

    items = {}  # unique -> (name, productCategory)
    for base in ["ExportWeapons", "ExportWarframes", "ExportSentinels", "ExportResources",
                 "ExportUpgrades", "ExportCustoms", "ExportFlavour", "ExportRelicArcane",
                 "ExportDrones", "ExportKeys"]:
        data = get(base)
        for lst in (data or {}).values():
            if not isinstance(lst, list):
                continue
            for x in lst:
                if isinstance(x, dict) and x.get("uniqueName") and x.get("name"):
                    names[x["uniqueName"].lower()] = x["name"]
                    if base in ITEM_FILES:
                        items[x["uniqueName"]] = (x["name"], x.get("productCategory"))

    man = get("ExportManifest")
    for m in (man or {}).get("Manifest", []):
        icons[m["uniqueName"].lower()] = m["textureLocation"]

    recipes = (get("ExportRecipes") or {}).get("ExportRecipes", [])
    primes = []
    for r in recipes:
        res = r.get("resultType")
        if res not in items:
            continue
        item_name, cat = items[res]
        parts = [(r["uniqueName"], 1)]  # the blueprint itself
        for ing in r.get("ingredients", []):
            u = ing["ItemType"]
            if "/Recipes/" in u and "Prime" in u:
                parts.append((u, ing.get("ItemCount", 1)))
        if len(parts) < 2:
            continue  # not a prime part recipe
        out_parts = []
        for u, n in parts:
            name = names.get(u.lower())
            if not name:
                name = f"{item_name} Blueprint" if u == r["uniqueName"] else derive_part_name(item_name, u)
                names[u.lower()] = name
            out_parts.append([u, name, n])
            # blueprints show the item's own picture
            if u == r["uniqueName"] and res.lower() in icons:
                icons.setdefault(u.lower(), icons[res.lower()])
        primes.append({"u": res, "n": item_name, "x": XP_BY_CATEGORY.get(cat, WEAPON_XP), "p": out_parts})

    primes.sort(key=lambda p: p["n"])
    return {"names": names, "icons": icons, "primes": primes}


def main():
    args = sys.argv[1:]
    lang = None
    if "--dir" in args:
        d = args[args.index("--dir") + 1]
        get = dir_source(d)
        lang = next((str(p) for p in Path(d).glob("*lang_dict*.json")), None)
    elif "--fetch" in args:
        get = fetch_source()
    else:
        sys.exit(__doc__)
    cat = build(get, lang)
    data = json.dumps(cat, separators=(",", ":"), ensure_ascii=False).encode()
    OUT.write_bytes(gzip.compress(data, 9, mtime=0))
    print(f"{len(cat['names'])} names, {len(cat['icons'])} icons, {len(cat['primes'])} primes "
          f"-> {OUT} ({OUT.stat().st_size/1e3:.0f} kB gz, {len(data)/1e6:.1f} MB raw)")


if __name__ == "__main__":
    main()
