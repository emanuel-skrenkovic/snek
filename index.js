const rust = import('./pkg')

const name_input = (score) =>
    overlay().innerHTML = `
        <label>Score: ${score}</label>
        <br>
        Press space to start again!`;


window.scored       = (score) => overlay().innerHTML = `Score: ${score}`
window.game_over    = (score) => name_input(score)
window.pause        = () => overlay().innerText = 'PAUSED'
window.clear_screen = () => overlay().innerText = ''

rust.then(m => {
    window.addEventListener('keydown', m.key_press_event)

    overlay().innerHTML = `
        Press space to start or pause the game.
        <br>
        Control the snek using WASD or arrow keys.`

    m.start()
}).catch(console.error);

const overlay = () => document.querySelector('#overlay')
