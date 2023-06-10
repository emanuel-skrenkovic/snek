const express = require('express');
const app = express();
const port = 8000;

app.use(express.json())

app.use(
    express.static('dist', {
        setHeaders: function (res, path, stat) {
            res.set('Cross-Origin-Embedder-Policy', 'require-corp');
            res.set('Cross-Origin-Opener-Policy', 'same-origin');
        },
    })
);

const sqlite3 = require('sqlite3').verbose()
const DB_SOURCE = 'db.sqlite'

const db = new sqlite3.Database(DB_SOURCE, (err) => {
    if (err) {
        console.error(err.message)
        throw err
    }

    db.run(
        `CREATE TABLE IF NOT EXISTS score (
            id INTEGER PRIMARY KEY, 
            name TEXT, 
            score INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )`,
        (err, _) => { if (err) console.error(err) }
    );
});

app.post('/scores', function(req, res) {
    const { name, score } = req.body

    const query  = 'INSERT INTO score (name, score, created_at) VALUES (?, ?, ?)'
    const params = [name, score, new Date().toUTCString()]
    db.run(query, params, (err, _) => {
        if (err) {
            res.status(500).json({'error': err.message})
            return
        }

        res.json()
    })
})

app.get('/scores', function(req, res) {
    const query = `
        SELECT 
            name, score 
        FROM score 
        ORDER BY 
            score DESC, 
            created_at ASC
        LIMIT 10`

    db.all(query, (err, rows) => {
        if (err) {
            res.status(400).json({'error': err.message})
            return
        }

        res.json(rows)
    })
})

app.listen(port, () => {
    console.log(`Listening on port ${port}`);
});
