# Help

ShelterSave-Studio edits Fallout Shelter save files. The program works offline and sends nothing anywhere.

## Before you start

- **Close the game.** While the game is running it keeps the vault in memory and writes it over your file when it saves. When the game is open, the toolbar shows "The game is running".
- **Backups.** Before the first write to a file the program puts a copy of it into `%LOCALAPPDATA%\ShelterSave-Studio\backups`. To bring a file back, use "Restore from backup" in the Save section.
- **Cloud saves.** If the game syncs saves through the cloud, it may replace the edited file with the cloud copy. Turn syncing off before editing or check which copy the game loads.

## Where the save file is

### Windows: Steam and Bethesda.net

The start screen lists the vaults from `%LOCALAPPDATA%\FalloutShelter` on its own. The game has only three save slots, so the files are always named `Vault1.sav`, `Vault2.sav` and `Vault3.sav`. The game ignores files with other names. `.sav.bkp` files are backups made by the game itself.

### Linux: Steam with Proton

`~/.steam/steam/steamapps/compatdata/588430/pfx/drive_c/users/steamuser/AppData/Local/FalloutShelter`

### Android

The `Android/data/com.bethsoft.falloutshelter/files` folder in the phone's internal storage. Copy the file to the computer over USB, edit it and put it back under the same name.

## How to edit a save

1. Open a vault from the start screen, with "Open" (Ctrl+O), or drop the file onto the window.
2. Change what you need. Every field has a description: hover over the question mark next to its name.
3. Press "Save" (Ctrl+S). The program shows all changes, checks the file for errors that could keep the game from loading it, and writes it in place. "Save as…" (Ctrl+Shift+S) writes the changes to another file.
4. Start the game.

## When the game changes the file

The program watches the open file. If the game or another program writes over it, a bar at the top offers to load the file again. Unsaved edits can be carried over to the new version of the file or dropped.

## Normal and expert mode

In normal mode values stay within the limits the game allows and technical fields are hidden. Expert mode (in Settings) opens every field of the file and the JSON tree. Values outside the game's limits are allowed there but can break the vault, so the program warns about them.

## Undo and crash recovery

Every edit can be undone (Ctrl+Z) and redone (Ctrl+Y). The clock button opens the journal of all steps. If the program closed unexpectedly, it offers to bring back the unsaved edits on the next start.

## Keys

- Ctrl+O – open a file, Ctrl+S – save, Ctrl+Shift+S – save as.
- Ctrl+Z – undo, Ctrl+Y or Ctrl+Shift+Z – redo.
- Ctrl+F – search fields, dwellers and items.
- Alt+1, Alt+2, Alt+3, Alt+4 – the Save, Vault, Dwellers and Other sections.
- On the vault map: Delete removes the selected room, Ctrl+D copies it, Alt and the arrow keys move it, Enter places it, Escape cancels.

## Game pictures

The program takes the pictures of items, rooms and dwellers from the installed PC version of the game (Steam or Bethesda.net) and finds it on its own. If the game is installed elsewhere, choose its folder in Settings ("Game folder (pictures)"). Game files are only read and never copied.

## Questions

**Is it safe for my save?** The program keeps everything in the file without loss and, before writing, checks the links and counters whose errors can make the game hang. A copy of the original file stays in the backup folder.

**Will I get banned?** Fallout Shelter is a single-player game. Editing your own save is your decision, and you are responsible for following the terms under which you got the game.

**My changes are gone after starting the game.** The game was running during editing and saved its own state over the file, or cloud sync brought back an older copy.

**The file does not open.** Make sure it is a Fallout Shelter save (`.sav` or the game's backup `.sav.bkp`) and not a file from another game. The error message tells at which step reading failed.
