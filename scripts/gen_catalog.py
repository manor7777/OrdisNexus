#!/usr/bin/env python3
"""Build assets/catalog.json.gz from Warframe's official Public Export files.

  python scripts/gen_catalog.py --dir <folder with Export*.json files>
  python scripts/gen_catalog.py --fetch        # download the latest Public Export

Files are matched by name (any prefix/suffix), so `ExportRecipes.json`,
`ExportRecipes_en.json` and `abc123-ExportRecipes.json` all work. An optional
`warframe_lang_dict.json` ({lowercase unique name: {"name": ...}}) adds names.

Output: names + icon paths for every item, every prime item with the parts it
needs, plus (schema v2) all recipes, masterable items, relics and their rewards,
resource location hints, market slugs and drop sources.
"""
import gzip, json, lzma, re, sys, urllib.request
from urllib.error import URLError
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "catalog.json.gz"
CONTENT = "https://content.warframe.com/PublicExport/"
INDEX = "https://content.warframe.com/PublicExport/index_en.txt.lzma"

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
        req = urllib.request.Request(url, headers={"User-Agent": "OrdisNexus-catalog"})
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
                 "ExportDrones", "ExportKeys", "ExportGear"]:
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


DROPS_URL = "https://raw.githubusercontent.com/WFCD/warframe-drop-data/gh-pages/data/all.slim.json"
MARKET_URL = "https://api.warframe.market/v2/items"
REFINEMENTS = ("Bronze", "Silver", "Gold", "Platinum")


def http_json(url):
    req = urllib.request.Request(url, headers={"User-Agent": "OrdisNexus-catalog", "Accept": "application/json",
                                                "Language": "en", "Platform": "pc"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.loads(r.read())


def strip_store(u):
    return u.replace("/StoreItems", "", 1)


def _chance(v):
    import math
    try:
        f = float(v or 0)
    except (TypeError, ValueError):
        return 0
    return round(f, 2) if math.isfinite(f) else 0


def build_v2(get, cat, online=True):
    """Adds recipes / items / relics / res / slugs / drops to `cat` (best effort per section)."""
    names = cat["names"]

    # ---- recipes (result -> how to build it)
    recipes = {}
    for r in (get("ExportRecipes") or {}).get("ExportRecipes", []):
        res = r.get("resultType")
        if not res or res.lower() in recipes:
            continue
        recipes[res.lower()] = {
            "r": res, "b": r["uniqueName"], "c": r.get("buildPrice", 0), "t": r.get("buildTime", 0),
            "n": r.get("num", 1), "i": [[i["ItemType"], i.get("ItemCount", 1)] for i in r.get("ingredients", [])],
        }
        # give blueprints and components readable names when nothing better is known
        rn = names.get(res.lower())
        if rn and r["uniqueName"].lower() not in names:
            names[r["uniqueName"].lower()] = rn + " Blueprint"
    cat["recipes"] = recipes

    # ---- masterable items
    items = []
    for base, default_cat in (("ExportWarframes", "Suits"), ("ExportWeapons", None), ("ExportSentinels", "Sentinels")):
        data = get(base) or {}
        for key, lst in data.items():
            if key not in ("ExportWarframes", "ExportWeapons", "ExportSentinels") or not isinstance(lst, list):
                continue
            for x in lst:
                u, n = x.get("uniqueName"), x.get("name")
                if not u or not n:
                    continue
                pc = x.get("productCategory") or default_cat
                if pc is None:
                    continue
                if base == "ExportWarframes" and "/Archwing/" in u:
                    pc = "SpaceSuits"
                items.append({"u": u, "n": n, "c": pc, "m": x.get("masteryReq", 0),
                              "x": XP_BY_CATEGORY.get(pc, WEAPON_XP)})
    cat["items"] = items

    # ---- relics
    relics = {}
    for r in (get("ExportRelicArcane") or {}).get("ExportRelicArcane", []):
        u = r.get("uniqueName", "")
        if "VoidProjection" not in u or not r.get("relicRewards"):
            continue
        base = u
        for ref in REFINEMENTS:
            if u.endswith(ref):
                base = u[: -len(ref)]
        if base.lower() in relics:
            continue
        relics[base.lower()] = {
            "n": r["name"].replace(" Relic", ""),
            "rw": [[strip_store(w["rewardName"]), w.get("rarity", "COMMON").lower(), w.get("itemCount", 1)]
                   for w in r["relicRewards"]],
        }
    cat["relics"] = relics

    # ---- resource location hints from the in-game description
    res = {}
    for x in (get("ExportResources") or {}).get("ExportResources", []):
        d = x.get("description") or ""
        m = re.search(r"Location:\s*(.+)", d, re.S)
        if m:
            res[x["uniqueName"].lower()] = " ".join(m.group(1).split())[:400]
    cat["res"] = res

    # ---- warframe.market slugs (+ names for parts that have none)
    slugs = {}
    if online:
        try:
            for it in http_json(MARKET_URL).get("data", []):
                ref, slug = it.get("gameRef"), it.get("slug")
                if ref and slug:
                    slugs[ref.lower()] = slug
                    nm = (it.get("i18n", {}).get("en", {}) or {}).get("name")
                    if nm and "prime" in slug:
                        names[ref.lower()] = nm
        except (URLError, ValueError, OSError) as e:
            print("market list failed:", e)
    cat["slugs"] = slugs

    # ---- drop sources by item name
    drops = {}
    if online:
        try:
            for d in http_json(DROPS_URL):
                place = re.sub(r"<[^>]+>", "", d.get("place", ""))
                item = d.get("item")
                if not item or "Relic" in place:
                    continue
                drops.setdefault(item, []).append([place, (d.get("rarity") or "")[:1], _chance(d.get("chance"))])
        except (URLError, ValueError, OSError) as e:
            print("drops failed:", e)
    cat["drops"] = drops
    return cat


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
    cat = build_v2(get, cat, online="--fetch" in args)
    if OUT.exists():
        old = json.loads(gzip.decompress(OUT.read_bytes()))
        for k in ("slugs", "drops"):
            if not cat.get(k) and old.get(k):
                cat[k] = old[k]
    data = json.dumps(cat, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode()
    OUT.write_bytes(gzip.compress(data, 9, mtime=0))
    print(f"{len(cat['recipes'])} recipes, {len(cat['items'])} items, {len(cat['relics'])} relics, "
          f"{len(cat['res'])} resource hints, {len(cat['slugs'])} market slugs, {len(cat['drops'])} dropped items")
    print(f"{len(cat['names'])} names, {len(cat['icons'])} icons, {len(cat['primes'])} primes "
          f"-> {OUT} ({OUT.stat().st_size/1e3:.0f} kB gz, {len(data)/1e6:.1f} MB raw)")


if __name__ == "__main__":
    main()
