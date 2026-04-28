# pov — Architecture & Concepts

Personal reference. Explains the role of every folder and file, and the concepts behind the stack choices.

---

## Big picture

The app has three distinct layers that each live in a different technology:

```
┌─────────────────────────────────────────────────────┐
│  macOS (.app)                                       │
│                                                     │
│  ┌─────────────────────┐   ┌──────────────────────┐ │
│  │   Tauri (Rust)      │   │  Python FastAPI      │ │
│  │   the shell         │   │  the brain           │ │
│  │                     │   │                      │ │
│  │  ┌───────────────┐  │   │  - reads/writes      │ │
│  │  │  WebView      │  │   │    TODO.md files     │ │
│  │  │  (React app)  │◄─┼───┼─ - manages SQLite    │ │
│  │  │  the face     │  │   │  - watches files     │ │
│  │  └───────────────┘  │   │  - runs git          │ │
│  └─────────────────────┘   └──────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

- **Tauri** is the native macOS wrapper. It creates the window, the tray icon, handles file dialogs. Think of it as the "OS glue".
- **React** (inside Tauri's WebView) is the UI. A WebView is essentially an embedded browser — your React app runs in it exactly like a website, except it's bundled inside the `.app`.
- **Python FastAPI** is the backend server, running silently in the background as a subprocess of Tauri. It exposes a local HTTP API that the React app calls.

These three parts talk to each other:
- React → Python: HTTP requests to `http://127.0.0.1:<port>`
- React → Tauri: special `invoke()` calls for native features (file browser, open terminal, etc.)
- Tauri → Python: starts and stops the Python process

---

## Root-level files

### `package.json`
The Node.js project manifest. Declares the frontend dependencies (React, Tailwind, Shadcn…) and the scripts you run (`npm run dev`, `npm run build`, `npm run tauri dev`). Think of it as the Python `pyproject.toml` of the frontend world.

### `package-lock.json`
Auto-generated. Records the exact version of every installed package (including dependencies of dependencies). You never edit this. Its purpose is reproducibility — `npm install` on any machine produces the exact same result.

### `node_modules/`
Where npm puts all downloaded packages. Never commit this. Never edit anything in here.

### `tsconfig.json` and `tsconfig.node.json`
TypeScript configuration. TypeScript is JavaScript with type annotations — it catches bugs at write time rather than at runtime. These files tell the TypeScript compiler what rules to enforce and how to resolve imports.

- `tsconfig.json` — rules for the React app code (`src/`)
- `tsconfig.node.json` — rules for the build tooling code (`vite.config.ts`)

The split exists because the React app runs in a browser environment, while the build config runs in Node.js — different globals available.

### `vite.config.ts`
Configuration for **Vite**, the build tool and development server. Vite takes your TypeScript + React source files and:
- In dev mode: serves them instantly with hot reload (change a file, the browser updates in ~50ms)
- In build mode: bundles and minifies everything into `dist/` for production

The file also configures the `@` alias — anywhere in the code you write `import X from "@/components/..."` and it resolves to `src/components/...`. Cleaner than relative paths.

### `components.json`
Configuration for **Shadcn**. Tells it where to put components, what Tailwind version to use, what style to apply. You don't edit this often.

### `index.html`
The single HTML file that bootstraps the entire React app. It contains nothing except a `<div id="root">` and a `<script>` that loads the compiled React bundle. React then takes over and renders everything inside that div.

### `.gitignore`
Files and folders git should not track: `node_modules/` (huge, reproducible), `dist/` (generated), `target/` (Rust build artifacts).

---

## `src/` — The React frontend

This is the UI layer. Everything here is TypeScript + React, compiled by Vite into a bundle that runs in Tauri's WebView.

### `src/main.tsx`
The entry point. This is where React starts. It finds the `<div id="root">` in `index.html` and renders the top-level `<App />` component into it. You rarely touch this file.

### `src/App.tsx`
Currently the root component with the default Tauri template. This will become the main layout of the app (project list, routing between views, etc.).

### `src/index.css`
The global stylesheet. The first line `@import "tailwindcss"` loads the entire Tailwind CSS framework. Shadcn then appends its design tokens (colors, radius, fonts) below.

### `src/App.css`
Styles specific to the default template. Will be cleaned up or removed when we build the real UI.

### `src/components/`
Where all UI components live. A component is a self-contained piece of UI — a button, a card, a sidebar. Each component is a function that returns HTML-like syntax (JSX).

#### `src/components/ui/`
Auto-generated Shadcn components. Shadcn is not a traditional component library (like Bootstrap) — it copies the source code of each component directly into your project. You own the code. This means you can read and modify every component freely, unlike a black-box library.

