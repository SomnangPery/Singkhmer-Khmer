# The "my words" window of Bunny Keyboard.
#
# My words tab: your own Singkhmer spellings for Khmer words, kept in
# %APPDATA%\Bunny Keyboard\my-words.txt ("bong = បង"). The keyboard reads that
# file again whenever it changes, so a saved word works at the next key press.
#
# Share tab: words to send to Bunny Keyboard's word list, through a Google
# Form, so everyone's dictionary gets better. It lists your own words and the
# words you corrected by picking a suggestion other than the first (noted by
# the keyboard in learned.txt: word pairs only, never what you wrote). Nothing
# is sent until you press Send and then Submit on the form.
#
# Opened from the Start menu ("Bunny Keyboard - My words") and from the
# buttons at the bottom of the suggestion popup:
#   "+ Add my word"  -Spelling <the word just typed>
#   "Wrong word?"    -Report -Spelling <word> -Typed <phrase> -Got <what the keyboard gave>

param(
    [string]$Spelling = '',
    [switch]$Report,
    [string]$Typed = '',
    [string]$Got = '',
    # For checking the look: saves a picture of the window instead of showing
    # it. -Tab picks the tab: 0 My words, 1 Share.
    [string]$Snapshot,
    [int]$Tab = 0,
    # For checking the form link: prints it for these lines and stops.
    [string[]]$PrintLink
)

# ---- where shared words go ----
#
# The Google Form for shared words. Leave $FormUrl empty to switch sharing
# off. $FormUrl is the form's address ending in /viewform. The two fields are
# the "entry.<number>" of its questions, from the form's pre-filled link:
# the Khmer question (a paragraph) gets one "spelling = Khmer" line per word,
# and the Singkhmer question gets the spellings on one line.
$FormUrl = 'https://docs.google.com/forms/d/e/1FAIpQLSe2-on-wyrh2fevqq8J9xHY5pr8sWre5k0pcGvKpGgDJTcXRA/viewform'
$FormWordsField = 'entry.1030374765'
$FormSpellingsField = 'entry.1067540385'
# A pre-filled link holds this many words at most; more go in a second send.
$MostPerSend = 25

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace BunnyKeyboard -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][BunnyKeyboard.Dpi]::SetProcessDPIAware()
[System.Windows.Forms.Application]::EnableVisualStyles()

$Folder = Join-Path $env:APPDATA 'Bunny Keyboard'
$File = Join-Path $Folder 'my-words.txt'
$LearnedFile = Join-Path $Folder 'learned.txt'
$SharedFile = Join-Path $Folder 'shared.txt'
$Header = @(
    '# My words for Bunny Keyboard (Singkhmer).'
    '# One per line: the spelling, = and the Khmer word. For example: bong = បង'
    '# The keyboard reads this file again whenever it changes.'
)
$Utf8 = New-Object Text.UTF8Encoding $false
$SharingOn = [bool]($FormUrl -and $FormWordsField)

function Read-Pairs([string]$path) {
    $pairs = New-Object System.Collections.Generic.List[object]
    if (Test-Path $path) {
        foreach ($line in [IO.File]::ReadAllLines($path, [Text.Encoding]::UTF8)) {
            if ($line.Trim() -eq '' -or $line.StartsWith('#')) { continue }
            $parts = if ($line.Contains("`t")) { $line.Split("`t", 2) } else { $line.Split('=', 2) }
            if ($parts.Count -eq 2 -and $parts[0].Trim() -and $parts[1].Trim()) {
                $pairs.Add([pscustomobject]@{ Spelling = $parts[0].Trim().ToLower(); Khmer = $parts[1].Trim() })
            }
        }
    }
    # The comma keeps PowerShell from unrolling the list into its items.
    ,$pairs
}

function Save-Words($words) {
    New-Item -ItemType Directory -Force -Path $Folder | Out-Null
    $lines = @($Header) + @($words | ForEach-Object { "$($_.Spelling) = $($_.Khmer)" })
    [IO.File]::WriteAllLines($File, [string[]]$lines, $Utf8)
}

