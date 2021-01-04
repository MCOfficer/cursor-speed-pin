# CursorSpeedPin

Sits in your tray, keeps your cursor speed at bay :)

## Motivation

Whenever I enter a game of League of Legends, the game resets my Windows cursor speed to an absurdly low value. I'd assume there are also some other buggy programs that do this. Born out of this frustration was this nifty program.

## Install

Head over to the [latest release](https://github.com/MCOfficer/cursor-speed-pin/releases/latest) and download the executable. There's no installation needed, just place it somewhere and run it.

## How to use

I hope this is fairly self-explanatory, so here's the short version.

Once you run CursorSpeedPin, it will automatically detect your current cursor speed as set in the control panel. It will give you a nice desktop notification that it is now watching for any changes to that speed, and it will indicate its presence with an icon in the system tray.

Should you want to change your speed (or disable it for some other reason), you can double-click the icon. You will receive a notification, and the icon will also change.

If CursorSpeedPin detects a change to your cursor speed, it will reset that change and send you a short desktop notification.

I would recommend creating an AutoStart entry for CursorSpeedPin, so you don't have to remember to start it. Repeat that to enable it again.

## Help it not work

Things aren't working as expected? Please [open an issue](https://github.com/MCOfficer/cursor-speed-pin/issues/new) with a detailed explanation of what you did, what you expected to happen and what happened instead. CursorSpeedPin also creates a logfile called `cursor-speed-pin.log` in the folder you placed it in - attach that to the issue, it makes my life so much easier!

If you don't even have a logfile, chances are that you don't have write access to the folder CursorSpeedPin is in; move it to a different folder.

Please be patient with issues, I'm doing this in my free time and it's only one of way too many projects I have going :)

## Acknowledgements

Parts of the code for desktop notifications have been taken from [AmaranthineCodices/win32_notification](https://github.com/AmaranthineCodices/win32_notification), under the same license.

The tray icons are based on [this image](https://pixabay.com/de/vectors/cursor-pfeil-zeiger-computer-maus-23229/) by [Clker-Free-Vector-Images](https://pixabay.com/de/users/clker-free-vector-images-3736/).
