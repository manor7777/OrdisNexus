# Ordis Nexus

A Warframe companion in the spirit of AlecaFrame, built for two friends to compare inventories, plan trades and work on goals together. It is a single Windows `.exe` with no installer, and it reads AlecaFrame's own data file. It has no server and needs no account.

Download `OrdisNexus.exe` from the [Releases](../../releases) page and run it. The app checks for new releases itself (see "Auto-update").

## Setup (2 minutes)

1. Install and run AlecaFrame at least once so it writes `%LOCALAPPDATA%\AlecaFrame\lastData.dat`.
2. Open Ordis Nexus, go to **Overview** and press **Use AlecaFrame's file**.
3. Both of you go to **Settings → Shared sync folder** and pick the same folder that syncs between your PCs (OneDrive, Dropbox, Google Drive, Syncthing). Each of you writes one small `.nexus.json` snapshot there and reads the other's.
4. Go to **Friends** and choose what you share.

No shared folder? Use **Friends → Export snapshot** and send the file over Discord. The other person loads it with **Load a .nexus.json**, or drags it onto the window.

## What it does

| Tab | Purpose |
|---|---|
| Overview | Your stats, recommendations, notices, loading inventories |
| Ordis | Ask questions in plain English ("what do I need for Wisp Prime?", "does Sam have a Nikana Prime blade?") |
| Goals | Build an item, collect N of something, or reach a mastery target, with the missing parts, resources, drops and relic sources worked out |
| Inventory | Searchable list with duplicates filter |
| Foundry | What is building, what is ready, what you could build now |
| Relics | Your relics and which rewards your goals still need |
| Mastery | Owned versus mastered, with filters |
| World | Cycles, fissures, sortie, Baro, invasions (from warframestat.us) |
| Friends | Sharing controls, mutual-help matching, search your friend's shared items |
| Compare | Raw side-by-side comparison with a trade planner, if you have each other's `lastData.dat` |
| Live | Game-log tracking and mission summaries |
| Settings | Names, paths, notifications, patterns, assistant options |

## Privacy

Nothing leaves your PC except the snapshot file in your own sync folder. A friend only sees what you tick under **Friends → What you share**: goals, activity note, giveable items (by your rules), mastery, resource quantities. Your full inventory is off by default. Giveable items can be limited to spares, to items you have mastered, to a platinum ceiling, with always-share and never-share lists. Platinum prices come from warframe.market and can be turned off in Settings.

## Live game features (need the game running; optional)

These were written without access to a running game, so treat them as best-effort and adjust them as described.

### Mission tracking via EE.log

Warframe writes `EE.log` as it runs. Ordis Nexus tails it read-only, with no injection and no Overwolf needed.

1. Default path: `%LOCALAPPDATA%\Warframe\EE.log`. Override it in **Settings → EE.log path**.
2. **Live → Start watching**, or tick "Start watching the game log on launch".
3. When a mission starts and ends, Ordis records it. After AlecaFrame refreshes your inventory, Ordis summarises what you gained by comparing the inventory before and after.
4. If missions are not detected, the log wording has probably changed. Open **Live**, run a mission, and look at the raw log lines. Then edit the patterns in **Settings → Live game**. Each pattern is a piece of text to match plus a kind (`mission_start`, `mission_end` or `note`). Changes apply the next time you start watching.

### Overlays and relic-reward screens (Overwolf)

Drawing on top of the game, or reading the relic reward screen, needs Overwolf's Game Events Provider, which only an Overwolf app can use. This exe does not include one. To add it:

1. Install [Overwolf](https://www.overwolf.com/) and create an app with the [Overwolf developer docs](https://overwolf.github.io/).
2. Use the Warframe game-events API (game id 8954) for mission and reward events.
3. Have the app write events to a file or a local HTTP endpoint that this app can watch. A new pattern in **Settings → Live game** matching your app's log line is the simplest bridge.

Until then, relic and mission information comes from EE.log and your inventory file only.

### Voice

- **Speaking:** tick "Speak answers aloud" in Settings. It uses Windows' built-in speech (SAPI).
- **Listening:** use Windows dictation (Win+H) in the Ordis chat box, or any speech-to-text tool such as a local Whisper app that types into the focused window.

### Optional language model

Ordis answers from your data with fixed logic. If you want friendlier wording, enable **Let a language model rephrase answers** and point it at any OpenAI-compatible server. For a local one, install [Ollama](https://ollama.com), run `ollama pull llama3.2`, and use the default endpoint `http://localhost:11434/v1`. The model only rewords the answer; facts always come from your data.

## Auto-update

On launch the app checks this repo's latest release and offers to install it. This works because the repo is public. If it is private, the check fails quietly and you update manually from Releases.

## Notes and limits

- AlecaFrame's file is decrypted with the same key as AlecaFrame itself. If a future AlecaFrame release changes it, set the `ALECAFRAME_KEY` and `ALECAFRAME_IV` environment variables (hex) to override.
- Item data comes from a bundled catalog (`assets/catalog.json.gz`) generated from Warframe's public export and community drop data. Refresh it with the **Catalog** workflow in GitHub Actions when new items release.
- This project contains no AlecaFrame code or assets. AlecaFrame was used only as a feature reference.

## Building

The exe is built by GitHub Actions (`.github/workflows/build.yml`) on every push to `main`. Locally: `cargo build --release` on Windows.