function Add-Shared($pairs) {
    New-Item -ItemType Directory -Force -Path $Folder | Out-Null
    [IO.File]::AppendAllLines($SharedFile, [string[]]@($pairs | ForEach-Object { "$($_.Spelling) = $($_.Khmer)" }), $Utf8)
}

function Test-Khmer([string]$text) {
    $text.Length -gt 0 -and -not ($text.ToCharArray() | Where-Object { [int]$_ -lt 0x1780 -or [int]$_ -gt 0x17FF })
}

# The link that opens the form with the words filled in. Each line is
# "spelling = Khmer", with what the keyboard gave when the user reported it.
function Get-FormLink($lines) {
    $spellings = @($lines | ForEach-Object { ($_ -split '=', 2)[0].Trim() }) -join ', '
    $url = "$FormUrl" + '?usp=pp_url&' + $FormWordsField + '=' + [Uri]::EscapeDataString(($lines -join "`n"))
    if ($FormSpellingsField) {
        $url += '&' + $FormSpellingsField + '=' + [Uri]::EscapeDataString($spellings)
    }
    $url
}

function Send-Words($lines) {
    Start-Process (Get-FormLink $lines)
}

if ($PrintLink) {
    Get-FormLink $PrintLink
    return
}

$Words = Read-Pairs $File

# ---- the window ----

$ui = New-Object System.Drawing.Font('Segoe UI', 10)
$khmerFont = New-Object System.Drawing.Font('Leelawadee UI', 13)
$listFont = New-Object System.Drawing.Font('Leelawadee UI', 11)
$titleFont = New-Object System.Drawing.Font('Segoe UI Semibold', 14)
$accent = [System.Drawing.Color]::FromArgb(0, 95, 184)
$grey = [System.Drawing.Color]::FromArgb(96, 96, 96)
$red = [System.Drawing.Color]::FromArgb(196, 43, 28)
$green = [System.Drawing.Color]::FromArgb(16, 124, 16)

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Bunny Keyboard - My words'
$form.Font = $ui
$form.StartPosition = 'CenterScreen'
$form.AutoScaleMode = 'Dpi'
$form.ClientSize = New-Object System.Drawing.Size(480, 660)
$form.MinimumSize = New-Object System.Drawing.Size(440, 560)
$form.BackColor = [System.Drawing.Color]::White
$form.TopMost = $true
$form.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon((Join-Path $PSHOME 'powershell.exe'))

$tabs = New-Object System.Windows.Forms.TabControl
$tabs.Dock = 'Fill'
$tabs.Padding = New-Object System.Drawing.Point(14, 5)
$form.Controls.Add($tabs)

function New-Page([string]$title) {
    $page = New-Object System.Windows.Forms.TabPage($title)
    $page.BackColor = [System.Drawing.Color]::White
    $layout = New-Object System.Windows.Forms.TableLayoutPanel
    $layout.Dock = 'Fill'
    $layout.Padding = New-Object System.Windows.Forms.Padding(12)
    $layout.ColumnCount = 2
    [void]$layout.ColumnStyles.Add((New-Object System.Windows.Forms.ColumnStyle('Percent', 50)))
    [void]$layout.ColumnStyles.Add((New-Object System.Windows.Forms.ColumnStyle('Percent', 50)))
    $page.Controls.Add($layout)
    [void]$tabs.TabPages.Add($page)
    $layout
}

function New-Label([string]$text, [System.Drawing.Font]$font = $ui, [System.Drawing.Color]$colour = [System.Drawing.Color]::FromArgb(26, 26, 26)) {
    $label = New-Object System.Windows.Forms.Label
    $label.Text = $text
    $label.Font = $font
    $label.ForeColor = $colour
    $label.AutoSize = $true
    $label.MaximumSize = New-Object System.Drawing.Size(430, 0)
    $label.Margin = New-Object System.Windows.Forms.Padding(0, 0, 0, 6)
    $label
}

