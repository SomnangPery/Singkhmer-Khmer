# Singkhmer-Khmer

A Khmer keyboard for Android and Windows, with two ways of typing:

- **Singkhmer.** Type Khmer the way people write it in chat, in Latin
  letters, and get Khmer script: `nh jg tv sala rean` becomes
  ខ្ញុំចង់ទៅសាលារៀន. Suggestions show while you type. Everything runs on the
  device, with no internet.
- **Khmer.** The standard Khmer (NiDA) layout, with one difference: every
  syllable is stored in the one correct order, whichever order its parts are
  typed in. Text that looks the same is then stored the same, so it searches,
  sorts and displays properly in every app.

All the Khmer logic is one Rust library, `crates/khmer-core`. Each platform
adds a thin layer on top that draws the keys and passes key presses in.

**This is an early prototype.** The keyboards build and pass their tests on a
computer, but neither has been tried on a real phone or in a real Windows app
yet.

| Where | State | How to get it |
| --- | --- | --- |
| Command line | Works | [Try it](#try-it), below |
| Android 8.0 and newer | Builds and passes its tests; not yet tried on a phone | [The Android app](#the-android-app) |
| Windows 10 and 11, 64-bit | Builds and passes its tests; not yet installed or tried in a real app | [The Windows keyboard](#the-windows-keyboard) |
| iPhone, Mac, Linux | Not started | |

## Try it

Install Rust from <https://rustup.rs>, then:

```
git clone https://github.com/SomnangPery/Singkhmer-Khmer.git
cd Singkhmer-Khmer
cargo run -p khmer-cli -- roman nh jg tv sala rean --out result.txt
```

Open `result.txt` in an editor to see the three best readings of the
sentence. (A terminal shows Khmer broken up; see [Running it](#running-it).)
`cargo run -p khmer-cli -- roman` with no sentence converts each line you
type.

Where the rest is:

- how each mode works and why: [The stored order](#the-stored-order),
  [Romanized mode](#romanized-mode);
- the word list and how to correct it: [The data files](#the-data-files);
- where the data and the rules come from: [References](#references).

## What is built

| Roadmap step | State |
| --- | --- |
| 1. Normalizer | Built: `normalize`, `validate`, and Khmer-mode typing through `Engine` |
| 2. Word list, language model, segmenter | Partly. The word list and the word-pair counts are imported from open datasets. The model is still a stand-in that looks one word back, and there is no segmenter |
| 3. Romanized converter | Prototype built: about 800 hand-written words plus about 122,000 imported ones, command-line demo, works through `Engine` |
| 4. Android | First version built: a keyboard app with Khmer, Singkhmer and English layouts. Builds and passes its tests on a computer; not yet tried on a phone |
| 5. Windows | First version built: an input method with Singkhmer and Khmer keyboards. Builds and passes its tests; not yet installed or tried in a real app |
| 6+. Other platforms | Not started |

## Layout

```
crates/khmer-core/          the engine (no dependencies)
  src/chars.rs              character classes: the table everything else uses
  src/normalize.rs          puts each syllable into its one stored order
  src/validate.rs           checks text against the syllable structure
  src/compose.rs            builds one syllable from key presses
  src/engine.rs             the API a platform keyboard calls
  src/romanize/             romanized mode
    skeleton.rs             the spelling variant rules (kh/k, ch/j, ...)
    lexicon.rs              the romanization dictionary and its lookups
    model.rs                the language model interface and a stand-in model
    search.rs               beam search for the most likely sentence
    phrase.rs               the phrase being typed, key by key
  data/words.tsv            Khmer words with their romanized spellings, hand-checked
  data/words-imported.tsv   more words, imported from other sources
  data/pairs.tsv            which word tends to follow which
  data/english.txt          English words that stay in Latin letters
  tests/data/cases.tsv      test cases as code points; add yours here
  tests/data/sentences.tsv  romanized test sentences; add yours here
  tests/cases.rs            runs cases.tsv
  tests/romanize.rs         romanized mode with the real data, and its accuracy
  tests/properties.rs       rules checked on thousands of random inputs
crates/khmer-cli/           command-line tool for trying the engine
crates/khmer-android/       the engine as a native library for the Android app
android/                    the Android keyboard app (open this folder in Android Studio)
  app/src/main/java/com/newkeyboard/khmer/
    KhmerImeService.kt      the keyboard: passes keys to the engine, writes its answers into the app
    KeyboardView.kt         draws the keys and turns touches into key presses
    Layouts.kt              which keys are on which layout; edit this to move a key
    SuggestionBar.kt        the strip of suggestions above the keys
    NativeEngine.kt         the Kotlin side of crates/khmer-android
    SetupActivity.kt        the screen behind the app icon: how to turn the keyboard on
  app/src/test/             tests that run on a computer, without a phone
crates/khmer-windows/       the Windows keyboard (builds khmer_tsf.dll)
  src/lib.rs                what Windows calls to load the DLL
  src/service.rs            the keyboard: passes keys to the engine, writes its answers into the app
  src/keys.rs               which key does what in each mode; edit this to move a key
  src/candidates.rs         the window that lists the suggestions
  src/register.rs           tells Windows that the keyboard exists
  tests/typing.rs           types into a stand-in text field through the real Windows text framework
  tests/preview.rs          draws the suggestion window into a picture
windows/install.ps1         installs the Windows keyboard on this PC, or removes it
windows/setup.ps1           the one-line install from GitHub: downloads the release, runs install.ps1
```

## Running it

Install Rust from <https://rustup.rs>, then:

```
cargo test
cargo run -p khmer-cli -- normalize --hex 179F 17B8 17D2 179A 17D2 178F
cargo run -p khmer-cli -- check "ខ្ញុំ"
cargo run -p khmer-cli -- type --hex 1789 17BB 17B6 17C6
cargo run -p khmer-cli -- roman nh jg tv sala rean
cargo run -p khmer-cli -- roman
cargo run -p khmer-cli -- keys "nh jg tv"
cargo run -p khmer-cli -- accuracy crates/khmer-core/tests/data/sentences.tsv
```

`normalize`, `check` and `type` print every result as text and as code points,
because two strings that look the same on screen can be stored differently.

`roman` shows the top three suggestions for a romanized sentence; with no
sentence it converts each line you type. `keys` types through the engine one
key at a time and shows what a keyboard would show after each key.
`accuracy` measures a file of test sentences.

Khmer looks broken in a terminal: the letters are there, but subscripts and
vowels sit side by side instead of stacking. Terminals draw one character per
cell and do not do the text shaping Khmer needs. The stored text is correct.
To read it properly, add `--out FILE` to any command and open the file in an
editor:

```
cargo run -p khmer-cli -- roman nh jg tv sala rean --out result.txt
```

On Windows with the GNU toolchain (`x86_64-pc-windows-gnu`), the test
dependencies fail to build with a `dlltool` error unless rustc is pointed at a
working one. Run `rustup component add llvm-tools`, copy `llvm-ar.exe` from the
toolchain's `lib/rustlib/x86_64-pc-windows-gnu/bin` folder to a file named
`llvm-dlltool.exe`, and set this in `.cargo/config.toml`:

```
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "dlltool=C:/path/to/llvm-dlltool.exe"]
```

The MSVC toolchain does not need this.

## The Android app

The app in `android/` is a system keyboard. It has three ways of typing, and
the globe key goes from one to the next:

| Mode | Keys | What happens to a key press |
| --- | --- | --- |
| ខ្មែរ | The standard Khmer (NiDA) layout. Hold a key for its shifted character | Goes through the engine's Khmer mode, so a syllable is stored in the correct order whichever order its parts are typed in |
| Singkhmer | QWERTY | Goes through the engine's romanized mode. The suggestions show above the keys |
| English | QWERTY | Written straight into the app |

Password, email, web address and number fields start in English by
themselves. Holding the globe key offers the phone's other keyboards.

### Building it

You need Rust with the Android targets, and Android Studio with the NDK
(version 28.2.13676358, set in `android/app/build.gradle.kts`):

```
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

Then open the `android` folder in Android Studio and press Run, or from a
terminal:

```
cd android
gradlew assembleDebug        builds app-debug.apk
gradlew installDebug         builds it and installs it on the phone that is plugged in
gradlew testDebugUnitTest    runs the tests, on the computer
```

Building the app builds the Rust engine too (the `cargoBuild` tasks in
`android/app/build.gradle.kts`). The finished app is in
`app/build/outputs/apk/debug/`, or under the folder named by `build.dir` in
`android/local.properties` if that line is there. That line moves all build
output out of the project folder; use it when the project is in a synced
folder such as OneDrive.

On the phone, open the Khmer Keyboard app and follow its three steps: turn the
keyboard on in the system settings, choose it, and try it.

### Its tests

Both run on a computer, with the real engine:

- `TypingTest` types through the keyboard into a stand-in text field and
  checks what ends up there: a Singkhmer phrase, a Khmer syllable typed out of
  order, Backspace, Enter, moving the cursor in the middle of a phrase, picking
  a suggestion, a password field.
- `KeyboardPreviewTest` draws every layout and saves it as a picture in the
  build folder's `keyboard-previews`. Look at them after changing `Layouts.kt`.

### Not built yet

- Trying it on real phones. Everything above was checked on a computer only.
- The dictionary is read as text when the keyboard starts (see "Not built
  yet" under Romanized mode). This is done in the background, but a key pressed
  in Singkhmer mode before it finishes has to wait. With 120,000 imported
  words it takes about 1.5 seconds on a computer and will take longer on a
  phone.
- Capital letters at the start of a sentence, sound on key press, a settings
  screen, swipe typing, emoji.
- A release build signed for the Play Store.

## The Windows keyboard

`crates/khmer-windows` builds `khmer_tsf.dll`, an input method for 64-bit
Windows 10 and 11. Once installed it works in every app. It adds two keyboards
to the Khmer language, and Windows+Space switches between them and your other
keyboards:

| Keyboard | Keys | What happens to a key press |
| --- | --- | --- |
| Singkhmer | QWERTY | Goes through the engine's romanized mode. The phrase shows underlined in the app while it is typed, and the suggestions show in a small window under it |
| Khmer NiDA | The standard Khmer layout, with Shift and AltGr | Goes through the engine's Khmer mode, so a syllable is stored in the correct order whichever order its parts are typed in |

Both go by where a key is on the keyboard, not by the letter printed on it,
so they type the same on every PC.

In Singkhmer, on top of the keys listed under "How it types" below:

| Key | What happens |
| --- | --- |
| Enter | Sends the highlighted suggestion to the app |
| Down arrow or Tab, Up arrow or Shift+Tab | Highlights the next or the previous suggestion |
| Space, with another suggestion highlighted | Sends that suggestion to the app |
| A click on a suggestion | Sends it to the app |
| Esc | Throws the phrase away |
| A digit or punctuation with no phrase being typed | Typed as on a US keyboard |

Ctrl and Alt shortcuts are always left to the app.

### Installing it

On any 64-bit Windows 10 or 11 PC, with no Rust and no copy of the project,
paste this into Windows PowerShell:

```
irm https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/setup.ps1 | iex
```

It downloads the keyboard from the latest release and runs `install.ps1` on
it. Windows asks for administrator permission once. Then press Windows+Space:
"Singkhmer (Khmer Keyboard)" and "Khmer NiDA (Khmer Keyboard)" are listed
under Khmer. To take it off again:

```
& ([scriptblock]::Create((irm https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/setup.ps1))) -Remove
```

A release is made by pushing a tag (`git tag v0.1.0`, then
`git push origin v0.1.0`): `.github/workflows/release-windows.yml` builds
`khmer_tsf.dll` on GitHub and attaches it, `install.ps1` and `setup.ps1` to
the release.

To build and install from the project folder instead, in Windows PowerShell:

```
powershell -ExecutionPolicy Bypass -File windows\install.ps1
```

This builds the DLL, copies it to `C:\Program Files\Khmer Keyboard`, tells
Windows about it, and adds the two keyboards to your Khmer language. Windows
asks for administrator permission once, for the copying and the telling. Then
press Windows+Space and choose "Singkhmer (Khmer Keyboard)". Apps that were
already open may need a restart before they offer it.

Run the same command again after changing the code. To take the keyboard off
the PC:

```
powershell -ExecutionPolicy Bypass -File windows\install.ps1 -Remove
```

Windows loads the DLL into every app you type in, so a bug in it can close
that app. If apps start closing while you type, switch to another keyboard
with Windows+Space and run the `-Remove` command.

### Its tests

`cargo test -p khmer-windows` runs them. Nothing is installed for it.

- The tests in `keys.rs` check the key tables: which key gives which
  character in each mode, with Shift, Caps Lock and AltGr.
- `tests/typing.rs` starts the real Windows text framework on the test's own
  thread, gives it a text field that exists only in memory, and types into it
  through the keyboard: a Singkhmer phrase, a Khmer syllable typed out of
  order, Backspace, Esc, picking a suggestion, typing in the middle of other
  text, switching keyboard mid-phrase, leaving the field, the app ending the
  phrase itself, a read-only field.
- `tests/preview.rs` draws the suggestion window and saves it as a picture
  (the test prints where). Look at it after changing `candidates.rs`.

### Not built yet

- Trying it in real apps. The keyboard has not been installed on any PC.
  Windows only hands key presses to an installed keyboard, so that last step,
  and how Notepad, Word or a browser behave, is not covered by the tests.
  `install.ps1` has not been run with administrator permission either; only
  its build step and its handling of a refused registration were tried.
- 32-bit apps. They need a 32-bit copy of the DLL, which is not built, and do
  not offer the keyboard.
- A shared dictionary. Each app Singkhmer is used in reads its own copy: about
  1 second and 160 MB, measured with the optimised build. It is read in the
  background when the keyboard is selected, but a key pressed before it is
  done waits. The Khmer NiDA keyboard reads no dictionary.
- An icon in the taskbar (Windows shows the language's own sign), a dark
  suggestion window, a settings screen.
- An entry under "Installed apps", and a signed installer for other people's
  PCs.

## Adding a test

When you find Khmer text that displays badly somewhere, add a line to
`crates/khmer-core/tests/data/cases.tsv`: the code points as you found them,
and the code points they should become. No Rust needed.

## The stored order

A syllable is stored as:

```
Base  Robat?  Subscripts (up to 2)  Shifter?  Vowel?  Signs (up to 2)  Final?
```

This is the Modern Khmer structure of Unicode Technical Note #61, "Khmer
Encoding Structure" (version 2, February 2025), which exists so that one visual
form has exactly one encoding. Its reference normalizer is the model for
`normalize`.

## Decisions

Each rule has a test in `cases.tsv`.

| Decision | Why |
| --- | --- |
| The shifter (៉ ៊) is stored after the subscripts: ម ្យ ៉ ា ង | UTN #61. The Unicode 16.0 core text puts it straight after the base instead. The note argues for after the subscripts: current shaping engines render it, Khmer sorting rules depend on it, and in the note's web counts it is already the more common spelling. |
| Subscript រ is stored after any other subscript | UTN #61. The two orders look the same, so one must be chosen. |
| Subscript ដ is stored as subscript ត | UTN #61. They look identical in Modern Khmer and are constantly confused. Can be turned off with `Options { fold_coeng_da: false }`. |
| Robat is stored straight after the base | UTN #61. |
| េ + ី becomes ើ, and េ + ា becomes ោ | UTN #61 "do not use" sequences: the pair looks like the single vowel. |
| េ + េ becomes ែ | Our own rule, in the same spirit. Not in UTN #61. |
| ុ followed by an above vowel (ិ ី ឹ ឺ ើ), by ាំ, by ៝, or (weak clusters) by ័ becomes a shifter | UTN #61. What looks like ុ there is a shifter that fonts draw low. It becomes ៊ if the cluster has a strong consonant and no ប, otherwise ៉. |
| Deprecated ឣ and ឤ become អ and អា | The Unicode Standard says to use these instead. |
| ZWNJ is moved only when it is attached to a shifter | Elsewhere it may control a ligature, and moving it could change how text looks. |

`normalize` does not delete anything it cannot explain. A vowel with no
consonant, a coeng sign with no consonant after it, two real vowels on one
consonant: these are left as they are and reported by `validate`.

Typing is stricter, because there the engine can simply refuse a key. In
Khmer mode a second vowel replaces the first, a repeated sign is dropped, a
third subscript is dropped, and a vowel, sign or coeng sign with no consonant
is ignored. Any key that would leave no valid syllable is ignored, so text
typed through `Engine` is always valid.

Not followed from UTN #61:

- ើ + ា versus ោ + ី. The note's grammar and its reference code disagree on
  which to keep, and neither occurs in Modern Khmer. Left alone.
- ឿ is not treated as a vowel that pushes a shifter down. The note's grammar
  leaves it out and its reference code includes it.
- Middle Khmer (final subscripts marked with ZWJ, several vowels on one
  consonant) and the lunar date symbols are out of scope.

## Romanized mode

Typing `nh jg tv sala rean` gives ខ្ញុំចង់ទៅសាលារៀន. The steps are the ones in
the design document:

1. The input is split into tokens at spaces.
2. Each token gets candidates from `words.tsv`: a listed spelling (`nh`), a
   spelling that differs only by a variant rule (`khnhom` for `knhom`), or a
   longer spelling that starts with the token (`sal` for `sala`).
3. A beam search keeps the five best sentences while it goes through the
   tokens. Each word is scored by how well the spelling fits it plus how
   likely the word is after the word before it.
4. The best three are the suggestions.
5. A token with no good Khmer match stays in Latin letters, and so does a word
   from `english.txt` unless `words.tsv` lists exactly that spelling.

### How it types

| Key | What happens |
| --- | --- |
| A letter | Added to the token being typed, which is shown as typed |
| Space | Ends the token: it turns into the top suggestion's Khmer word. Nothing is sent to the app yet |
| Space again | Sends the phrase to the app, followed by a real space |
| Punctuation, a digit, Enter | Sends the phrase to the app, followed by that character |
| Picking a suggestion | Sends that suggestion to the app |
| Backspace | Undoes one key, including a Space |

The phrase is sent by itself after eight tokens. Khmer words are joined
without spaces; a word kept in Latin letters gets a space on each side.

### The data files

All three are plain text and can be edited without knowing Rust. `cargo test`
reports a bad line with its line number.

- `words.tsv`: one Khmer word per line, with a rough count per million words,
  its romanized spellings (most usual first), and the meaning. The columns are
  separated by a TAB character. This is the hand-checked list, and it wins:
  put every correction here.
- `words-imported.tsv`: word lists taken from other sources, in the same
  format. Made by `khmer import`, not edited by hand. Today it holds about
  122,000 words gathered from the open datasets listed under
  [References](#references). A word that `words.tsv` also has keeps its count
  and spellings from there and gets the imported spellings added after them.
- `pairs.tsv`: word pairs with a rough percent, for example "after ចង់, the
  next word is ទៅ about 12% of the time". Hand-written weights for common chat
  words, plus pairs counted from a Khmer text corpus.
- `english.txt`: one English word per line.

The `datasets` folder holds the gathered pair files that
`words-imported.tsv` is made from, one per source, and the scripts that
fetched them. The raw downloads themselves are not in the repository.

**The words, spellings and counts in `words.tsv` were written by hand as a
first draft and have not been checked against real text.** Some spellings will
be wrong or unusual. Correcting `words.tsv` is the most useful thing anyone
can do for this mode.

An imported word gets its count from a frequency list when it is found in
one, and a low floor count otherwise, so a common word that no list has is
ranked too low. The imported list also has some misspelled Khmer words and odd
spellings; nothing in it has been reviewed word by word.

### Adding a dataset

Any file with one Khmer/Latin pair per line can be imported. The two sides may
be in either order, separated by a TAB, a colon or a comma, and extra columns
are ignored:

```
cargo run -p khmer-cli -- import pairs.csv crates/khmer-core/data/words-imported.tsv --note "Source: ..." --note "Licence: ..."
cargo test
```

The command reports how many rows it kept and how many it dropped for each
reason (a space in the Latin side, a Khmer side with digits or the repeat sign
ៗ, and so on). It replaces the output file, so to combine several sources, join
their pair files into one first. Before importing, check that the source's
licence allows the use, and add it to `THIRD-PARTY-NOTICES.md`.

### Accuracy

`tests/data/sentences.tsv` holds 78 test sentences, and `cargo test` fails if
the right answer is among the first three suggestions for fewer than 80% of
them (the roadmap's goal for the prototype). Today the result is 100%, but
that number means little: the sentences were written together with the
dictionary. Real accuracy has to be measured on real messages typed by other
people. Put them in a file in the same format and run `khmer accuracy FILE`.

### Decisions

| Decision | Why |
| --- | --- |
| Space turns a token into Khmer but the phrase is only sent to the app when it ends | The design document says both "Space commits the top one" and "a beam search keeps the best 5 sentences as the user types". Keeping the phrase open lets a later word correct an earlier one. To the user, Space still makes the word appear in Khmer |
| The suggestions are whole phrases, not single words | The design document's example shows whole sentences as alternatives |
| A longer spelling can match any token, not only the last one | Then Space always gives exactly what the first suggestion showed |
| A listed spelling beats a variant, and a known English word normally beats a variant or a prefix match | "ok" must not become អរគុណ because "okun" starts with it |
| A Khmer mark typed with no consonant is dropped | Same rule as Khmer mode, so only valid text reaches the app |

### Not built yet

- Learning the user's own spellings (the personal dictionary).
- The real language model. `SeedModel` is a stand-in that looks one word
  back; the real one will be counted from Khmer text and look two words back.
  It plugs in through the `LanguageModel` trait.
- Switching one word between Khmer and Latin by long-press. For now the Latin
  form shows up among the suggestions.
- Storing the dictionary as an `fst`. The whole list is read as text when the
  engine starts, which is fine on a computer but has not been timed on a phone.
- A speed benchmark. In a rough check with the optimised build, 88 key presses
  added no measurable time on top of starting the program, but the time per
  key has not been measured properly.
- Tuning the scoring weights in `lexicon.rs` and `search.rs`. They are first
  guesses.

## References

### Standards and platform documentation

- Unicode Technical Note #61, *Khmer Encoding Structure* (version 2,
  February 2025): <https://www.unicode.org/notes/tn61/>. The stored order of
  a syllable and the rules of `normalize` follow it; see
  [The stored order](#the-stored-order) for where this project differs.
- The Unicode Standard, Khmer code chart (U+1780 to U+17FF):
  <https://www.unicode.org/charts/PDF/U1780.pdf>.
- Microsoft, *Text Services Framework*:
  <https://learn.microsoft.com/windows/win32/tsf/text-services-framework>.
  The Windows keyboard is a text service of this framework.
- Android Developers, *Create an input method*:
  <https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method>.
  The Android app is an input method service.

### Libraries

- [windows-rs](https://github.com/microsoft/windows-rs): the Windows API for
  Rust, used by `crates/khmer-windows`.
- [jni-rs](https://github.com/jni-rs/jni-rs): the bridge between Rust and
  Kotlin, used by `crates/khmer-android`.
- [proptest](https://github.com/proptest-rs/proptest): the random-input
  tests of the core.
- [Robolectric](https://robolectric.org) and [JUnit 4](https://junit.org/junit4/):
  the Android tests that run on a computer.

`khmer-core` itself has no dependencies.

### Data sources

The imported word list, the word-pair counts and the test sentences in
`datasets` were gathered from the projects below. Each has its own licence
and terms; the details, with what was taken from each, are in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

| Source | What it gave |
| --- | --- |
| [Chhunneng/khmer-text-transliteration](https://github.com/Chhunneng/khmer-text-transliteration) | Khmer and Latin word pairs |
| [Akkhara keyboard](https://github.com/MuyleangIng/akkhara-keyboard), [Akkhara desktop](https://github.com/MuyleangIng/akkhara-desktop) and the [Akkhara-Sing-Khmer dataset](https://huggingface.co/datasets/muyleanging/Akkhara-Sing-Khmer) | Spellings, abbreviations, word-pair counts, test sentences |
| [richardcheam/roman2khmer](https://github.com/richardcheam/roman2khmer) | Spellings, loanwords, chat test sentences |
| [Nel-sokchhunly/typer](https://github.com/Nel-sokchhunly/typer) | Spellings, word frequencies |
| [darhnoel/khmerime](https://github.com/darhnoel/khmerime) | Spellings |
| [HengHuyLong/SingKhmerKeyboard](https://github.com/HengHuyLong/SingKhmerKeyboard) | Spellings |
| [chhay-sophal/Khmer-Loeun](https://github.com/chhay-sophal/Khmer-Loeun) | Provinces, offices, schools, family words |
| [chsahaka/singkhmertran](https://github.com/chsahaka/singkhmertran) | Chat abbreviations |
| [M0nGKol/singkhmer](https://github.com/M0nGKol/singkhmer) | Chat words |
| [pheasaKhmer/khmer-engine](https://github.com/pheasaKhmer/khmer-engine) | Chat spellings, test sentences |
| [google/language-resources](https://github.com/google/language-resources) (Khmer lexicon) | Phonetic spellings |
| [leangsuor/latin_to_khmer_transliteration_model](https://huggingface.co/leangsuor/latin_to_khmer_transliteration_model) | Spellings, test sentences |
| [Kaikki.org Khmer dictionary](https://kaikki.org/dictionary/Khmer/) and [Vuizur/Wiktionary-Dictionaries](https://github.com/Vuizur/Wiktionary-Dictionaries), both from Wiktionary | Spellings |
| [sungkhum/tiptap-khmer-line-breaker](https://github.com/sungkhum/tiptap-khmer-line-breaker) | Words with more than one Khmer spelling |
| [Wikidata](https://www.wikidata.org) | Place names and other proper nouns |
| [silnrsi/khmerlbdict](https://github.com/silnrsi/khmerlbdict) (SEALang frequency list) | Word frequencies |

The word frequencies also draw on two public collections of Khmer Facebook
comments and reviews, and the chat slang list on a small hand-made set; both
are described in the notices file.
