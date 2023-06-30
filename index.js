const rust = import('./pkg')

const name_input = (score) => {
    let name;

    window.on_name_changed = e => {
        if (e.currentTarget.value.length > 3) return
        name = e.currentTarget.value.toUpperCase()
    }

    window.on_submit_score = async e => {
        e.preventDefault()
        if (name.length < 3) return

        await save_score({ name, score })
        await on_save(score)
    }

    overlay().innerHTML = `
    <form onsubmit="on_submit_score(event)">
        <label>Score: ${score}</label>
        <input id="score-input" type="text" autofocus maxlength="3" onchange="on_name_changed(event)" />
        <div>
            <button id="save-button" onclick="on_submit_score(event)">Save</button>\
        </div>
    </form>`

    const save_button_style = document.querySelector('#save-button').style
    save_button_style.border          = 'none'
    save_button_style.borderRadius    = '15px'
    save_button_style.fontSize        = '32px'
    save_button_style.backgroundColor = '#5cdb5c'
}

const save_score = (body) => fetch('/scores', {
    method: 'POST',
    body: JSON.stringify(body),
    headers: {'Content-Type': 'application/json'}
})

const get_scores = async () => {
    const response = await fetch('/scores', { method: 'GET' })
    return await response.json()
}

const on_save = async (score) => {
    const scores = await get_scores()

    const high_scores_list = scores.map(({ name, score }) => `
        <tr>
            <td>${name}</td>
            <td>${score}</td>
        </tr>`
    ).reduce((agg, current) => `${agg}${current}`)

    const table_body = `
        <table class="high-score-list">
            <thead>
            <tr>
                <th></th>
                <th></th>
            </tr>
            </thead>
            <tbody>
                ${high_scores_list}
            </tbody>
        </table>`

    overlay().innerHTML = `
        You scored: ${score}
        <br>
        High scores:
        <br>
        ${table_body}
        
        <div id="again">
            <h2>Press space to start again.</h2>
        </div>`;

    const again_style = document.querySelector('#again').style
    again_style.color     = 'white'
    again_style.position  = 'relative'
    again_style.top       = '50%'
    again_style.left      = '50%'
    again_style.transform = 'translate(50%, 50%)'
}

window.scored       = (score) => overlay().innerHTML = `Score: ${score}`
window.game_over    = (score) => name_input(score)
window.pause        = () => overlay().innerText = 'PAUSED'
window.clear_screen = () => overlay().innerText = ''

rust.then(m => {
    window.addEventListener('keydown', m.key_press_event)

    overlay().innerHTML = `
        Can you solve
        <h2>the_snack_case!?!?!</h2>
        
        Press space to start the game.
        Control the snek using WASD or arrow keys.
    
        <p>
            <strong>Minimum system requirements:</strong>
            <br>
            - Apple Mac M2 Pro
        </p>`

    m.start()
}).catch(console.error);

const overlay = () => document.querySelector('#overlay')
