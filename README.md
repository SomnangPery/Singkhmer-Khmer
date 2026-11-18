# Bunny Keyboard

Type Khmer the way you already chat, in Latin letters, and get Khmer script.

```
nh jg tv sala rean   →   ខ្ញុំចង់ទៅសាលារៀន
```

A keyboard for Windows 10 and 11, and for the Mac. Suggestions show while you
type, and it works in every app: browsers, Word, Telegram, Messenger, Notepad,
Pages. Everything runs on your computer, with no internet. Nothing is sent
anywhere unless you choose to [share words](#help-make-it-better).

Install it on [Windows](#on-windows) or on a [Mac](#on-a-mac).

> **Early version.** It works, but some words will come out wrong. Please
> [tell us](#help-make-it-better) when they do.

## Install

### On Windows

Open **Command Prompt** or **PowerShell** (press Windows, type `cmd`, press
Enter), paste this line and press Enter:

```
powershell -c "irm https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/setup.ps1 | iex"
```

Windows asks for permission to change your PC: click **Yes**. That is all.
You do not need to download the project or install anything else.

Then press **Windows + Space**. Under Khmer there is one keyboard,
**Singkhmer (Bunny Keyboard)**: pick it.

```
ខ្មែរ  Khmer
     Singkhmer (Bunny Keyboard)
```

Apps that were already open may need a restart before they offer it.

If your PC already had Khmer, its Windows Khmer keyboard stays in the list
too, as "Khmer / Khmer". To keep only Bunny Keyboard, open **Settings → Time &
language → Language & region**, click **⋯** next to Khmer, **Language
options**, and remove the keyboard named **Khmer**.

Needs 64-bit Windows 10 or 11. To take it off again, see
[Uninstall](#uninstall).

### On a Mac

> **New.** The Mac version is newer than the Windows one and has had less
> use. Please [tell us](#help-make-it-better) how it goes.

**1. Install it.** Open **Terminal**: press **Command + Space**, type
`terminal` and press **Return**. Paste this line into Terminal and press
**Return**:

```
curl -fsSL https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/install-mac.sh | bash
```

It takes a few seconds and ends with "Bunny Keyboard is installed." No
password is needed: it installs for your user only. You do not need to
download the project or install anything else.

**2. Pick it.** Click the input menu at the top right of the screen (a flag
or a letter, next to the clock), or press **Control + Space**, and pick
**Singkhmer (Bunny Keyboard)**.

```
ខ្មែរ  Singkhmer (Bunny Keyboard)
🇺🇸   ABC
```

**3. Only if it is not in that menu:** open **System Settings → Keyboard**,
and under **Text Input** click **Edit…**. Click **+** at the bottom left,
choose **Khmer**, then **Singkhmer (Bunny Keyboard)**, and click **Add**. If
it is not in that list either, log out and in again, and look once more.

Now type in any app: `nh jg tv`, then Space twice, gives ខ្ញុំចង់ទៅ.

**Switching back** to your usual keyboard: press **Control + Space** again,
or pick it in the input menu. (On some Macs the key is **Fn** or **🌐**
instead; it is set in System Settings → Keyboard.)

Good to know on a Mac:

- Use the Terminal line above. A zip downloaded in Safari and opened by hand
  is stopped by macOS, because the keyboard is not yet signed by Apple.
- Password fields always type plain letters: macOS uses its own keyboard
  there.
- Needs macOS 12 (Monterey) or newer, on an Apple silicon or Intel Mac. To
  take it off again, see [Uninstall](#uninstall).

## How to type

Type each word the way you would spell it in a chat, with a space after it.
Each word turns into Khmer as you go. A second space sends the whole phrase.
This works the same on Windows and on the Mac.

| You type | You get |
| --- | --- |
| `nh` | ខ្ញុំ |
| `jg` | ចង់ |
| `tv` | ទៅ |
| `nh jg tv` | ខ្ញុំចង់ទៅ |
| `nh jg tv sala rean` | ខ្ញុំចង់ទៅសាលារៀន |

Short forms and longer spellings both work: `nh`, `knhom` and `khnhom` all
give ខ្ញុំ. English words such as `facebook` or `ok` stay in English.

While you type, a small popup shows the three best guesses:

| Key | What it does |
| --- | --- |
| Space | Ends the word and turns it into Khmer |
| Space again | Sends the phrase, then a space |
| Enter | Sends the highlighted suggestion |
| Down / Up arrow, or Tab / Shift+Tab | Highlights another suggestion |
| A click on a suggestion | Sends that one |
| Backspace | Undoes one key, including a space |
| Esc | Throws the phrase away |
| A shortcut (Ctrl, or Command on a Mac) | Sends the phrase, then does the shortcut |
| `.` `,` `?` a digit | Sends the phrase, then that character |

Khmer words are joined with no spaces between them, the way Khmer is written.

## Add your own words

If the keyboard does not know a word, gives a wrong one, or you spell it your
own way, add it:

- click **+ Add my word** at the bottom of the popup (it fills in the word you
  just typed),
- click **Wrong word?** next to it (it also shows what you typed and what the
  keyboard gave), or
- on Windows, open **Bunny Keyboard - My words** from the Start menu;
  on a Mac, click the input menu at the top right and **My words…**.

Write the spelling and the Khmer word, then click **Add word**. From the next
key press, typing that spelling gives your word first, in every app. On a
Mac, the window also shows the keyboard's guesses for the spelling: click one
to use it.

Your words are kept in a plain text file you can also edit by hand, one per
line. On Windows it is `%APPDATA%\Bunny Keyboard\my-words.txt` (open it in
Notepad); on a Mac it is `~/Library/Application Support/Bunny Keyboard/my-words.txt`
(the **Open the file** button in My words opens it in TextEdit):

```
bong = បង
oun = អូន
```

They stay on your computer when the keyboard is updated or removed. To use
the same words on both, copy the file from one to the other.

## Update

Run the install line again ([Windows](#on-windows), [Mac](#on-a-mac)). Your
own words are kept.

## Uninstall

### On Windows

1. Open **Settings → Apps → Installed apps**.
2. Find **Bunny Keyboard**, click **⋯** and then **Uninstall**.
3. Click **Yes** when Windows asks for permission. A message says when it is
   done.

**Bunny Keyboard is not in Installed apps?** You installed it before it was
added there. Run the [install line](#install) once to update it, and then it
will be there to uninstall.

If Khmer was in your language list only for Bunny Keyboard, it is taken off
too, so your PC is back to how it was.

### On a Mac

Paste this line in **Terminal** and press Return:

```
curl -fsSL https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/install-mac.sh | bash -s -- --remove
```

Your words stay in `~/Library/Application Support/Bunny Keyboard`; delete that
folder to remove them too.

## Good to know

- **The first word takes a moment.** The first time you use Singkhmer in an
  app, it reads its dictionary of about 120,000 words, which takes about a
  second.
- **"Access is denied" or an antivirus warning when you install?** The
  keyboard is new and not signed, and a line that downloads and runs a script
  looks suspicious to antivirus programs, Microsoft Defender included. Check
  that you copied the install line exactly as it is above, with nothing added.
- **An app closes while you type?** Press Windows + Space to switch to another
  keyboard and uninstall Bunny Keyboard in Settings, then
  [tell us](#help-make-it-better) which app it was.
- **32-bit apps** (a few old programs) do not offer the keyboard yet.
- **On a Mac, the suggestions show in the wrong place** in a few apps that do
  not tell the keyboard where the text is. They still work: click one or use
  the arrow keys.

## Help make it better

The words you add, and the ones you fix, can make Bunny Keyboard better for
everyone. To send them, open My words (see [Add your own words](#add-your-own-words))
and the **Share** tab:

1. It lists your own words, and the words you corrected by picking the
   second or third suggestion instead of the first.
2. Untick any you would rather keep to yourself, and click **Send**.
3. A form opens in your browser with the words filled in. Check them and click
   **Submit**. You do not need an account.

You can also tick **Also send it to Bunny Keyboard** when you add a word, or
when you use **Wrong word?**.

**What is kept, and what is sent:**

- When you pick a suggestion other than the first, the keyboard notes just
  that word and its spelling (for example `tv = ធ្វើ`) in `learned.txt`, next
  to `my-words.txt`. It notes only words you picked from the suggestions,
  never the rest of what you write.
- Nothing leaves your computer until you click **Submit** on the form.
- Words you send are given to the public domain (CC0), so anyone may use them,
  in Bunny Keyboard and elsewhere. A word goes into the dictionary once
  several people have sent the same one.

You can also report a wrong word or any other problem on
[GitHub](https://github.com/SomnangPery/Singkhmer-Khmer/issues).

---

For developers (building it, the Android app, the Mac version, how the engine works, the
data): [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). The word lists come from
open datasets listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
