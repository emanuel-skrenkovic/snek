// For more comments about what's going on here, check out the `hello_world`
// example.
const rust = import('./pkg')

window.game_over = (score) => {
    console.log('score', score)
}

rust.then(m => {
    m.start()

    window.addEventListener('keypress', m.key_press_event)
}).catch(console.error);

