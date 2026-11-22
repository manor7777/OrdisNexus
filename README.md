# Ordis Nexus

A Warframe companion for two friends. It does what AlecaFrame does with your inventory, and adds the thing AlecaFrame doesn't: **comparing inventories with a friend** so you can see what to work on, what to trade and how to progress together.

**Ordis Nexus is an Overwolf app.** It is meant to be installed through Overwolf and to run only inside it, with no separate program and no AlecaFrame. Your inventory comes straight from the game through Overwolf's Game Events Provider (GEP), the same source AlecaFrame itself uses. Friends exchange small snapshot files or codes, with no account and no server.

> **Where things stand.** The Overwolf app lives in [`overwolf/ordis/`](overwolf/ordis) and is being built now. Its engine (inventory parsing, build planning, goals, "what am I close to completing", relics, mastery) is written in plain JavaScript and tested with `node --test`. The manifest, background script and window (Overview, Ordis chat, Goals, Inventory, Relics, Mastery, Foundry, Settings) are written, but have not run inside Overwolf yet. Friend sharing and comparing are still to come. Until it is ready, the repository also contains the earlier **desktop build** (`OrdisNexus.exe` plus a small Overwolf "bridge"). That build is a stop-gap for testing, not the intended product, and it is described in [the legacy section](#legacy-desktop-build-and-bridge) at the bottom. Nothing here has been run against a live game by the author, so expect the first real-game check to need small fixes.

---

## Contents

1. [Install](#install)
2. [Features, tab by tab](#features-tab-by-tab)
3. [How sharing and privacy work](#how-sharing-and-privacy-work)
4. [Live game tracking](#live-game-tracking-eelog)
5. [Overwolf app](#overwolf-app)
6. [Voice and the optional language model](#voice-and-the-optional-language-model)
7. [Auto-update](#auto-update)
8. [Where data is stored](#where-data-is-stored)
9. [Data sources](#data-sources)
10. [Troubleshooting](#troubleshooting)
11. [Building and the catalog](#building-and-the-catalog)
12. [Legacy desktop build and bridge](#legacy-desktop-build-and-bridge)

---

## Install

Needs [Overwolf](https://www.overwolf.com) and Warframe. No AlecaFrame.

1. **When the app is published:** install it from the Overwolf store. Overwolf refuses unverified `.opk` files, so the `.opk` on Releases will not install until the app is approved.
2. **For testing now (developer mode):** download `OrdisNexus-Overwolf-unpacked.zip` from [Releases](../../releases) and extract it. In Overwolf choose **Settings → About → Development options → Load unpacked extension** and pick the extracted folder. (Or use the `overwolf/ordis` folder from the repo.)
3. Start Warframe. Ordis registers for game events and your inventory arrives on its own. It refreshes when the game refreshes it (see [data notes](#things-to-know-about-the-data)).
4. Share with a friend using a snapshot file or code (see [sharing](#how-sharing-and-privacy-work)). Choose what they may see first.

The feature list below describes the whole product. Tabs marked as desktop-build only are being ported to the Overwolf app.

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
- **"What am I close to completing?"** Ranks the items you could finish soonest (your goals plus any Prime you already hold a part of) by how few parts you are missing. For each it says exactly what is missing and where to get it: which relics drop each part (marking relics you already own), which resources you are short of with the best drop locations or farming hint, and any credits shortfall. Add "how" or "where" to the question for the full sources list.
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
- **Overwolf bridge** status: connected or waiting, inventory updates received and when, whether Warframe is running, and recent bridge messages.
- Start/stop watching Warframe's `EE.log`, with status.
- Event list (mission name and start, mission end, credits earned, inventory sync, and any other patterns you add) and a filterable view of the latest raw log lines.
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
3. Start a mission. Events appear in the list. When a mission ends, Ordis waits for your inventory to refresh and then summarises what you gained.

The default patterns were written from a real `EE.log` supplied by a Warframe player, and are covered by tests against lines copied from it:

| Event | Log line it looks for |
|---|---|
| Mission name | `ThemedSquadOverlay.lua: Mission name: …` |
| Mission started | `Game [Info]: OnStateStarted, mission type=…` |
| Mission succeeded | `EndOfMatch.lua: Mission Succeeded` |
| Mission ended (any way) | `GameRulesImpl - changing state from SS_STARTED to SS_ENDING` |
| Credits earned | `EndOfMatch.lua: NotifyTagMultiple(MISSION_REWARD_CREDITS): …` |
| Inventory synced / saved | `OnInventoryResults completed`, `CommitInventoryChangesCallback - Success` |

Warframe can change its log wording in an update. If missions stop being detected, open **Live**, play a mission, read the raw lines, and edit the patterns in **Settings → Live game → Patterns** (a label, text the line must contain, and a kind: `mission_start`, `mission_end`, `mission_name`, `credits` or `note`). Restart watching to apply. **Reset to defaults** restores the shipped ones.

---

## Overwolf app

Ordis Nexus runs only inside Overwolf.

```
Warframe → Overwolf GEP → Ordis Nexus (Overwolf app)
                          background window: receives and stores inventory
                          main window: all tabs, goals, Ordis chat
```

- The app asks Overwolf for the `match_info` and `game_info` features, reads `match_info.inventory` (the whole account as JSON) and keeps it in the app's own storage.
- A partial update never replaces a complete inventory. Only a document that looks like a full account (player level, credits and items present) is accepted.
- No network connection is needed for your data. Nothing leaves your PC unless you share a snapshot.
- Overwolf's developer account is needed to publish on the store. Until then it loads in developer mode as in [Install](#install).

### Things to know about the data

- Warframe does **not** push an updated inventory the instant a mission ends. It can stay stale for a while and often refreshes when you leave the Relay or Orbiter. Ordis shows when the last inventory update arrived.
- Some updates carry no inventory. That is normal.
- The app is checked against a real inventory sample for parsing, but the live Overwolf side has not been run by the author. The Settings diagnostics list the raw messages received, so a wrong feature name or game id can be spotted and fixed quickly.

### Overlays

Overlays are a second in-game window in the same app. Not built yet.

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

- **Your inventory:** Overwolf's Game Events Provider for Warframe. (The legacy desktop build can also read AlecaFrame's file.)
- **Items, recipes, relics, resources:** a bundled catalog (`assets/catalog.json.gz`) generated from Warframe's public export, community drop-rate data (WFCD) and warframe.market item names.
- **Prices:** warframe.market (optional).
- **World state:** the community warframestat.us API.
- AlecaFrame is not used. No AlecaFrame code or assets are included; it was only a feature reference.

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

- The Overwolf app is plain JavaScript. `node overwolf/ordis/build.mjs` bundles the modules into `bundle/background.js` and `bundle/ui.js`, which are the classic scripts the app actually loads (CI does this before packaging; the bundles are also committed so the repo folder loads directly). Tests: `cd overwolf/ordis && node --test test/*.test.js`. The `.opk` is a zip of the app folder, built by CI as `OrdisNexus-Overwolf.opk`. The version lives in `overwolf/ordis/manifest.json`; Overwolf delivers updates from that.
- The legacy exe is built by GitHub Actions (`.github/workflows/build.yml`) on Windows for every push to `main`. It runs the unit tests first, then builds and publishes.
- Locally on Windows: `cargo test` then `cargo build --release`.
- When new items release, refresh the catalog with the **Catalog** workflow (Actions tab → Run workflow). It regenerates `assets/catalog.json.gz` and commits it.
- Code layout: `src/views/` has one file per tab. `src/share.rs` is the snapshot and privacy logic, `src/recipes.rs` and `src/goals.rs` the build planning, `src/trade.rs` the trade planner, `src/eelog.rs` the log watcher, and `src/assistant.rs` the chat.

---

## Legacy desktop build and bridge

Before the Overwolf app existed, Ordis Nexus was built as a standalone Windows program, `OrdisNexus.exe`, that read AlecaFrame's `lastData.dat` and could optionally receive the live inventory from a small Overwolf app, the **Ordis Nexus Bridge** (`overwolf/ordis-bridge`, packaged as `OrdisNexusBridge.opk`). That needs developer mode in Overwolf and a separate program, so it is not suitable for ordinary users. It remains in the repository as an interim build and as a reference, and will be retired once the Overwolf app covers its features.

Short notes if you still use it:
- Download `OrdisNexus.exe` from [Releases](../../releases). **Overview → Use AlecaFrame's file**, or install the bridge and leave **Settings → Inventory source** on Automatic.
- The bridge posts GEP updates to `127.0.0.1:17642` and the desktop app only accepts `overwolf-extension://` origins. **Live → Overwolf bridge** shows connection status and recent messages.
- Sharing uses a synced folder (**Settings → Shared sync folder**) or exported `.nexus.json` files.
- Startup problems are logged to `%LOCALAPPDATA%\OrdisNexus\startup.log`.
