# Ordis Nexus

A Warframe companion for two friends. It does what AlecaFrame does with your inventory, and adds the thing AlecaFrame doesn't: **comparing inventories with a friend** so you can see what to work on, what to trade and how to progress together.

It is one Windows `.exe`. There is no installer, no server and no account. It reads AlecaFrame's own data file from its own folder, and friends exchange small snapshot files through a folder you both already sync.

> Status: built and tested automatically (unit tests run on every push), but **the author could not run it against a live game**. Everything that reads your inventory file is covered by tests with realistic data. The parts that depend on the running game (the log tracking) are best-effort, and the README explains exactly how to adjust them.

---

## Contents

1. [Quick start](#quick-start)
2. [Features, tab by tab](#features-tab-by-tab)
3. [How sharing and privacy work](#how-sharing-and-privacy-work)
4. [Live game tracking](#live-game-tracking-eelog)
5. [Why there is no Overwolf support (and how to add it)](#why-there-is-no-overwolf-support-and-how-to-add-it)
6. [Voice and the optional language model](#voice-and-the-optional-language-model)
7. [Auto-update](#auto-update)
8. [Where data is stored](#where-data-is-stored)
9. [Data sources](#data-sources)
10. [Troubleshooting](#troubleshooting)
11. [Building and the catalog](#building-and-the-catalog)

---

## Quick start

1. Install and run **AlecaFrame** at least once with Warframe open, so it writes `%LOCALAPPDATA%\AlecaFrame\lastData.dat`. Ordis Nexus reads that file. It does not talk to the game.
2. Download `OrdisNexus.exe` from [Releases](../../releases) and run it.
3. **Overview → Use AlecaFrame's file.** Your inventory loads and reloads itself whenever AlecaFrame updates the file.
4. **Settings → Shared sync folder.** Both of you pick the same folder that syncs between your PCs (OneDrive, Dropbox, Google Drive, Syncthing). Each of you writes one `.nexus.json` snapshot there and reads the other's.
5. **Friends → What you share.** Choose what your friend may see. Defaults are conservative.
6. Put your names in **Settings** so snapshots and chat use them.

No shared folder? Use **Friends → Export snapshot to a file** and send the file over Discord. Your friend either drags it onto the window or uses **Load a .nexus.json**.

---

## Features, tab by tab

### 🏠 Overview
- Shows your inventory status (items, total copies, file location) and your friend's.
- Load your own file from AlecaFrame's default location, browse to another file, or reload.
- Optionally load your friend's raw `lastData.dat` (if they sent you one) for the full Compare tab.
- Stat cards: Mastery Rank, credits, platinum, endo.
- **What should I do tonight?** A ranked list of suggestions, for example crack a relic that rewards a part your goals need, farm a resource you are short of, ask your friend for a part they have spare, swap parts you can each give, or farm together for something you are both short of.
- Notices feed: goal completed, a friend newly has something you need, mission started/ended. Each kind can be turned off in Settings.
- Drag a `.nexus.json` onto the window anywhere to load a friend snapshot.

### 💬 Ordis (assistant)
Ask in plain English. Answers are computed from your own data, never invented. Examples:
- "What am I missing for Wisp Prime?"
- "What should I farm tonight?"
- "Does my friend have Wisp Prime Systems?" / "Can my friend help me?"
- "What can I build right now?" / "Which relics should I crack?"
- "What are my goals?" / "How much Ferrite do I have?"
- "Any good fissures? Is Baro here? Any invasions?"
- "What's my mastery progress? What's in the Foundry?"

Optional extras: speak answers aloud, and let a language model reword them (see [Voice and the optional language model](#voice-and-the-optional-language-model)).

### 🎯 Goals
- Three kinds: **Build** an item, **Collect** N of something, or work toward a **Mastery** target.
- Type a name and pick from suggestions. Goals have a count and a free-text note.
- For a build goal Ordis walks the recipe tree (up to four levels deep) and shows:
  - which parts you have built, which you only hold as blueprints, and which you still need;
  - for each missing part, the relics that drop it, marking relics you own;
  - the resources still short across the whole tree, with drop locations and hints;
  - total credits and build time;
  - an overall progress bar.
- A goal completes automatically when the item shows up as owned in your inventory.
- Each goal has a **share** checkbox, so you decide which goals your friend sees.

### 🎒 Inventory
- Everything you own in one searchable, filterable list: search by name, filter by category, duplicates-only (your spares).
- Icons are downloaded on demand and cached.

### 🔨 Foundry
- Everything currently building, with time remaining or "ready to claim".
- **Ready to build now:** blueprints you hold where every resource and the credits are covered.
- Credits, platinum and endo totals.

### 🔮 Relics
- All relics you own by type and refinement (Intact, Exceptional, Flawless, Radiant) with counts and every reward and its rarity.
- Highlights rewards your goals still need.
- Search by relic or reward, and a toggle to show only relics that help a goal.

### ⭐ Mastery
- Mastery Rank, mastered count against the total, and how many items you own but have not levelled yet.
- Filters: not mastered, owned but not mastered, not owned, mastered, by item type, plus search.
- Mastery data is only shared with your friend if you tick it.

### 🌍 World
- Open-world cycles, void fissures (with Steel Path and Railjack tags), Sortie, Archon hunt, Baro, Nightwave, Steel Path reward and invasions.
- Fetched from the community world-state API and refreshed on demand.

### 🤝 Friends
- **Their snapshot:** name, how old the snapshot is, their activity note, mastery rank, and the goals they chose to share with progress bars.
- **Working together:** what your friend has that you need, what you have that they need, whether a mutual swap is possible, and resources you are both short of (farm together).
- **Does your friend have…?** Search their shared items.
- **What you share:** per-category switches (see [Privacy](#how-sharing-and-privacy-work)).
- **Which items count as giveable:** your rules.
- **Preview** of exactly what a friend would see as giveable, and a manual snapshot export.

### ⚖ Compare
For when you have your friend's raw `lastData.dat`. This is the original Inventory Compare functionality:
- **Trades:** for each direction, the parts one of you can hand to the other. Parts of items the giver has already mastered are marked safe. Spare copies are marked as your call. Extra copies nobody needs are listed as free to sell.
- **Still needed:** parts missing to build each prime, filtered to "both need", "only you need", "only they need" or everything, optionally only primes someone has started.
- **Full compare:** every item side by side with counts, filtered by who has more, only one has, or the same, by category and by search.
- Blueprints count as owning a part for "do I have it", but only the blueprint form can be handed over.

### 📡 Live
- Start/stop watching Warframe's `EE.log`, with status.
- Event list (mission started, mission ended, and any other patterns you add) and a filterable view of the latest raw log lines.
- After a mission, Ordis compares your inventory before and after, once AlecaFrame refreshes the file, and tells you what you gained.

### ⚙ Settings
Names, file paths, shared folder, auto-reload, notification switches, platinum price lookups on/off, log-watching on launch, the editable log patterns, voice, the language model options, version and update check.

---

## How sharing and privacy work

- There is no server. Nothing is uploaded anywhere. The only thing that leaves your PC is a snapshot file written into the folder you chose.
- **Technical access is not social permission.** Ordis can read your whole inventory, but a friend only ever sees what you tick:
  - your goals and progress
  - an activity note ("farming Rhino Prime relics")
  - items you could give away (by your rules)
  - mastery rank and mastered items
  - resource quantities
  - your entire inventory (**off** by default)
- **Giveable item rules:** include prime parts, mods, relics; only parts of items you have already mastered; keep N spares; a platinum price ceiling using warframe.market (0 turns it off); whether to share items whose price is unknown; plus always-share and never-share lists matched by name.
- Prices come from warframe.market, rate-limited and cached on disk for six hours. You can turn price lookups off.
- The snapshot is plain JSON, so you can open it and see exactly what you are sharing.

---

## Live game tracking (EE.log)

Warframe writes a log file as it runs. Ordis Nexus reads it, read-only, from the end of the file, so it only sees new activity. It does not inject into the game and needs no extra software.

1. Default path: `%LOCALAPPDATA%\Warframe\EE.log`. Override it in **Settings → EE.log path**.
2. Open **Live → Start watching** (or tick "Start watching on launch").
3. Start a mission. Events appear in the list. When a mission ends, Ordis waits for AlecaFrame to refresh your inventory and then summarises what you gained.

**Important: the log wording is best-effort.** The author could not run the game, and the exact lines Warframe writes can change between updates. If missions are not detected:

1. Go to **Live**, play a mission, and read the raw log lines on the right (there is a filter box).
2. Find a line that appears when a mission starts and one when it ends.
3. **Settings → Live game → Patterns:** add or edit a pattern. Each has a label, a piece of text the line must contain, and a kind: `mission_start`, `mission_end` or `note`.
4. Stop and start watching to apply it. If you find better patterns, **Reset to defaults** goes back to the shipped ones.

---

## Why there is no Overwolf support (and how to add it)

This was a deliberate decision, not an oversight, so here is the reasoning.

**What Overwolf would give you.** Overwolf is a platform that can draw overlays on top of a running game and receive structured "game events" through its Game Events Provider (GEP). In theory that means things like an in-game overlay and reading the relic reward screen.

**Why it is not in this build:**

1. **It is a different kind of program.** An Overwolf app is a packaged web app (HTML/JavaScript) that only runs inside Overwolf. It cannot be a plain `.exe`. Supporting it means writing and maintaining a second application, not a feature added to this one.
2. **It needs Overwolf installed and, to be shared, approved.** Both of you would have to install Overwolf. Publishing through their store involves review. Running an unpublished one requires developer mode. That is heavy for two people.
3. **It cannot be developed blind.** An overlay or event consumer can only be checked against the live game and a running Overwolf. The author had neither, and shipping untestable code that claims to work is worse than being upfront that it is missing.
4. **Almost everything that matters here does not need it.** Inventory, goals, relics, foundry, mastery, sharing, trade planning and the assistant all come from the inventory file. Mission start/end comes from the log file. Overwolf only adds *in-game overlays* and, possibly, live reward-screen reading.
5. **A standalone exe keeps things simple and private.** No extra runtime, no account, no store, no ads platform, nothing running inside the game process.

I also could not confirm which events Overwolf's GEP actually exposes for Warframe, so I did not want to promise features that may not exist. **Check Overwolf's current Warframe documentation before building on it.**

**If you (the friend, who has the game) want to add it:**

1. Install [Overwolf](https://www.overwolf.com/) and read the [developer documentation](https://overwolf.github.io/). Use developer mode and "Load unpacked extension" to run your own app without publishing.
2. Create a small app that subscribes to whatever Warframe events GEP currently provides, and optionally draws an overlay.
3. Have that app append one line per event to a text file, or write to a local file the Ordis Nexus log watcher can read. You can point **Settings → EE.log path** at that file, or add a second watcher for it.
4. In **Settings → Live game → Patterns**, add a pattern matching your event text, with kind `mission_start`, `mission_end` or `note`. Those then show up in **Live** and trigger the mission summary.

This reuses everything already built (pattern matching, event list, before/after inventory summary). The only new piece is the Overwolf app that produces the lines. If you build it and it works, a pull request is welcome.

**What you can do today without Overwolf:** use the relic and goal views to decide what to crack, check **World → fissures**, and read the mission summaries from the log.

---

## Voice and the optional language model

- **Speaking:** tick "Speak answers aloud" in Settings. This uses Windows' built-in speech (SAPI), so nothing is downloaded.
- **Listening:** there is no built-in microphone input. Use Windows dictation (Win+H) while the Ordis chat box is focused, or any speech-to-text tool (for example a local Whisper app) that types into the focused window.
- **Language model (optional, off by default):** Ordis already answers from fixed logic over your data. If you want friendlier wording, enable "Let a language model rephrase answers" and point it at any OpenAI-compatible server. For a local one, install [Ollama](https://ollama.com), run `ollama pull llama3.2`, and use the default endpoint `http://localhost:11434/v1` and model `llama3.2`. A hosted service also works if you provide its base URL, model and API key. The model only rewords; the facts always come from your data. If the model is unreachable you simply get the plain answer.

---

## Auto-update

On launch the app checks this repository's latest release and offers to install it. This works for a **public** repository. If the repo is private the check fails quietly and you update manually from [Releases](../../releases). **Settings → Check for updates** runs the check on demand.

Builds from `main` publish as prereleases named like `v0.3.0-build.4`. Pushing a version tag publishes a normal release.

---

## Where data is stored

`%LOCALAPPDATA%\OrdisNexus\`
- `settings.json`: your settings, goals, rules and patterns
- `icons\`: cached item icons
- price cache for warframe.market lookups

Nothing is written to AlecaFrame's folder. Delete the folder to reset the app.

---

## Data sources

- **Your inventory:** AlecaFrame's `lastData.dat`. The file is encrypted with the key AlecaFrame itself uses. If a future AlecaFrame release changes it, set `ALECAFRAME_KEY` and `ALECAFRAME_IV` (hex) as environment variables to override.
- **Items, recipes, relics, resources:** a bundled catalog (`assets/catalog.json.gz`) generated from Warframe's public export, community drop-rate data (WFCD) and warframe.market item names.
- **Prices:** warframe.market (optional).
- **World state:** the community warframestat.us API.
- No AlecaFrame code or assets are included. AlecaFrame was used only as a feature reference.

---

## Troubleshooting

| Problem | What to try |
|---|---|
| "Decrypted fine, but no items" or decrypt error | AlecaFrame may not have refreshed the file yet, or changed its key. Open Warframe with AlecaFrame running, then reload. See the key override above. |
| Friend's snapshot never appears | Both of you must use the same synced folder. Check the sync has finished, and that each name differs so the files have different names. |
| Prices missing | warframe.market may be rate-limiting. They fill in over time. Turn price lookups off if you don't need them. |
| Missions not detected | See [Live game tracking](#live-game-tracking-eelog) and edit the patterns. |
| Icons are blank | They download on demand. Check your connection. |
| Updates do not install | The repository must be public, or update manually. |

---

## Building and the catalog

- The exe is built by GitHub Actions (`.github/workflows/build.yml`) on Windows for every push to `main`. It runs the unit tests first, then builds and publishes.
- Locally on Windows: `cargo test` then `cargo build --release`.
- When new items release, refresh the catalog with the **Catalog** workflow (Actions tab → Run workflow). It regenerates `assets/catalog.json.gz` and commits it.
- Code layout: `src/views/` has one file per tab. `src/share.rs` is the snapshot and privacy logic, `src/recipes.rs` and `src/goals.rs` the build planning, `src/trade.rs` the trade planner, `src/eelog.rs` the log watcher, and `src/assistant.rs` the chat.
