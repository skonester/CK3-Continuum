# Prototype architecture

The renderer is Svelte 5 and TypeScript, built by Vite. Electron main is CommonJS so its sandboxed preload can expose a small explicit bridge without a second bundling pipeline. The Rust package is independent and usable from Node through CJS or ESM.

## Data flow

1. The renderer requests a campaign or reference slot.
2. Main validates the top-level sender frame and slot, then opens the native file picker.
3. Main checks the selected regular file and launches an Electron utility process.
4. That process loads the napi-rs addon and streams the save using Jomini.
5. Main retains the successful report in memory and sends the bounded summary to Svelte.
6. Export writes main's retained report to a new JSON path selected by a save dialog.

Renderer requests cannot carry arbitrary read/write paths or report contents. A failed or cancelled inspection leaves the previous successful result intact. A five-minute timeout and cancellation terminate the utility process. Only one dialog/inspection/export job runs at a time. The renderer and main never receive decompressed gamestate bytes.

## Desktop boundary

Context isolation, renderer sandboxing, disabled Node integration, a content security policy, blocked navigation/new windows, and denied permissions constrain the local renderer. The development server is restricted to 127.0.0.1:5173. Packaged builds ignore the development URL environment flag and load bundled assets.

These choices follow Electron's [context isolation](https://www.electronjs.org/docs/latest/tutorial/context-isolation) and [sandbox](https://www.electronjs.org/docs/latest/tutorial/sandbox) guidance. The native addon is packaged as an external resource using [Electron Builder application contents](https://www.electron.build/docs/contents/) configuration.

## Report preservation

Report output is JSON only. It is written and synced into a temporary sibling, then hard-linked to the requested destination; an existing destination is rejected. The temporary name is removed afterward. This prevents overwriting an existing file, including a save hard-linked under another name. The current implementation requires a filesystem supporting hard links, such as NTFS; unsupported destinations fail without replacing existing files.

Reports omit absolute input paths but include filenames and campaign metadata. Nothing is automatically uploaded. Results live in memory until the user chooses export; Electron itself maintains its normal local runtime profile.

## Deliberate scope

A reference save is structural evidence. Counts vary with campaign age, mods, DLC, and world state. The UI does not infer universal conversion rules, detect a mod playset, or assign a compatibility score from those counts.

The only configured research target is 1.19.0.6, based on the supplied research pair. It is not a claim that 1.19.0.6 is the latest released engine. No internet lookup runs in the app. Future profiles must identify exact engine/content definitions.

## Experimental conversion (v0.2)

Main retains dialog-selected source paths privately. The renderer supplies only a validated mode. Main chooses output via a save dialog and creates a temporary sibling directory. The worker produces a staged save, then main commits save and JSON journal through no-overwrite hard links. The report link is rolled back if publishing the save fails. This is a two-file transaction with rollback, not a single atomic rename; a power loss between links can leave an orphan report.

Cancellation kills the worker and main removes its owned staging directory, verifying containment under the selected parent. An abrupt app/machine termination can leave a staging directory; it is not completed output. Cancellation is no longer effective after the worker finishes and publication begins.

The package embeds the supplied ICO and passes it to BrowserWindow. Executable resource editing is enabled; code signing remains disabled.
