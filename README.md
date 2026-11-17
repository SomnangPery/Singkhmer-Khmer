# Bunny Keyboard

Type Khmer the way you already chat, in Latin letters, and get Khmer script.

```
nh jg tv sala rean   →   ខ្ញុំចង់ទៅសាលារៀន
```

A keyboard for Windows 10 and 11. Suggestions show while you type, and it
works in every app: browsers, Word, Telegram, Messenger, Notepad. Everything
runs on your PC, with no internet. Nothing is sent anywhere unless you choose
to [share words](#help-make-it-better).

> **Early version.** It works, but some words will come out wrong. Please
> [tell us](#help-make-it-better) when they do.

## Install

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

## How to type

Type each word the way you would spell it in a chat, with a space after it.
Each word turns into Khmer as you go. A second space sends the whole phrase.

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
| `.` `,` `?` a digit | Sends the phrase, then that character |

Khmer words are joined with no spaces between them, the way Khmer is written.

## Add your own words

If the keyboard does not know a word, gives a wrong one, or you spell it your
own way, add it:

- click **+ Add my word** at the bottom of the popup (it fills in the word you
  just typed),
- click **Wrong word?** next to it (it also shows what you typed and what the
  keyboard gave), or
- open **Bunny Keyboard - My words** from the Start menu.

Write the spelling and the Khmer word, then click **Add word**. From the next
key press, typing that spelling gives your word first, in every app.

Your words are kept in a plain text file you can also edit in Notepad,
`%APPDATA%\Bunny Keyboard\my-words.txt`, one per line:

```
bong = បង
oun = អូន
```

They stay on your PC when the keyboard is updated or removed.

## Update

Run the [install line](#install) again. Your own words are kept.

## Uninstall

1. Open **Settings → Apps → Installed apps**.
2. Find **Bunny Keyboard**, click **⋯** and then **Uninstall**.
3. Click **Yes** when Windows asks for permission. A message says when it is
   done.

**Bunny Keyboard is not in Installed apps?** You installed it before it was
added there. Run the [install line](#install) once to update it, and then it
will be there to uninstall.

If Khmer was in your language list only for Bunny Keyboard, it is taken off
too, so your PC is back to how it was.

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

## Help make it better

The words you add, and the ones you fix, can make Bunny Keyboard better for
everyone. To send them, open **Bunny Keyboard - My words** and the **Share**
tab:

1. It lists your own words, and the words you corrected by picking the
   second or third suggestion instead of the first.
2. Untick any you would rather keep to yourself, and click **Send**.
3. A form opens in your browser with the words filled in. Check them and click
   **Submit**. You do not need an account.

You can also tick **Also send it to Bunny Keyboard** when you add a word, or
when you use **Wrong word?**.

**What is kept, and what is sent:**

- When you pick a suggestion other than the first, the keyboard notes just
  that word and its spelling (for example `tv = ធ្វើ`) in
  `%APPDATA%\Bunny Keyboard\learned.txt` on your PC. It notes only words you
  picked from the suggestions, never the rest of what you write.
- Nothing leaves your PC until you click **Submit** on the form.
- Words you send are given to the public domain (CC0), so anyone may use them,
  in Bunny Keyboard and elsewhere. A word goes into the dictionary once
  several people have sent the same one.

You can also report a wrong word or any other problem on
[GitHub](https://github.com/SomnangPery/Singkhmer-Khmer/issues).

---

For developers (building it, the Android app, how the engine works, the
data): [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). The word lists come from
open datasets listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
