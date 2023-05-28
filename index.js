// For more comments about what's going on here, check out the `hello_world`
// example.
const rust = import('./pkg')

rust.then(m => {
    m.start()

    window.addEventListener('keydown', m.key_down_event)
    window.addEventListener('keyup', m.key_up_event)
    window.addEventListener('keypress', m.key_press_event)
}).catch(console.error);

