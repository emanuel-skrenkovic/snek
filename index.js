// For more comments about what's going on here, check out the `hello_world`
// example.
const rust = import('./pkg')

const overlay = () => document.querySelector('#overlay')

window.game_over = (score) => {
    overlay().innerText = `You scored: ${score}`;
}

window.pause = () => {
    overlay().innerText = 'PAUSED'
}

window.clear_screen = () => {
    overlay().innerText = ''
}

rust.then(m => {
    m.start()

    window.addEventListener('keypress', m.key_press_event)
}).catch(console.error);

