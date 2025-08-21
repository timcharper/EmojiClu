# Application metadata
app-title = EmojiClu
app-description = Un jeu de puzzle avec des emojis et des indices

# Window and main UI
paused = EN PAUSE
solve-button = Résoudre
show-hint = Afficher l'Indice
hints-label = Indices : 
select-difficulty = Sélectionner la Difficulté

# Menu items
menu-new-game = Nouveau Jeu
menu-restart = Redémarrer
menu-statistics = Statistiques
menu-seed = Graine
menu-about = À propos

# Settings menu
settings-show-clue-tooltips = Afficher les Infobulles des Indices
settings-touch-screen-controls = Contrôles d'Écran Tactile

# Buttons
submit = Soumettre
submit-solution = Soumettre la Solution ?
submit-puzzle-solution = Soumettre la solution du puzzle
go-back = Retour
ok = OK
cancel = Annuler
close = Fermer

# Dialogs
game-seed = Graine du Jeu
game-statistics = Statistiques du Jeu
best-times = Meilleurs Temps
global-statistics = Statistiques Globales

# About dialog
about-author = Tim Harper
about-website-label = Dépôt GitHub
about-website = https://github.com/timcharper/emojiclu

# Stats dialog headers
stats-rank = Rang
stats-time = Temps
stats-hints = Indices
stats-grid-size = Taille de la Grille
stats-difficulty = Difficulté
stats-date = Date
stats-unknown = Inconnu

# Timer
timer-pause = ⏸︎
timer-pause-tooltip = Mettre en Pause (Espace)

# Tutorial messages
tutorial-welcome = <b>Bienvenue dans EmojiClu</b>, un jeu de puzzle de déduction logique.
    
    Au-dessus de ce texte se trouve la grille de puzzle, à droite et en bas se trouvent les indices. Votre objectif est de découvrir l'emplacement de diverses tuiles en faisant des déductions avec les indices.
    
    D'abord, commençons par utiliser le système d'indices. Appuyez sur le bouton {"{"}icon:view-reveal-symbolic{"}"} (dans le coin supérieur gauche) maintenant.

tutorial-phase2 = Parfait ! Le jeu a sélectionné et mis en évidence un indice que vous devriez examiner.
    
    <b>Survolez l'indice sélectionné</b> pour voir une info-bulle expliquant ce que signifie l'indice.
    
    Appuyer sur {"{"}icon:view-reveal-symbolic{"}"} une seconde fois vous donne une aide supplémentaire.
    
    Appuyez sur le bouton {"{"}icon:view-reveal-symbolic{"}"} une seconde fois, maintenant.

tutorial-phase3-prefix = La deuxième fois que nous avons appuyé sur le bouton d'indice, le jeu a mis en évidence une tuile qui est l'une des déductions que vous pouvez faire à partir de l'indice.
    
    Nous pouvons déduire ici de l'indice que la tuile {"{"}tile:{$tile}{"}"} dans la colonne {$column} devrait être {$action}.

tutorial-phase3-action = {$control_text} la tuile {"{"}tile:{$tile}{"}"} dans la colonne {$column} maintenant.

tutorial-phase3-oops = Oups ! Ce n'était pas tout à fait correct. La tuile {"{"}tile:{$tile}{"}"} dans la colonne {$column} n'est pas {$action}.
    
    Appuyez sur le bouton {"{"}icon:edit-undo-symbolic{"}"} de manière répétée jusqu'à ce qu'aucune autre annulation ne soit possible.

tutorial-undo = Parfait !
    
    Maintenant, à tout moment, vous pouvez annuler tous les mouvements que vous faites avec le bouton d'annulation, ou en appuyant sur <tt>Ctrl+Z</tt>.
    
    Remettons le jeu au début. Appuyez sur le bouton {"{"}icon:edit-undo-symbolic{"}"} de manière répétée jusqu'à ce qu'aucune autre annulation ne soit possible.

tutorial-select-clue = Parfait ! Maintenant, utilisons le système de sélection d'indices.
    
    Sélectionner un indice vous aide à suivre ce sur quoi vous travaillez actuellement. Vous pouvez sélectionner un indice soit en cliquant dessus, soit en naviguant vers lui en utilisant les touches <tt>A</tt> ou <tt>D</tt>.
    
    Sélectionnons un indice maintenant.