function New-Button([string]$text, [bool]$primary) {
    $button = New-Object System.Windows.Forms.Button
    $button.Text = $text
    $button.AutoSize = $true
    $button.Cursor = [System.Windows.Forms.Cursors]::Hand
    if ($primary) {
        $button.FlatStyle = 'Flat'
        $button.FlatAppearance.BorderSize = 0
        $button.BackColor = $accent
        $button.ForeColor = [System.Drawing.Color]::White
        $button.Padding = New-Object System.Windows.Forms.Padding(14, 4, 14, 4)
    }
    $button
}

function Set-Rows($layout, [int]$fill, [int]$count) {
    foreach ($row in 0..($count - 1)) {
        $style = if ($row -eq $fill) { New-Object System.Windows.Forms.RowStyle('Percent', 100) } else { New-Object System.Windows.Forms.RowStyle('AutoSize') }
        [void]$layout.RowStyles.Add($style)
    }
}

function Put($layout, $control, [int]$column, [int]$row, [int]$span = 1) {
    $layout.Controls.Add($control, $column, $row)
    if ($span -gt 1) { $layout.SetColumnSpan($control, $span) }
}

# ---- My words tab ----

$mine = New-Page 'My words'
if ($Report) {
    Put $mine (New-Label 'Fix a wrong word' $titleFont) 0 0 2
    $about = "You typed:  $Typed`nThe keyboard gave:  $Got`n`nWrite the spelling and the Khmer word you wanted. It will come first from now on."
} else {
    Put $mine (New-Label 'Add your own word' $titleFont) 0 0 2
    $about = 'When you type the spelling, your word comes first in the suggestions.'
}
Put $mine (New-Label $about $ui $grey) 0 1 2

Put $mine (New-Label 'Singkhmer spelling') 0 2
Put $mine (New-Label 'Khmer word') 1 2
$spellingBox = New-Object System.Windows.Forms.TextBox
$spellingBox.Font = $khmerFont
$spellingBox.Dock = 'Fill'
$spellingBox.Text = $Spelling.ToLower()
$spellingBox.Margin = New-Object System.Windows.Forms.Padding(0, 0, 8, 0)
$khmerBox = New-Object System.Windows.Forms.TextBox
$khmerBox.Font = $khmerFont
$khmerBox.Dock = 'Fill'
$khmerBox.Margin = New-Object System.Windows.Forms.Padding(0)
Put $mine $spellingBox 0 3
Put $mine $khmerBox 1 3

$hint = New-Label 'Tip: you can type the Khmer word here in Singkhmer too, then press Space twice.' $ui ([System.Drawing.Color]::FromArgb(112, 112, 112))
$hint.Margin = New-Object System.Windows.Forms.Padding(0, 6, 0, 4)
Put $mine $hint 0 4 2

$shareToo = New-Object System.Windows.Forms.CheckBox
$shareToo.Text = 'Also send it to Bunny Keyboard, to fix it for everyone'
$shareToo.AutoSize = $true
$shareToo.Checked = $Report -and $SharingOn
$shareToo.Enabled = $SharingOn
$shareToo.Margin = New-Object System.Windows.Forms.Padding(0, 2, 0, 4)
Put $mine $shareToo 0 5 2

$message = New-Label '' $ui $red
Put $mine $message 0 6 2

$addButton = New-Button 'Add word' $true
$addButton.Margin = New-Object System.Windows.Forms.Padding(0, 0, 0, 14)
Put $mine $addButton 0 7
$form.AcceptButton = $addButton

Put $mine (New-Label 'My words' (New-Object System.Drawing.Font('Segoe UI Semibold', 11))) 0 8 2

$list = New-Object System.Windows.Forms.ListView
$list.View = 'Details'
$list.FullRowSelect = $true
$list.HeaderStyle = 'Nonclickable'
$list.Font = $listFont
$list.Dock = 'Fill'
$list.BorderStyle = 'FixedSingle'
[void]$list.Columns.Add('Spelling', 180)
[void]$list.Columns.Add('Khmer', 230)
Put $mine $list 0 9 2

