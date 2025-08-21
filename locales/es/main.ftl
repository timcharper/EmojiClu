# Application metadata
app-title = EmojiClu
app-description = Un juego de rompecabezas con emojis y pistas

# Window and main UI
paused = PAUSADO
solve-button = Resolver
show-hint = Mostrar Pista
hints-label = Pistas: 
select-difficulty = Seleccionar Dificultad

# Menu items
menu-new-game = Nuevo Juego
menu-restart = Reiniciar
menu-statistics = Estadísticas
menu-seed = Semilla
menu-about = Acerca de

# Settings menu
settings-show-clue-tooltips = Mostrar Tooltips de Pistas
settings-touch-screen-controls = Controles de Pantalla Táctil

# Buttons
submit = Enviar
submit-solution = ¿Enviar Solución?
submit-puzzle-solution = Enviar solución del rompecabezas
go-back = Volver
ok = OK
cancel = Cancelar
close = Cerrar

# Dialogs
game-seed = Semilla del Juego
game-statistics = Estadísticas del Juego
best-times = Mejores Tiempos
global-statistics = Estadísticas Globales

# About dialog
about-author = Tim Harper
about-website-label = Repositorio de GitHub
about-website = https://github.com/timcharper/emojiclu

# Stats dialog headers
stats-rank = Rango
stats-time = Tiempo
stats-hints = Pistas
stats-grid-size = Tamaño de Cuadrícula
stats-difficulty = Dificultad
stats-date = Fecha
stats-unknown = Desconocido

# Timer
timer-pause = ⏸︎
timer-pause-tooltip = Pausar Juego (Espacio)

# Tutorial messages
tutorial-welcome = <b>¡Bienvenido a EmojiClu</b>, un juego de rompecabezas de deducción lógica.
    
    Encima de este texto está la cuadrícula del rompecabezas, a la derecha y abajo están las pistas. Tu objetivo es descubrir la ubicación de varios mosaicos haciendo deducciones con las pistas.
    
    Primero, comencemos usando el sistema de pistas. Presiona el botón {"{"}icon:view-reveal-symbolic{"}"} (en la esquina superior izquierda) ahora.

tutorial-phase2 = ¡Excelente! El juego seleccionó y resaltó una pista que deberías mirar.
    
    <b>Pasa el cursor sobre la pista seleccionada</b> para ver una descripción que explica qué significa la pista.
    
    Presionar {"{"}icon:view-reveal-symbolic{"}"} una segunda vez te da ayuda adicional.
    
    Presiona el botón {"{"}icon:view-reveal-symbolic{"}"} una segunda vez, ahora.

tutorial-phase3-prefix = La segunda vez que presionamos el botón de pista, el juego resaltó un mosaico que es una de las deducciones que puedes hacer de la pista.
    
    Podemos deducir aquí de la pista que el mosaico {"{"}tile:{$tile}{"}"} en la columna {$column} debería estar {$action}.

tutorial-phase3-action = {$control_text} el mosaico {"{"}tile:{$tile}{"}"} en la columna {$column} ahora.

tutorial-phase3-oops = ¡Ups! Eso no estuvo del todo bien. El mosaico {"{"}tile:{$tile}{"}"} en la columna {$column} no está {$action}.
    
    Presiona el botón {"{"}icon:edit-undo-symbolic{"}"} repetidamente hasta que no sean posibles más deshacer.

tutorial-undo = ¡Excelente!
    
    Ahora, en cualquier momento, puedes deshacer cualquier movimiento que hagas con el botón deshacer, o presionando <tt>Ctrl+Z</tt>.
    
    Regresemos el juego al inicio. Presiona el botón {"{"}icon:edit-undo-symbolic{"}"} repetidamente hasta que no sean posibles más deshacer.

tutorial-select-clue = ¡Excelente! Ahora, usemos el sistema de selección de pistas.
    
    Seleccionar una pista te ayuda a seguir en lo que estás trabajando actualmente. Puedes seleccionar una pista haciendo clic en ella, o navegando hacia ella usando las teclas <tt>A</tt> o <tt>D</tt>.
    
    Seleccionemos una pista ahora.