# Play to end messages
tutorial-mistake = <b>Oups !</b> Vous avez fait une erreur. Essayons encore.
    
    Appuyez sur le bouton {"{"}icon:edit-undo-symbolic{"}"}.

tutorial-congratulations = <b>Félicitations !</b>
    
    Vous avez terminé le tutoriel ! Vous pouvez essayer un puzzle facile en sélectionnant <tt>'Facile'</tt> du sélecteur de difficulté en haut à gauche.
    
    Ou, appuyez sur <tt>Ctrl+N</tt> pour redémarrer ce tutoriel.

tutorial-next-clue = Passons au prochain indice.

tutorial-clue-complete = <b>Indice terminé !</b>
    
    Cet indice est entièrement encodé dans le plateau. Marquez-le comme terminé en appuyant sur <tt>'C'</tt>, ou en {$action} l'indice.

tutorial-no-deduction = Nous ne pouvons rien déduire de plus de cet indice pour le moment, <i>mais il n'est pas terminé</i>. Passez au prochain indice.

tutorial-keep-going = Continuons. Sélectionnez un indice.

tutorial-clue-analysis = <big>{$clue_title}</big> :
    
    {$clue_description}
    
    Donc, {"{"}tile:{$tile}{"}"} <b>{$must_be}</b> dans la colonne <big><tt>{$column}</tt></big>{$converging_note}.

# Control text
control-long-press = appui long
control-left-click = clic gauche
control-tap = toucher
control-right-click = clic droit

# Action words
action-selected = sélectionné
action-eliminated = éliminé
action-must-be = doit être
action-cannot-be = ne peut pas être

# Converging deduction note
converging-note = (<i>toutes les solutions possibles pour cet indice se chevauchent cette cellule, donc elle ne peut être qu'une des valeurs de l'indice</i>)

# Clue type titles
clue-title-three-adjacent = Trois Adjacentes
clue-title-two-apart-not-middle = Deux Séparées, Mais Pas Au Milieu
clue-title-left-of = À Gauche De
clue-title-two-adjacent = Deux Adjacentes
clue-title-not-adjacent = Non Adjacentes
clue-title-all-in-column = Toutes En Colonne
clue-title-two-in-column = Deux En Colonne
clue-title-one-matches-either = Une Correspond À L'Une Ou L'Autre
clue-title-not-in-same-column = Pas Dans La Même Colonne
clue-title-two-in-column-one-not = Deux En Colonne, Une Pas

# Clue descriptions
clue-desc-adjacent = {$tiles} sont adjacentes (dans les deux directions).
clue-desc-two-adjacent = {"{"}tile:{$tile1}{"}"} est à côté de {"{"}tile:{$tile2}{"}"} (dans les deux directions).
clue-desc-two-apart = {"{"}tile:{$tile1}{"}"} est à deux de distance de {"{"}tile:{$tile3}{"}"}, sans {"{"}tile:{$tile2}{"}"} au milieu (dans les deux directions).
clue-desc-left-of = {"{"}tile:{$left}{"}"} est à gauche de {"{"}tile:{$right}{"}"} (n'importe quel nombre de tuiles entre).
clue-desc-not-adjacent = {"{"}tile:{$tile1}{"}"} n'est pas à côté de {"{"}tile:{$tile2}{"}"} (dans les deux directions).
clue-desc-same-column = {$tiles} sont dans la même colonne.
clue-desc-two-in-column-without = {"{"}tile:{$tile1}{"}"} et {"{"}tile:{$tile2}{"}"} sont dans la même colonne, mais {"{"}tile:{$tile3}{"}"} ne l'est pas.
clue-desc-not-same-column = {"{"}tile:{$tile1}{"}"} n'est pas dans la même colonne que {"{"}tile:{$tile2}{"}"}
clue-desc-one-matches-either = {"{"}tile:{$tile1}{"}"} est soit dans la même colonne que {"{"}tile:{$tile2}{"}"} ou {"{"}tile:{$tile3}{"}"}, mais pas les deux.

# Difficulty levels
difficulty-tutorial = Tutoriel
difficulty-easy = Facile
difficulty-moderate = Modéré
difficulty-hard = Difficile
difficulty-veteran = Vétéran

# Debug/Development
destroying-window = Destruction de la fenêtre
weird = Bizarre
