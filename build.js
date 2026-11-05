// Собирает единый index.html и раскладывает его по платформам.
const fs = require('fs');
const path = require('path');

const core = fs.readFileSync('src/core.js', 'utf8');
const html = fs.readFileSync('src/index.template.html', 'utf8').replace('/*CORE*/', () => core);

for (const dir of ['dist', 'desktop', 'mobile/www']) {
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'index.html'), html);
}
fs.mkdirSync('mobile/assets', { recursive: true });
for (const f of ['icon-only.png', 'icon-foreground.png', 'icon-background.png']) {
  if (fs.existsSync('assets/' + f)) fs.copyFileSync('assets/' + f, 'mobile/assets/' + f);
}
console.log('index.html →', ['dist', 'desktop', 'mobile/www'].join(', '), `(${(html.length / 1024).toFixed(0)} KB)`);