# Play to end messages
tutorial-mistake = <b>¡Ups!</b> Has cometido un error. Intentemos de nuevo.
    
    Presiona el botón {"{"}icon:edit-undo-symbolic{"}"}.

tutorial-congratulations = <b>¡Felicitaciones!</b>
    
    ¡Has completado el tutorial! Puedes probar un rompecabezas fácil seleccionando <tt>'Fácil'</tt> del selector de dificultad en la esquina superior izquierda.
    
    O, presiona <tt>Ctrl+N</tt> para reiniciar este tutorial.

tutorial-next-clue = Pasemos a la siguiente pista.

tutorial-clue-complete = <b>¡Pista completa!</b>
    
    Esta pista está completamente codificada en el tablero. Márcala como completada presionando <tt>'C'</tt>, o haciendo {$action} en la pista.

tutorial-no-deduction = No podemos deducir nada más de esta pista en este momento, <i>pero no está completa</i>. Pasa a la siguiente pista.

tutorial-keep-going = Sigamos adelante. Selecciona una pista.

tutorial-clue-analysis = <big>{$clue_title}</big>:
    
    {$clue_description}
    
    Entonces, {"{"}tile:{$tile}{"}"} <b>{$must_be}</b> en la columna <big><tt>{$column}</tt></big>{$converging_note}.

# Control text
control-long-press = presión larga
control-left-click = clic izquierdo
control-tap = tocar
control-right-click = clic derecho

# Action words
action-selected = seleccionado
action-eliminated = eliminado
action-must-be = debe estar
action-cannot-be = no puede estar

# Converging deduction note
converging-note = (<i>todas las soluciones posibles para esta pista se superponen en esta celda, por lo que solo puede ser uno de los valores de la pista</i>)

# Clue type titles
clue-title-three-adjacent = Tres Adyacentes
clue-title-two-apart-not-middle = Dos Separadas, Pero No En El Medio
clue-title-left-of = A La Izquierda De
clue-title-two-adjacent = Dos Adyacentes
clue-title-not-adjacent = No Adyacentes
clue-title-all-in-column = Todas En Columna
clue-title-two-in-column = Dos En Columna
clue-title-one-matches-either = Una Coincide Con Cualquiera
clue-title-not-in-same-column = No En La Misma Columna
clue-title-two-in-column-one-not = Dos En Columna, Una No

# Clue descriptions
clue-desc-adjacent = {$tiles} son adyacentes (en cualquier dirección).
clue-desc-two-adjacent = {"{"}tile:{$tile1}{"}"} está junto a {"{"}tile:{$tile2}{"}"} (en cualquier dirección).
clue-desc-two-apart = {"{"}tile:{$tile1}{"}"} está a dos de distancia de {"{"}tile:{$tile3}{"}"}, sin {"{"}tile:{$tile2}{"}"} en el medio (en cualquier dirección).
clue-desc-left-of = {"{"}tile:{$left}{"}"} está a la izquierda de {"{"}tile:{$right}{"}"} (cualquier número de fichas en el medio).
clue-desc-not-adjacent = {"{"}tile:{$tile1}{"}"} no está junto a {"{"}tile:{$tile2}{"}"} (en cualquier dirección).
clue-desc-same-column = {$tiles} están en la misma columna.
clue-desc-two-in-column-without = {"{"}tile:{$tile1}{"}"} y {"{"}tile:{$tile2}{"}"} están en la misma columna, pero {"{"}tile:{$tile3}{"}"} no.
clue-desc-not-same-column = {"{"}tile:{$tile1}{"}"} no está en la misma columna que {"{"}tile:{$tile2}{"}"}
clue-desc-one-matches-either = {"{"}tile:{$tile1}{"}"} está en la misma columna que {"{"}tile:{$tile2}{"}"} o {"{"}tile:{$tile3}{"}"}, pero no ambas.

# Difficulty levels
difficulty-tutorial = Tutorial
difficulty-easy = Fácil
difficulty-moderate = Moderado
difficulty-hard = Difícil
difficulty-veteran = Veterano

# Debug/Development
destroying-window = Destruyendo ventana
weird = Extraño
