# The Search For Speed Analyzer — v0.5.12

### Immediate Post-Exit Tire Checks & Pit-Exit Popup Focus:
* **Faster Post-Exit Tire Capture (`Esc`):**
  * Cockpit `TIRES` and Stint Log tires now continue checking live SDK wear/temperatures immediately after exiting the car (with upward carcass-temp validation so cold resets never overwrite real data), paired with a 200 ms / 6-second post-exit `CarSetup` YAML burst poll so measurements latch as soon as iRacing publishes them.
* **Pit-Exit-Only Floating Tire Popup:**
  * Kept the floating post-pit Tire Popup strictly tied to **Pit Exit** (driving back out onto the track) so it never pops up uninvited when exiting the car or opening the Garage.

---

# The Search For Speed Analyzer — v0.5.11

### Seamless 100% Corner & Short-Stint Tire Capture:
* **Unworn `100%` Corners Now Update Immediately on Pit Stops:**
  * Fixed an issue (notably on Dirt Late Models where `LF` often stays at `100%` while `RF`/`LR`/`RR` wear to `99%`–`95%`) where individual `100%` corners waited until you opened the Garage before appearing.
  * Pit-stall detection now evaluates service across all 4 corners as one set, attaching `100%` wear, temperatures, and pressures right alongside worn corners immediately in the pit stall.
* **Short-Run `100%` Stints with Hot Temperatures:**
  * Short runs where all 4 corners remain at `100%` tread—whether captured in the pit stall via carcass temperature jump or in the Garage with real thermal spread—now attach cleanly and survive local Stint DB sync, while uniform cold setup resets continue to be ignored.

---

# The Search For Speed Analyzer — v0.5.10

### Instant Garage & Pit-Exit Tire Latching + Clean Release History:
* **Immediate Garage Tire Capture on Exit-Car (`Esc`):**
  * Eliminated a frame-ordering delay where exiting the car on the same tick iRacing published garage tire measurements could skip attaching those tires until the next YAML refresh.
  * Exiting the car now forces an immediate YAML check and UI refresh so wear, hot pressures, and pyrometer temperatures attach right away.
* **4-Corner Garage Tire Validity & Doc Cleanup:**
  * Ensured garage tire detection checks all 4 corners consistently, and streamlined `RELEASE_NOTES.md` and `CHANGELOG.md` into clean driver-focused highlights.
