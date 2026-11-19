# Desktop version

ShelterSave-Studio for Windows, Linux and macOS is the same editor as on this website, as a program for your computer.

## Why use it

- **Works offline.** The program makes no network requests at all: no telemetry, no update checks, no downloads.
- **Finds your saves** in the Steam and Bethesda.net game folders and lists your vaults.
- **Writes the file in place.** It makes a backup before the first write, and writes atomically, so an interrupted write never damages the file.
- **Shows game pictures:** items, rooms, themes and dwellers. The program reads them from the game installed on your computer; it contains no pictures itself.
- **Notices when the game has saved the file** and offers to load the new version.
- **Recovers your edits** after a crash.

## Download

Builds for all systems are published on the project's release page: [github.com/Mahiron-hq/ShelterSave-Studio/releases/latest](https://github.com/Mahiron-hq/ShelterSave-Studio/releases/latest).

- **Windows 10 and 11 (x64):** installer, or a portable file that runs without installation.
- **Linux (x64):** AppImage, .deb and .rpm packages.
- **macOS:** separate builds for Intel and Apple Silicon, as a .zip archive and a .dmg image.

Every file comes with a SHA-256 checksum.

## System warning on first start

The builds are not signed with a developer certificate, so the system may warn about an unknown publisher.

- **Windows:** in the SmartScreen window press "More info", then "Run anyway".
- **macOS:** open the program with right click → "Open", then confirm.

You can check that a file was not altered with its SHA-256 checksum from the release page.
