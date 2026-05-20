# Script Library Notes

- The app writes OCR snapshots to `Scripts/.cache/snapshot.json` every ~5s. Use `snapshot_reader.py` to consume health/mana/gold/sync data from Python scripts.
- External repos are unpacked here with minimal wrappers (`run.bat`). They still need their own dependencies (Python, Selenium, etc.).
- To hook a script into shared state:
  1) Import `snapshot_reader` (adjacent) and call `load_snapshot()` instead of doing your own OCR.
  2) Avoid sending inputs if `Dry run` is checked in the UI.
- Repos included:
  - BazaarTCBot (Wizard101-Bazaar-Tc-Farming)
  - CrownsTriviaBot (FreekiGames-Trivia-Bot) — needs manual clone.
  - SeleniumCaptchaBot (Wizard101Bot-Selenium-Captcha) — needs manual clone.
  - wizAPI (wizAPI-master)
  - wizSDK (wizSDK-master)
  - wizwalker (wizwalker-master)
  - Wizard101-Farming
  - Wizard101-Utilities

If a repo folder is missing, the launcher opens its README/folder. Install dependencies per each repo’s instructions.
