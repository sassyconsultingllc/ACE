Title: Interaction-Based Page Trust System for Web Browsers

Field

This disclosure relates to web browser security, and more specifically to systems and methods that dynamically assign trust levels to web pages based on measured, user-originated interactions to gate sensitive capabilities.

Background

Existing browsers grant broad runtime capabilities to pages once the page requests permission or users click dialogs. This enables automated or deceptive attacks (clickjacking, drive-by downloads). There is a need for mechanisms that require demonstrated, meaningful user intent before enabling sensitive operations.

Summary

A browser-integrated Page Trust system that: (a) initializes every page in a restricted state ("Untrusted"); (b) records typed interaction events (clicks, keyboard input, user-initiated scrolls, form submissions) with contextual heuristics (element size, element type, time-since-load, minimum spacing between events); (c) counts only "meaningful" interactions per defined rules; (d) promotes page trust through discrete trust levels (Cautious, Warming, Trusted) after a threshold of meaningful interactions; and (e) gates sensitive capabilities (clipboard, downloads, notifications, geolocation, autoplay, fullscreen, popup creation) according to current trust level.

Detailed Description (summary points)

- Define interaction types and criteria for "meaningful":
  - Clicks: element dimensions >= 20x20 px; non-ad element types; spaced >= 1s apart; page age >=2s.
  - Keyboard input: in-form-field and char_count >= 3.
  - Scroll: user_initiated true and delta in acceptable range.
  - Form submission: always meaningful.
- Maintain per-page sandbox state: trust enum, interaction log, meaningful_count, timestamps, blocked_actions.
- Transition trust state at discrete counts (1->Cautious, 2->Warming, 3+->Trusted).
- When action requested, consult sandbox to allow or block and record blocked action with reason.
- UI surfaces trust status (text + color) and blocked-action review.

Example Advantages

- Prevents drive-by downloads and automated permission grants.
- Hardens against clickjacking by requiring minimum element size and time spacing.
- Allows legitimate workflows (OAuth popups after user gesture) while blocking surprise behavior.

Potential Claims (examples)

1. A method for gating web page capabilities comprising: initializing a page as untrusted; monitoring user-originated interactions; determining whether each interaction meets a set of criteria for being meaningful; incrementing a meaningful interaction counter when criteria are met; promoting the page to higher trust states when the counter reaches thresholds; and permitting a restricted capability only if the page's trust state satisfies a required level.

2. The method of claim 1 where the criteria for meaningful clicks include element dimension thresholds and exclusion of known ad element types.

3. The method of claim 1 where interactions separated by less than a minimum interval are ignored to prevent automated scripts from triggering trust elevation.

Figures (suggested)

- Flowchart of interaction -> is_meaningful -> increment -> trust transition -> permission check.
- State diagram of trust levels and gated capabilities.


---

(End of draft - suitable for provisional filing; further expansion available on request.)