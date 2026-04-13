# Design System Specification: Cyber-Neuro Synthesis

## 1. Overview & Creative North Star: "The Ethereal Command"
This design system is built upon the "Cyber-Neuro Synthesis" paradigm—a fusion of advanced technical telemetry and "Nothing Design" principles. The goal is to move beyond the cluttered "HUD" tropes of sci-fi and instead create a sophisticated, editorial interface that feels like a biological extension of the user’s consciousness.

**The Creative North Star: The Ethereal Command.**
The interface should not feel like software "installed" on a screen; it should feel like a localized density of light and data within a void. We achieve this through:
*   **Intentional Asymmetry:** Avoid perfect grid-mirroring. Use "tension" by placing heavy telemetry data against wide-open negative space.
*   **Tonal Depth:** Replacing structural borders with light-based depth.
*   **Ephemeral Motion:** Transitions are near-instant (75ms), mimicking the speed of neural firing rather than mechanical sliding.

---

## 2. Colors & Surface Philosophy
The palette is rooted in a deep, atmospheric "Substrate" that avoids the amateur mistake of #000 pitch black, favoring instead a layered, cosmic neutral.

### The Substrate
*   **Global Background:** A radial gradient centered on the primary focal point.
    *   `from-zinc-900` (Inner)
    *   `via-[#0a0a0e]` (Middle)
    *   `to-black` (Outer edge)

### Surface Hierarchy (The "No-Line" Rule)
Sectioning must be achieved through background shifts, never 1px solid lines.
*   **Surface (Base):** `#0e0e12` – The foundational plane.
*   **Surface-Container-Low:** `#131317` – For subtle grouping of secondary data.
*   **Surface-Container-High:** `#1f1f25` – For interactive panels and active telemetry.
*   **The Glass & Gradient Rule:** Use `bg-black/40 backdrop-blur-xl` for any floating modal or overlay. Main CTAs should utilize a subtle gradient from `primary` (#ba9eff) to `primary_container` (#ac91f1) to give the light "weight."

---

## 3. Typography: Technical Editorial
We utilize two typefaces to distinguish between "Human Intent" and "System Data."

*   **Space Grotesk (Headings/UI):** The voice of the user. It is expressive yet engineered.
    *   **Display-LG (3.5rem):** Used for singular mission-critical metrics or high-level section headers.
    *   **Headline-SM (1.5rem):** Used for panel titles.
*   **Space Mono (Telemetry/Technical):** The voice of the machine.
    *   **Label-MD (0.75rem):** Used for all data readouts, timestamps, and coordinate tracking.

**Hierarchy Strategy:** Always pair a large `Display-MD` Space Grotesk title with a tiny `Label-SM` Space Mono subtitle. This contrast in scale creates a high-end, bespoke editorial feel that standard UI lacks.

---

## 4. Elevation & Depth: The Layering Principle
Traditional structural lines are prohibited. Instead, we use "Tonal Stacking."

*   **Ambient Shadows (The Neural Pulse):** For floating elements, use a signature glow instead of a black shadow: `shadow-[0_0_15px_rgba(186,158,255,0.15)]`. This mimics the radiance of an OLED screen.
*   **The Ghost Border:** If high-contrast accessibility is required, use the `outline_variant` (#48474c) at **15% opacity**. It should be felt, not seen.
*   **Haptic Transitions:** `duration-75 ease-out`. This system must feel incredibly responsive. Hover states should reflect an immediate "pulse" in brightness rather than a slow fade.

---

## 5. Components

### Buttons & Interaction
*   **Primary:** Gradient background (`primary` to `primary_container`), `on_primary` text. No rounded corners beyond `0.25rem`. 
*   **Tertiary (Ghost):** No background, `Space Mono` text, `primary` color. Underline on hover only.
*   **Neural Chips:** Small, high-density data points using `surface_container_high`. Use `primary` for the "active" state indicator—a 2px dot rather than a full color fill.

### Input Fields & Telemetry
*   **Text Inputs:** No background fill. Only a bottom "Ghost Border" (15% opacity). Upon focus, the border pulses to 100% `primary` opacity.
*   **Checkboxes/Radios:** Square (`rounded-sm`). Checked states use the "Neural Pulse Glow" to indicate vitality.

### Cards & Lists (The "Anti-Divider" Rule)
*   **Cards:** Forbid the use of divider lines. Separate content using `1.5rem` to `2rem` of vertical whitespace or a transition from `surface-container-low` to `surface-container-highest`.
*   **Lists:** Leading elements (icons/numbers) should be set in `Space Mono` at `0.6875rem` to emphasize the technical nature of the mission control substrate.

---

## 6. Do’s and Don’ts

### Do:
*   **Nesting:** Place a `surface_container_highest` element inside a `surface_container_low` section to create natural focus.
*   **Negative Space:** Allow headers to "breathe" with at least 64px of top margin.
*   **Micro-Telemetry:** Add non-interactive `Space Mono` labels like `[SYS_LOAD_04]` in corners to reinforce the Cyber-Neuro aesthetic.

### Don't:
*   **No #000:** Never use pure pitch black for backgrounds; it kills the "depth" of the radial gradient.
*   **No Rounded-3XL:** The maximum corner radius is `xl` (0.75rem). The system must feel precise and sharp, not "bubbly" or consumer-grade.
*   **No Heavy Shadows:** Never use `black/50` drop shadows. If it doesn't glow, it shouldn't have a shadow.
*   **No Dividers:** If you feel the need to draw a line to separate content, increase the whitespace or shift the background tone instead.

---

## 7. Signature Element: The Pulse Transition
Whenever a new data panel is initialized, it should not slide. It should appear at 90% opacity and instantly "snap" to 100% with a slight `backdrop-blur` increase. This is the hallmark of the Cyber-Neuro Synthesis: speed over ornamentation.