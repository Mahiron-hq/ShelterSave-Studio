# Help

ShelterSave-Studio edits Fallout Shelter save files. On this website the file is processed only in your browser and is never sent anywhere.

## Before you start

- **Close the game.** While the game runs it keeps its own copy of the vault in memory and will write it over your file when it saves.
- **Keep a copy of the original file.** The editor does not change the file on disk: you download a new file and put it into the game yourself. Keep the old file next to it so you can go back.
- **Cloud saves.** If the game syncs saves through the cloud, it may replace an edited file with the cloud copy. Turn syncing off before editing or check which copy the game loads.

## Where the save file is

### Windows: Steam and Bethesda.net

Press Win+R, paste `%LOCALAPPDATA%\FalloutShelter` and press Enter. The game has only three save slots, so the files are always named `Vault1.sav`, `Vault2.sav` and `Vault3.sav`; the game ignores files with any other name. `.sav.bkp` files are backups the game makes itself.

### Linux: Steam with Proton

`~/.steam/steam/steamapps/compatdata/588430/pfx/drive_c/users/steamuser/AppData/Local/FalloutShelter`

### Android

The folder `Android/data/com.bethsoft.falloutshelter/files` in the phone storage. On Android 11 and newer most file managers do not show it: connect the phone to a computer over USB or use a file manager that has access to this folder.

## How to edit a save

1. Open the [editor](/en/) and drop the file onto the page or press "Choose file".
2. Change what you need. Every item has an explanation: point at the question mark next to its name.
3. Press "Download" in the top bar. The editor lists all changes and checks the file for problems that could stop the game from loading it.
4. Put the downloaded file into the save folder with the same name as the original, replacing the old file.
5. Start the game.

## Normal and expert mode

In normal mode values stay within what the game accepts and technical fields are hidden. Expert mode (in Settings) opens every field of the file and the JSON tree. Values outside game limits are allowed there but can break the vault, so the editor warns about them.

## Undo and unfinished sessions

Every edit can be undone (Ctrl+Z) and redone (Ctrl+Y). The history of all steps opens with the clock button. If "Restore unfinished session" is on, the open file and its history are kept in your browser; after a page reload or a browser crash the website offers to continue.

## Game pictures

The website does not show or store pictures from the game: the images belong to the game's rights holders. If you have the PC version of the game installed, you can load its file `FalloutShelter_Data\data.unity3d` with the "Load the game file" button. In Steam, right-click the game in the library and choose Manage → Browse local files; the usual path is `C:\Program Files (x86)\Steam\steamapps\common\Fallout Shelter\FalloutShelter_Data\data.unity3d`. The Android version has no such file. Your browser reads the pictures of items, rooms and dwellers from that file right on your device; the file is not sent or saved anywhere, and the pictures are gone when you close the tab. The [desktop version](/en/desktop/) finds the game by itself.

## Common questions

**Is it safe for my save?** The editor keeps everything in the file without loss and, before you download, checks the links and counters whose errors can make the game hang. Still, keep a copy of the original file.

**Will I get banned?** Fallout Shelter is a single-player game. Editing your own save is your decision, and you are responsible for following the terms under which you obtained the game.

**My changes are gone after starting the game.** The game was running while you edited and saved its own state over the file, or cloud syncing brought the old copy back.

**The file does not open.** Make sure it is a Fallout Shelter save (`.sav` or the game backup `.sav.bkp`), not a file of another game. The error message tells at which step the file could not be read.

**Does the website send my file anywhere?** No. Decryption, editing and encryption happen in your browser. See the [privacy policy](/en/privacy/).
