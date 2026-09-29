# CatDo for Windows

A calm personal task manager from WorkerCat.

CatDo for Windows is a single portable program for 64-bit Windows 10 or 11 on
x86-64. It needs no installer, administrator rights, or Rust. Keep `catdo.exe`
in a folder you own, such as `%LOCALAPPDATA%\Programs\CatDo`, and run it from
there. You can pin it to Start or the taskbar.

The program is not yet code-signed. Windows SmartScreen may warn when you first
run it; choose **More info**, then **Run anyway**. Compare the download with its
`.sha256` file first if you want to confirm it is the published build.

Tasks are stored in `%LOCALAPPDATA%\workercat\catdo\data\catdo.sqlite3`. Desktop
use needs no account; choose **Sign in to sync** to connect your WorkerCat
account. Sign-in is kept in Windows Credential Manager, not the task database.

## Updates, the notification area, and reminders

CatDo checks GitHub Releases at startup and every six hours. The small button at
the bottom of the sidebar offers a download when a newer stable version is
available. Click once to download and verify it, then click again to install and
restart. Updates replace `catdo.exe` in place and keep `catdo.exe.previous` for
rollback; your tasks are not changed.

Closing the window keeps CatDo in the notification area, with sync and
reminders still running. Click its icon, choose **Open CatDo**, or launch CatDo
again to reopen it. Choose **Quit CatDo** from the icon's menu or press
**Ctrl+Q** to exit. Windows may place the icon in the overflow menu; drag it to
the taskbar to keep it visible.

Reminders appear as Windows notifications. When Do Not Disturb is on, they go
straight to the notification center.

## Verify the download

In PowerShell, from the download folder:

```powershell
(Get-FileHash .\catdo-<version>-windows-x86_64.exe).Hash
Get-Content .\catdo-<version>-windows-x86_64.exe.sha256
```

The two hashes should match (the checksum file uses lowercase).

[Website](https://catdo.workercat.com) · [Source and documentation](https://github.com/workercatstudios/catdo)