$removeButton = New-Button 'Remove selected' $false
$removeButton.Margin = New-Object System.Windows.Forms.Padding(0, 8, 0, 0)
Put $mine $removeButton 0 10
Set-Rows $mine 9 11

function Show-Words {
    $list.BeginUpdate()
    $list.Items.Clear()
    foreach ($word in $Words) {
        $item = New-Object System.Windows.Forms.ListViewItem($word.Spelling)
        [void]$item.SubItems.Add($word.Khmer)
        [void]$list.Items.Add($item)
    }
    $list.EndUpdate()
    $removeButton.Enabled = $Words.Count -gt 0
}

function Say([string]$text, [System.Drawing.Color]$colour) {
    $message.ForeColor = $colour
    $message.Text = $text
}

$addButton.Add_Click({
    $spelling = $spellingBox.Text.Trim().ToLower()
    $khmer = $khmerBox.Text.Trim()
    if ($spelling -notmatch '^[a-z]+$') {
        Say 'The spelling should be letters a-z only, with no spaces.' $red
        $spellingBox.Focus(); return
    }
    if (-not (Test-Khmer $khmer)) {
        Say 'The Khmer word should be Khmer letters only, with no spaces.' $red
        $khmerBox.Focus(); return
    }
    $known = $Words | Where-Object { $_.Spelling -eq $spelling -and $_.Khmer -eq $khmer }
    if (-not $known) {
        # The newest word for a spelling goes first: it is the one wanted now.
        $Words.Insert(0, [pscustomobject]@{ Spelling = $spelling; Khmer = $khmer })
        try {
            Save-Words $Words
        } catch {
            $Words.RemoveAt(0)
            Say "Could not save: $($_.Exception.Message)" $red
            return
        }
    }
    $done = if ($known) { "'$spelling = $khmer' is already in your words." } else { "Added. Type '$spelling' to get $khmer." }
    if ($shareToo.Checked) {
        $line = "$spelling = $khmer"
        if ($Got) { $line += " (keyboard gave: $Got; typed: $Typed)" }
        Send-Words @($line)
        Add-Shared @([pscustomobject]@{ Spelling = $spelling; Khmer = $khmer })
        $done += ' The form is open in your browser: press Submit there to send it.'
    }
    Say $done $green
    $spellingBox.Clear(); $khmerBox.Clear(); $shareToo.Checked = $false; $spellingBox.Focus()
    Show-Words
    Show-Shareable
})

$removeButton.Add_Click({
    $gone = @($list.SelectedIndices | Sort-Object -Descending)
    if (-not $gone) { return }
    foreach ($index in $gone) { $Words.RemoveAt($index) }
    Save-Words $Words
    Say 'Removed.' $grey
    Show-Words
    Show-Shareable
})

$spellingBox.Add_TextChanged({ $message.Text = '' })
$khmerBox.Add_TextChanged({ $message.Text = '' })

# ---- Share tab ----

$share = New-Page 'Share'
Put $share (New-Label 'Help make Bunny Keyboard better' $titleFont) 0 0 2
Put $share (New-Label ("These are your own words, and words you corrected by picking another " +
    "suggestion. Only the words are kept, never what you wrote. Tick the ones to send, then press " +
    "Send: a form opens in your browser with them filled in, and nothing is sent until you press " +
    "Submit there. Sent words are given to the public domain (CC0), so anyone may use them.") $ui $grey) 0 1 2

$shareList = New-Object System.Windows.Forms.ListView
$shareList.View = 'Details'
$shareList.CheckBoxes = $true
$shareList.FullRowSelect = $true
$shareList.HeaderStyle = 'Nonclickable'
$shareList.Font = $listFont
$shareList.Dock = 'Fill'
$shareList.BorderStyle = 'FixedSingle'
[void]$shareList.Columns.Add('Spelling', 140)
[void]$shareList.Columns.Add('Khmer', 170)
[void]$shareList.Columns.Add('From', 110)
$shareList.Margin = New-Object System.Windows.Forms.Padding(0, 6, 0, 0)
Put $share $shareList 0 2 2

