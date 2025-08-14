<div align="center">

# Axec

Lightweight, fast AppImage manager built with Tauri + React.

## (Not fully tested yet !)

</div>

## ✨ Features

- Add AppImages via file picker (copies; does not move)
- Auto integration into the menu (desktop entries + extracted icons)
- List, search, launch, and remove AppImages
- Linux-first, tiny footprint

## 📁 Default paths

- Storage: `~/.local/share/axec/appimages`
- Desktop entries: `~/.local/share/applications/axec-<id>.desktop`

Icons are extracted via `--appimage-extract` when possible.

## 🚀 Quick start (Dev)

Prereqs: Rust (stable), Node.js (>=18), npm

```sh
npm install
npm run tauri dev
```

## 🛠️ Build (Release)

```sh
npm run tauri build
# Run the binary
./src-tauri/target/release/axec
```

## 🧪 How to use

1) Click “Add AppImage” and choose a `.appimage`/`.AppImage` file.
2) Axec copies it to the storage folder and makes it executable.
3) It extracts an icon when available and creates a desktop entry.
4) You can launch or remove apps from the list; search helps you filter.

If the menu entry doesn’t appear immediately, it usually shows up in a few seconds.

## 📦 Flatpak

Flatpak manifest (Flathub-ready baseline): `packaging/com.praneeshrv.Axec.json` (org.freedesktop 24.08).

Local build and run:
```sh
flatpak install -y flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08 \
	org.freedesktop.Sdk.Extension.rust-stable org.freedesktop.Sdk.Extension.node20
cd packaging
flatpak-builder --user --install --force-clean ../build-dir com.praneeshrv.Axec.json
cd -
flatpak run com.praneeshrv.Axec
```

Self-host a repo (optional):
```sh
cd packaging
flatpak-builder --user --repo=../repo --force-clean ../build-dir com.praneeshrv.Axec.json
flatpak build-update-repo ../repo
cd -
flatpak build-update-repo repo
```

Notes:
- Inside a Flatpak sandbox, Axec stores data under XDG data dir and skips creating host `.desktop` files (policy-friendly).
- For Flathub submission, pin immutable sources and complete AppStream (homepage, screenshots, releases).

## 🧰 Tech stack

- Tauri v2 (Rust backend)
- React + Vite + Tailwind CSS (TypeScript)

## 🔒 Security & permissions

- Axec only copies files you pick and writes into your user data directories.
- No network access or elevated privileges required.

## 🗺️ Roadmap

- AppImage update checks (optional)
- Drag-and-drop and multi-select
- More robust icon extraction and metadata parsing

## 🤝 Contributing

PRs and issues are welcome. Please keep changes small and focused.

## 📄 License

MIT — see `LICENSE`.
