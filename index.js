const rust = import('./pkg')

const save_score = (body) => fetch('/scores', {
    method: 'POST',
    body: JSON.stringify(body),
    headers: {'Content-Type': 'application/json'}
})

const get_scores = async () => {
    const response = await fetch('/scores', { method: 'GET' })
    return await response.json()
}

window.game_over = async (score) => {
    await save_score({ name: 'test', score: score })
    const scores = await get_scores()

    const high_scores_list = scores.map(({ name, score }) => `
    <tr>
        <td>${name}</td>
        <td>${score}</td>
    </tr>`).reduce((agg, current) => `${agg}${current}`)

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
    ${table_body}`;
}

window.pause = () => overlay().innerText = 'PAUSED'
window.clear_screen = () => overlay().innerText = ''

window.scored = (score) => {
    overlay().innerHTML = `Score: ${score}`
}

rust.then(m => {
    window.addEventListener('keypress', m.key_press_event)
    overlay().innerHTML = `Score: 0`
    m.start()
}).catch(console.error);

const overlay = () => document.querySelector('#overlay')