`button.tsx` was added automatically when we ran `shadcn init`. We'll add more (dialog, card, etc.) as needed with `shadcn add <component>`.

### `src/lib/utils.ts`
A single utility function `cn()` added by Shadcn. It merges Tailwind class names intelligently, handling conflicts. Example: `cn("p-4 p-2")` correctly resolves to `"p-2"` instead of both being applied.

### `src/assets/`
Static files (images, SVGs) used directly in the React code.

### `src/vite-env.d.ts`
TypeScript type declarations for Vite-specific features (like `import.meta.env`). Auto-generated, don't touch.

---

## `src-tauri/` — The Tauri shell (Rust)

This is the native macOS layer. It wraps the React app, creates the window, handles OS-level features.

### `src-tauri/tauri.conf.json`
The main Tauri configuration. Defines:
- The app window (size, decorations, visibility on start)
- Bundle settings (icon files, app identifier)
- macOS-specific options (`macOSPrivateApi: true` to allow setting the activation policy)

### `src-tauri/Cargo.toml`
The Rust project manifest — equivalent of `package.json` for Rust. Declares Rust dependencies ("crates") and their features. Cargo is the Rust package manager.

### `src-tauri/Cargo.lock`
Same role as `package-lock.json` — locks exact dependency versions. Unlike Node projects (where you often gitignore the lock), you always commit `Cargo.lock` for applications.

### `src-tauri/src/lib.rs`
The core Rust code of the app. Currently handles:
- Creating the tray icon and its menu
- Left-click → toggle window show/hide
- Right-click → Show / Quit
- Setting `ActivationPolicy::Accessory` so the app has no Dock icon
- Hiding the window when it loses focus (click-away)

`lib.rs` contains the logic as a library. `main.rs` just calls `run()` from `lib.rs`. The split is a Tauri convention for technical reasons (cross-platform compilation).

### `src-tauri/build.rs`
Runs at compile time before the app is built. Calls `tauri_build::build()` which generates some glue code from `tauri.conf.json`. You never edit this.

### `src-tauri/capabilities/default.json`
Tauri v2's permission system. Explicitly declares what APIs the frontend JavaScript is allowed to call. By default it allows a minimal safe set. When we add features (file system access, shell commands), we'll add permissions here.

### `src-tauri/icons/`
App icon in every size required by macOS, Windows, and Linux. The `.icns` file is the macOS format (multi-resolution bundle). The `.ico` is Windows. Tauri generates all sizes from a single source icon.

### `src-tauri/gen/schemas/`
Auto-generated JSON schemas that Tauri uses internally for config validation. Never edit.

### `src-tauri/target/`
Rust build artifacts. Very large (several GB). Gitignored. Regenerated on every build.

---

## `public/`
Static files that Vite copies verbatim into `dist/` without processing. Useful for assets that need a stable URL. Currently just holds the default Tauri and Vite SVG logos — will be cleaned up.

---

## `dist/`
The compiled frontend output produced by `npm run build`. Tauri bundles this folder into the `.app`. Gitignored — it's generated, not authored.

---

## Concepts cheat sheet

| Term | What it means |
|---|---|
| **Component** | A reusable piece of UI, written as a TypeScript function that returns JSX |
| **JSX** | HTML-like syntax inside TypeScript files. `<Button onClick={...}>` compiles to regular JS |
| **Props** | Parameters passed to a component, like function arguments |
| **Hook** | A React function (starting with `use`) that adds behaviour to a component — `useState` for local state, `useEffect` for side effects |
| **TypeScript** | JavaScript + types. Types describe what shape data has, caught at compile time |
| **Tailwind** | A CSS framework where you style things with utility classes directly on elements: `className="flex gap-4 text-sm font-bold"` |
| **Shadcn** | A set of pre-built, accessible UI components (button, dialog, card…) copied into your project as source code |
| **Vite** | The dev server and bundler. In dev mode it's instant. In prod it compiles everything into optimised static files |
| **WebView** | An embedded browser inside a native app. Tauri renders your React app in one |
| **Sidecar** | A subprocess launched by Tauri alongside the app. Our Python backend will be a sidecar |
| **Crate** | A Rust library/package (equivalent of an npm package) |
| **Cargo** | The Rust package manager (equivalent of npm) |
| **FastAPI** | A Python web framework for building HTTP APIs. We'll use it to expose endpoints that the React app calls |
| **SQLite** | A file-based SQL database. The entire database is a single `.db` file, no server required |
| **watchdog** | A Python library that watches the filesystem for changes and fires callbacks |
