# CatDo for Linux

A calm personal task manager from WorkerCat.

The `.tar.gz` archive contains the Linux x86-64 desktop app. CI builds it on Ubuntu 24.04
(glibc 2.39); use a desktop Linux distribution with glibc 2.39 or newer, such as
current Fedora. ARM and Windows binaries are not included.

## AppImage

Download the `.AppImage` from GitHub Releases, make it executable, and run it.
Substitute the downloaded filename below:

```sh
chmod +x CatDo.AppImage
./CatDo.AppImage
```

The AppImage includes the app icon and desktop metadata for launcher integration
tools. If FUSE is unavailable, run `./CatDo.AppImage --appimage-extract-and-run`.
AppImage bundles native libraries where appropriate; it still requires host
graphics drivers, Fontconfig, a graphical session, and glibc 2.39 or newer.

## Archive

Extract the archive and run `./catdo` from this directory. Rust is not required.
The app needs your distribution's D-Bus, Fontconfig, XCB, xkbcommon (including
its X11 library), and graphics drivers. For sync, an unlocked Secret Service
keyring such as GNOME Keyring must be available in your desktop session.

## Install or update

From the extracted directory:

```sh
bash scripts/install-desktop.sh --prebuilt
```

The installer uses Bash, Python 3, and standard Linux utilities. It installs the
binary to `~/.local/bin/catdo` and adds CatDo to your application launcher.
Installing a newer archive replaces the binary and launcher without changing
your tasks. `XDG_BIN_HOME` and `XDG_DATA_HOME` overrides are respected.

Tasks are stored in `~/.local/share/catdo/catdo.sqlite3` by default. Desktop use
needs no account; choose **Sign in to sync** to connect your WorkerCat account.

If the native Wayland graphics path has trouble, try:

```sh
env -u WAYLAND_DISPLAY ./catdo
```

## Verify the download

Keep the `.tar.gz` and its `.sha256` file together, then run:

```sh
sha256sum --check catdo-*.tar.gz.sha256
```

[Website](https://catdo.workercat.com) · [Source and documentation](https://github.com/workercatstudios/catdo)
