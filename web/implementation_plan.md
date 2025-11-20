# Implementation Plan - UI Polish & Bug Fixes

The user has reported two specific issues:
1.  **Visual:** The leaf icon in the header is not vertically centered with the "FlowBridge" text.
2.  **Functional:** Task labels are not appearing on cards and are failing to save.

## User Review Required

> [!IMPORTANT]
> I will be modifying the `app.js` logic for label handling. If the backend expects a specific format that differs from what I find, I might need to check `main.rs`.

- **Icon Alignment:** I will use Flexbox on the `.app-title` container to perfectly center the text and the icon.
- **Labels:** I will debug the `createTask` and `saveTaskDetails` functions to ensure the `labels` array is correctly populated and sent to the API. I will also verify the `createTaskCard` function to ensure it correctly iterates over and renders the labels.

## Proposed Changes

### UI Polish (Icon Alignment)

#### [index.html]
- Update the `.app-title` structure if necessary to support flexbox (it might already be a div).

#### [modern.css]
- Update `.app-title` to use `display: flex`, `align-items: center`, and `gap` for spacing.
- Remove the inline styles from the SVG in `index.html` and move them to CSS for cleaner separation.

### Bug Fix (Labels)

#### [Backend]
- **Issue:** The running server returns `"label": null` (singular), but the source code defines `labels: Vec<Label>` (plural). This indicates the binary is outdated.
- **Fix:** Force a rebuild of the Rust backend (`cargo build --release`).

#### [app.js]
- **Investigation:**
    - Check `selectedLabels` Set to ensure it's being populated correctly by the picker.
    - Check `createTask` to see how it converts the Set to an array of objects.
    - Check `createTaskCard` to see if it expects `task.labels` to be an array of objects or strings.
- **Fixes:**
    - Ensure `createTask` and `saveTaskDetails` map the selected label names back to the full label objects (including color) before sending to the API, as the UI relies on `label.color`.
    - Verify `renderTasks` handles the `task.labels` data structure correctly.

### Gamification (Dopamine Micro-Rewards)

#### [index.html]
- Add `canvas-confetti` library via CDN (`https://cdn.jsdelivr.net/npm/canvas-confetti@1.6.0/dist/confetti.browser.min.js`).

#### [app.js]
- **New Function:** `triggerConfetti(type)`
    - `full`: Large explosion for task completion.
    - `mini`: Small burst for step completion.
- **Updates:**
    - `updateTaskStatus`: Call `triggerConfetti('full')` when status changes to 'complete'.
    - `toggleStep`: Call `triggerConfetti('mini')` when a step is checked.
    - `playSound`: Review and potentially enhance the 'success' sound frequency ramp for a more "winning" feeling.

## Verification Plan

### Automated Tests
- None available for this frontend-specific task.

### Manual Verification
- **Icon:** Visually verify the header title and icon are aligned.
- **Labels:**
    1.  Open "New Task" dialog.
    2.  Select a label.
    3.  Create the task.
    4.  Verify the label appears on the card in the board.
    5.  Refresh the page (to verify persistence/saving).
    6.  Verify the label still appears.
- **Gamification:**
    1.  Complete a task (drag to Done or click checkbox if available). Verify confetti explosion and sound.
    2.  Check off a step in a task. Verify small confetti burst and sound.
