# The "my words" editor: your own Singkhmer spellings for Khmer words.
#
# Opened from the Start menu ("Bunny Keyboard - My words") or from "Add my
# word" at the bottom of the suggestion popup, which passes the word just
# typed as -Spelling. The words are kept in %APPDATA%\Bunny Keyboard\my-words.txt,
# one per line ("bong = បង"); the keyboard reads that file again whenever it
# changes, so a saved word works at the next key press, in every app.

param(
    [string]$Spelling = '',
    # For checking the look: saves a picture of the window instead of showing it.
    [string]$Snapshot
)

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace KhmerKeyboard -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][KhmerKeyboard.Dpi]::SetProcessDPIAware()
[System.Windows.Forms.Application]::EnableVisualStyles()

$Folder = Join-Path $env:APPDATA 'Bunny Keyboard'
$File = Join-Path $Folder 'my-words.txt'
$Header = @(
    '# My words for Bunny Keyboard (Singkhmer).'
    '# One per line: the spelling, = and the Khmer word. For example: bong = បង'
    '# The keyboard reads this file again whenever it changes.'
)

function Read-Words {
    $words = New-Object System.Collections.Generic.List[object]
    if (Test-Path $File) {
        foreach ($line in [IO.File]::ReadAllLines($File, [Text.Encoding]::UTF8)) {
            if ($line.Trim() -eq '' -or $line.StartsWith('#')) { continue }
            $parts = if ($line.Contains("`t")) { $line.Split("`t", 2) } else { $line.Split('=', 2) }
            if ($parts.Count -eq 2) {
                $words.Add([pscustomobject]@{ Spelling = $parts[0].Trim().ToLower(); Khmer = $parts[1].Trim() })
            }
        }
    }
    # The comma keeps PowerShell from unrolling the list into its items.
    ,$words
}

function Save-Words($words) {
    New-Item -ItemType Directory -Force -Path $Folder | Out-Null
    $lines = @($Header) + @($words | ForEach-Object { "$($_.Spelling) = $($_.Khmer)" })
    [IO.File]::WriteAllLines($File, [string[]]$lines, (New-Object Text.UTF8Encoding $false))
}

function Test-Khmer([string]$text) {
    $text.Length -gt 0 -and -not ($text.ToCharArray() | Where-Object { [int]$_ -lt 0x1780 -or [int]$_ -gt 0x17FF })
}

$Words = Read-Words

# ---- the window ----

$ui = New-Object System.Drawing.Font('Segoe UI', 10)
$khmerFont = New-Object System.Drawing.Font('Leelawadee UI', 13)
$accent = [System.Drawing.Color]::FromArgb(0, 95, 184)

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Bunny Keyboard - My words'
$form.Font = $ui
$form.StartPosition = 'CenterScreen'
$form.AutoScaleMode = 'Dpi'
$form.ClientSize = New-Object System.Drawing.Size(460, 640)
$form.MinimumSize = New-Object System.Drawing.Size(420, 520)
$form.BackColor = [System.Drawing.Color]::White
$form.TopMost = $true
$form.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon((Join-Path $PSHOME 'powershell.exe'))

$layout = New-Object System.Windows.Forms.TableLayoutPanel
$layout.Dock = 'Fill'
$layout.Padding = New-Object System.Windows.Forms.Padding(16)
$layout.ColumnCount = 2
[void]$layout.ColumnStyles.Add((New-Object System.Windows.Forms.ColumnStyle('Percent', 50)))
[void]$layout.ColumnStyles.Add((New-Object System.Windows.Forms.ColumnStyle('Percent', 50)))
$form.Controls.Add($layout)

function New-Label([string]$text, [System.Drawing.Font]$font = $ui, [System.Drawing.Color]$colour = [System.Drawing.Color]::FromArgb(26, 26, 26)) {
    $label = New-Object System.Windows.Forms.Label
    $label.Text = $text
    $label.Font = $font
    $label.ForeColor = $colour
    $label.AutoSize = $true
    $label.Margin = New-Object System.Windows.Forms.Padding(0, 0, 0, 6)
    $label
}

$title = New-Label 'Add your own word' (New-Object System.Drawing.Font('Segoe UI Semibold', 14))
$layout.Controls.Add($title, 0, 0); $layout.SetColumnSpan($title, 2)
$intro = New-Label 'When you type the spelling, your word comes first in the suggestions.' $ui ([System.Drawing.Color]::FromArgb(96, 96, 96))
$intro.MaximumSize = New-Object System.Drawing.Size(420, 0)
$layout.Controls.Add($intro, 0, 1); $layout.SetColumnSpan($intro, 2)

$layout.Controls.Add((New-Label 'Singkhmer spelling'), 0, 2)
$layout.Controls.Add((New-Label 'Khmer word'), 1, 2)

$spellingBox = New-Object System.Windows.Forms.TextBox
$spellingBox.Font = $khmerFont
$spellingBox.Dock = 'Fill'
$spellingBox.Text = $Spelling.ToLower()
$spellingBox.Margin = New-Object System.Windows.Forms.Padding(0, 0, 8, 0)
$khmerBox = New-Object System.Windows.Forms.TextBox
$khmerBox.Font = $khmerFont
$khmerBox.Dock = 'Fill'
$khmerBox.Margin = New-Object System.Windows.Forms.Padding(0)
$layout.Controls.Add($spellingBox, 0, 3)
$layout.Controls.Add($khmerBox, 1, 3)

