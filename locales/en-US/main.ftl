# Application metadata
app-title = EmojiClu
app-description = A puzzle game with emojis and clues

# Window and main UI
paused = PAUSED
solve-button = Solve
show-hint = Show Hint
hints-label = Hints: 
select-difficulty = Select Difficulty

# Menu items
menu-new-game = New Game
menu-restart = Restart
menu-statistics = Statistics
menu-seed = Seed
menu-about = About

# Settings menu
settings-show-clue-tooltips = Show Clue Tooltips
settings-touch-screen-controls = Touch Screen Controls

# Buttons
submit = Submit
submit-solution = Submit Solution?
submit-puzzle-solution = Submit puzzle solution
go-back = Go Back
ok = OK
cancel = Cancel
close = Close

# Dialogs
game-seed = Game Seed
game-statistics = Game Statistics
best-times = Best Times
global-statistics = Global Statistics

# About dialog
about-author = Tim Harper
about-website-label = GitHub Repository
about-website = https://github.com/timcharper/emojiclu

# Stats dialog headers
stats-rank = Rank
stats-time = Time
stats-hints = Hints
stats-grid-size = Grid Size
stats-difficulty = Difficulty
stats-date = Date
stats-unknown = Unknown

# Timer
timer-pause = ⏸︎
timer-pause-tooltip = Pause Game (Space)

# Tutorial messages
tutorial-welcome = <b>Welcome to EmojiClu</b>, a logical deduction puzzle game.
    
    Above this text is the puzzle grid, to the right and bottom are clues. Your goal is to figure out the location of various tiles making deductions with the clues.
    
    First, let's start off using the hint system. Press the {"{"}icon:view-reveal-symbolic{"}"} button (in the top-right corner) now.

tutorial-phase2 = Great! The game selected and highlighted a clue you should look at.
    
    <b>Hover over the selected clue</b> to see a tooltip explaining what the clue means.
    
    Pressing {"{"}icon:view-reveal-symbolic{"}"} a second time gives you additional help.
    
    Press the {"{"}icon:view-reveal-symbolic{"}"} button a second time, now.

tutorial-phase3-prefix = The second time we pressed the hint button, the game highlighted a tile that is one of the deductions you can make from the clue.
    
    We can deduce here from the clue that tile {"{"}tile:{$tile}{"}"} in column {$column} should be {$action}.

tutorial-phase3-action = {$control_text} the tile {"{"}tile:{$tile}{"}"} in column {$column} now.

tutorial-phase3-oops = Oops! That wasn't quite right. Tile {"{"}tile:{$tile}{"}"} in column {$column} is not {$action}.
    
    Press the {"{"}icon:edit-undo-symbolic{"}"} button repeatedly until no further undos are possible.

tutorial-undo = Great!
    
    Now, at any time, you can undo any moves you make with the undo button, or by pressing <tt>Ctrl+Z</tt>.
    
    Let's get the game back to the start. Press the {"{"}icon:edit-undo-symbolic{"}"} button repeatedly until no further undos are possible.

tutorial-select-clue = Great! Now, let's use the clue selection system.
    
    Selecting a clue helps you track what you're currently working on. You can select a clue either by clicking on it, or navigating to it using the keys <tt>A</tt> or <tt>D</tt>.
    
    Let's select a clue now.

# Play to end messages
tutorial-mistake = <b>Oops!</b> You've made a mistake. Let's try again.
    
    Press the {"{"}icon:edit-undo-symbolic{"}"} button.

tutorial-congratulations = <b>Congratulations!</b>
    
    You've completed the tutorial! You can try an easy puzzle by selecting <tt>'Easy'</tt> from the top-left difficulty selector.
    
    Or, press <tt>Ctrl+N</tt> to restart this tutorial.

tutorial-next-clue = Let's move on to the next clue.

tutorial-clue-complete = <b>Clue complete!</b>
    
    This clue is fully encoded in the board. Mark it as completed by pressing <tt>'C'</tt>, or by {$action}ing the clue.

tutorial-no-deduction = We can't deduce anything more from this clue at this time, <i>but it is not complete</i>. Move on to next clue.

tutorial-keep-going = Let's keep going. Select a clue.

tutorial-clue-analysis = <big>{$clue_title}</big>:
    
    {$clue_description}
    
    So, {"{"}tile:{$tile}{"}"} <b>{$must_be}</b> in column <big><tt>{$column}</tt></big>{$converging_note}.

# Control text
control-long-press = long press
control-left-click = left click
control-tap = tap
control-right-click = right click

# Action words
action-selected = selected
action-eliminated = eliminated
action-must-be = must be
action-cannot-be = cannot be

# Converging deduction note
converging-note = (<i>all possible solutions for this clue overlap this cell, so it can only be one of the clue values</i>)

# Clue type titles
clue-title-three-adjacent = Three Adjacent
clue-title-two-apart-not-middle = Two Apart, But Not The Middle
clue-title-left-of = Left Of
clue-title-two-adjacent = Two Adjacent
clue-title-not-adjacent = Not Adjacent
clue-title-all-in-column = All In Column
clue-title-two-in-column = Two In Column
clue-title-one-matches-either = One Matches Either
clue-title-not-in-same-column = Not In Same Column
clue-title-two-in-column-one-not = Two In Column, One Not

# Clue descriptions
clue-desc-adjacent = {$tiles} are adjacent (in either direction).
clue-desc-two-adjacent = {"{"}tile:{$tile1}{"}"} is next to {"{"}tile:{$tile2}{"}"} (in either direction).
clue-desc-two-apart = {"{"}tile:{$tile1}{"}"} is two away from {"{"}tile:{$tile3}{"}"}, without {"{"}tile:{$tile2}{"}"} in the middle (in either direction).
clue-desc-left-of = {"{"}tile:{$left}{"}"} is left of {"{"}tile:{$right}{"}"} (any number of tiles in between).
clue-desc-not-adjacent = {"{"}tile:{$tile1}{"}"} is not next to {"{"}tile:{$tile2}{"}"} (in either direction).
clue-desc-same-column = {$tiles} are in the same column.
clue-desc-two-in-column-without = {"{"}tile:{$tile1}{"}"} and {"{"}tile:{$tile2}{"}"} are in the same column, but {"{"}tile:{$tile3}{"}"} isn't.
clue-desc-not-same-column = {"{"}tile:{$tile1}{"}"} is not in the same column as {"{"}tile:{$tile2}{"}"}
clue-desc-one-matches-either = {"{"}tile:{$tile1}{"}"} is either in the same column as {"{"}tile:{$tile2}{"}"} or {"{"}tile:{$tile3}{"}"}, but not both.

# Debug/Development
destroying-window = Destroying window
weird = Weird