$shareMessage = New-Label '' $ui $grey
$shareMessage.Margin = New-Object System.Windows.Forms.Padding(0, 8, 0, 4)
Put $share $shareMessage 0 3 2

$sendButton = New-Button 'Send...' $true
Put $share $sendButton 0 4
Set-Rows $share 2 5

function Show-Shareable {
    $sent = @{}
    foreach ($pair in (Read-Pairs $SharedFile)) { $sent["$($pair.Spelling) = $($pair.Khmer)"] = $true }
    $rows = [ordered]@{}
    foreach ($pair in $Words) {
        $rows["$($pair.Spelling) = $($pair.Khmer)"] = @{ Pair = $pair; From = 'your word'; Times = 0 }
    }
    foreach ($pair in (Read-Pairs $LearnedFile)) {
        $key = "$($pair.Spelling) = $($pair.Khmer)"
        if (-not $rows.Contains($key)) { $rows[$key] = @{ Pair = $pair; From = ''; Times = 0 } }
        $rows[$key].Times++
    }
    $shareList.BeginUpdate()
    $shareList.Items.Clear()
    foreach ($key in $rows.Keys) {
        if ($sent.ContainsKey($key)) { continue }
        $row = $rows[$key]
        $from = if ($row.From) { $row.From } elseif ($row.Times -eq 1) { 'picked' } else { "picked $($row.Times)x" }
        $item = New-Object System.Windows.Forms.ListViewItem($row.Pair.Spelling)
        [void]$item.SubItems.Add($row.Pair.Khmer)
        [void]$item.SubItems.Add($from)
        $item.Checked = $true
        $item.Tag = $row.Pair
        [void]$shareList.Items.Add($item)
    }
    $shareList.EndUpdate()
    $count = $shareList.Items.Count
    $sendButton.Enabled = $SharingOn -and $count -gt 0
    # A flat button keeps its colour when switched off: grey it by hand.
    $sendButton.BackColor = if ($sendButton.Enabled) { $accent } else { [System.Drawing.Color]::FromArgb(200, 200, 200) }
    $shareMessage.ForeColor = $grey
    $shareMessage.Text = if (-not $SharingOn) {
        'Sending is not switched on in this version yet.'
    } elseif ($count -eq 0) {
        'Nothing new to send. Words you add or correct will show up here.'
    } else {
        "$count word(s) not sent yet."
    }
}

$sendButton.Add_Click({
    $chosen = @($shareList.CheckedItems | Select-Object -First $MostPerSend | ForEach-Object { $_.Tag })
    if (-not $chosen) {
        $shareMessage.Text = 'Tick at least one word to send.'
        return
    }
    Send-Words @($chosen | ForEach-Object { "$($_.Spelling) = $($_.Khmer)" })
    Add-Shared $chosen
    Show-Shareable
    $shareMessage.ForeColor = $green
    $shareMessage.Text = "The form is open in your browser with $($chosen.Count) word(s): press Submit there to send them. Thank you!"
})

$form.Add_Shown({
    $form.TopMost = $false
    $form.Activate()
    if ($spellingBox.Text) { $khmerBox.Focus() } else { $spellingBox.Focus() }
})

Show-Words
Show-Shareable
$tabs.SelectedIndex = $Tab

if ($Snapshot) {
    $form.Show()
    [System.Windows.Forms.Application]::DoEvents()
    $picture = New-Object System.Drawing.Bitmap($form.Width, $form.Height)
    $form.DrawToBitmap($picture, (New-Object System.Drawing.Rectangle(0, 0, $form.Width, $form.Height)))
    $picture.Save($Snapshot, [System.Drawing.Imaging.ImageFormat]::Png)
    $form.Close()
    return
}
[void]$form.ShowDialog()