$hint = New-Label 'Tip: you can type the Khmer word here in Singkhmer too, then press Space twice.' $ui ([System.Drawing.Color]::FromArgb(112, 112, 112))
$hint.MaximumSize = New-Object System.Drawing.Size(420, 0)
$hint.Margin = New-Object System.Windows.Forms.Padding(0, 6, 0, 6)
$layout.Controls.Add($hint, 0, 4); $layout.SetColumnSpan($hint, 2)

$message = New-Label '' $ui ([System.Drawing.Color]::FromArgb(196, 43, 28))
$layout.Controls.Add($message, 0, 5); $layout.SetColumnSpan($message, 2)

$addButton = New-Object System.Windows.Forms.Button
$addButton.Text = 'Add word'
$addButton.AutoSize = $true
$addButton.FlatStyle = 'Flat'
$addButton.FlatAppearance.BorderSize = 0
$addButton.BackColor = $accent
$addButton.ForeColor = [System.Drawing.Color]::White
$addButton.Padding = New-Object System.Windows.Forms.Padding(14, 4, 14, 4)
$addButton.Margin = New-Object System.Windows.Forms.Padding(0, 0, 0, 14)
$addButton.Cursor = [System.Windows.Forms.Cursors]::Hand
$layout.Controls.Add($addButton, 0, 6)
$form.AcceptButton = $addButton

$listTitle = New-Label 'My words'  (New-Object System.Drawing.Font('Segoe UI Semibold', 11))
$layout.Controls.Add($listTitle, 0, 7); $layout.SetColumnSpan($listTitle, 2)

$list = New-Object System.Windows.Forms.ListView
$list.View = 'Details'
$list.FullRowSelect = $true
$list.HeaderStyle = 'Nonclickable'
$list.Font = $khmerFont
$list.Dock = 'Fill'
$list.BorderStyle = 'FixedSingle'
[void]$list.Columns.Add('Spelling', 180)
[void]$list.Columns.Add('Khmer', 220)
$layout.Controls.Add($list, 0, 8); $layout.SetColumnSpan($list, 2)

$removeButton = New-Object System.Windows.Forms.Button
$removeButton.Text = 'Remove selected'
$removeButton.AutoSize = $true
$removeButton.Margin = New-Object System.Windows.Forms.Padding(0, 8, 0, 0)
$layout.Controls.Add($removeButton, 0, 9)

foreach ($row in 0..9) {
    $style = if ($row -eq 8) { New-Object System.Windows.Forms.RowStyle('Percent', 100) } else { New-Object System.Windows.Forms.RowStyle('AutoSize') }
    [void]$layout.RowStyles.Add($style)
}

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

$addButton.Add_Click({
    $spelling = $spellingBox.Text.Trim().ToLower()
    $khmer = $khmerBox.Text.Trim()
    if ($spelling -notmatch '^[a-z]+$') {
        $message.Text = 'The spelling should be letters a-z only, with no spaces.'
        $spellingBox.Focus(); return
    }
    if (-not (Test-Khmer $khmer)) {
        $message.Text = 'The Khmer word should be Khmer letters only, with no spaces.'
        $khmerBox.Focus(); return
    }
    if ($Words | Where-Object { $_.Spelling -eq $spelling -and $_.Khmer -eq $khmer }) {
        $message.Text = "'$spelling = $khmer' is already in your words."
        return
    }
    # The newest word for a spelling goes first: it is the one the user wants now.
    $Words.Insert(0, [pscustomobject]@{ Spelling = $spelling; Khmer = $khmer })
    try {
        Save-Words $Words
    } catch {
        $Words.RemoveAt(0)
        $message.Text = "Could not save: $($_.Exception.Message)"
        return
    }
    $message.ForeColor = [System.Drawing.Color]::FromArgb(16, 124, 16)
    $message.Text = "Added. Type '$spelling' to get $khmer."
    $spellingBox.Clear(); $khmerBox.Clear(); $spellingBox.Focus()
    Show-Words
})

$removeButton.Add_Click({
    $gone = @($list.SelectedIndices | Sort-Object -Descending)
    if (-not $gone) { return }
    foreach ($index in $gone) { $Words.RemoveAt($index) }
    Save-Words $Words
    $message.ForeColor = [System.Drawing.Color]::FromArgb(96, 96, 96)
    $message.Text = 'Removed.'
    Show-Words
})

$spellingBox.Add_TextChanged({ $message.Text = ''; $message.ForeColor = [System.Drawing.Color]::FromArgb(196, 43, 28) })
$khmerBox.Add_TextChanged({ $message.Text = ''; $message.ForeColor = [System.Drawing.Color]::FromArgb(196, 43, 28) })

$form.Add_Shown({
    $form.TopMost = $false
    $form.Activate()
    if ($spellingBox.Text) { $khmerBox.Focus() } else { $spellingBox.Focus() }
})

Show-Words
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
