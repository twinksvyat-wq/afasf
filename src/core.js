/* Сетка — ядро: разбор текста, раскладка OCR, генерация .xlsx (без зависимостей) */
(function (root) {
  'use strict';

  /* ---------- Разбор текста ---------- */

  const DELIMS = { tab: '\t', semicolon: ';', comma: ',', pipe: '|', spaces: '  ' };

  function splitQuoted(line, d) {
    const out = []; let cur = ''; let q = false;
    for (let i = 0; i < line.length; i++) {
      const c = line[i];
      if (q) {
        if (c === '"') { if (line[i + 1] === '"') { cur += '"'; i++; } else q = false; }
        else cur += c;
      } else if (c === '"' && cur.trim() === '') { q = true; cur = ''; }
      else if (c === d) { out.push(cur); cur = ''; }
      else cur += c;
    }
    out.push(cur);
    return out.map(s => s.trim());
  }

  function splitLine(line, kind) {
    if (kind === 'spaces') return line.trim().split(/\s{2,}|\t/).map(s => s.trim());
    if (kind === 'single') return line.trim().split(/\s+/);
    if (kind === 'pipe') {
      let l = line.trim();
      if (l.startsWith('|')) l = l.slice(1);
      if (l.endsWith('|')) l = l.slice(0, -1);
      return l.split('|').map(s => s.trim());
    }
    return splitQuoted(line, DELIMS[kind]);
  }

  function countOutsideQuotes(line, ch) {
    let n = 0, q = false;
    for (const c of line) { if (c === '"') q = !q; else if (!q && c === ch) n++; }
    return n;
  }

  function detectDelimiter(lines) {
    const sample = lines.slice(0, 50);
    const score = {};
    for (const kind of ['tab', 'pipe', 'semicolon', 'comma']) {
      const counts = sample.map(l => countOutsideQuotes(l, DELIMS[kind]));
      const nonZero = counts.filter(c => c > 0);
      if (nonZero.length < Math.max(1, sample.length * 0.6)) continue;
      const freq = {}; nonZero.forEach(c => freq[c] = (freq[c] || 0) + 1);
      const mode = +Object.keys(freq).sort((a, b) => freq[b] - freq[a])[0];
      const consistency = freq[mode] / sample.length;
      score[kind] = consistency * 10 + Math.min(mode, 5) * 0.1 + (kind === 'tab' ? 0.5 : kind === 'pipe' ? 0.4 : kind === 'semicolon' ? 0.3 : 0);
    }
    const sp = sample.filter(l => /\S\s{2,}\S/.test(l.trim())).length;
    if (sp >= Math.max(1, sample.length * 0.6)) score.spaces = (sp / sample.length) * 9;
    const best = Object.keys(score).sort((a, b) => score[b] - score[a])[0];
    if (best) return best;
    // одиночные пробелы — только если число слов в строках совпадает
    const wc = sample.map(l => l.trim().split(/\s+/).length);
    if (wc.length > 1 && wc.every(n => n === wc[0]) && wc[0] > 1) return 'single';
    return 'none';
  }

  /* ---------- Свободный текст: записи, счёт «1-3», списки ---------- */
  const PAIR_RE = /(\d+(?:[.,]\d+)?)\s*([-–—:])\s*(\d+(?:[.,]\d+)?)/g;
  function findPairs(line) {
    const out = []; PAIR_RE.lastIndex = 0; let m;
    while ((m = PAIR_RE.exec(line))) {
      const start = m.index, end = start + m[0].length;
      const prev = line[start - 1] || '', next = line.slice(end, end + 2);
      if (/[\d.,:\/–—-]/.test(prev)) continue;              // часть даты/диапазона подлиннее
      if (/^[.:\/–—-]\d/.test(next)) continue;              // 01-02-2026, 12:30:45
      if (m[2] === ':' && m[3].length === 2 && m[1].length <= 2 && +m[1] <= 23 &&
          (m[3][0] === '0' || /^[1-5][05]$/.test(m[3]))) continue;  // похоже на время: 9:05, 12:30
      out.push({ start, end, a: m[1], b: m[3], raw: m[0].replace(/\s+/g, '') });
    }
    return out;
  }
  const hasPair = l => findPairs(l).length > 0;

  function splitPlain(line) {
    const l = line.trim();
    if (/\t/.test(l)) return l.split('\t').map(s => s.trim());
    if (/\s{2,}/.test(l)) return l.split(/\s{2,}/).map(s => s.trim());
    if (/;/.test(l)) return l.split(';').map(s => s.trim());
    if (/\|/.test(l)) return splitLine(l, 'pipe');
    return [l];
  }
  const cleanEdge = s => s.replace(/^[\s,;:|·•—–-]+|[\s,;:|·•—–=-]+$/g, '').trim();

  function structureFree(text, opts) {
    const split = !opts || opts.splitPairs !== false;
    const lines = String(text).replace(/\r\n?/g, '\n').split('\n').map(l => l.replace(/^\s*[•*·▪]\s*/, '').trim()).filter(Boolean);
    const rows = []; // {cells, lead}
    for (const line of lines) {
      const pairs = findPairs(line);
      if (!pairs.length) { rows.push({ cells: splitPlain(line), lead: true }); continue; }
      let residual = ''; let pos = 0;
      for (const p of pairs) { residual += line.slice(pos, p.start); pos = p.end; }
      residual += line.slice(pos);
      if (!residual.replace(/[\s,;|/]+/g, '')) {          // строка только из пар: 1-3 4-3 2-2
        for (const p of pairs) rows.push({ cells: split ? [p.a, p.b] : [p.raw], lead: false });
        continue;
      }
      // несколько записей в строке: «Иванов 4-3, Петров 1-0» → по строке на запись
      if (pairs.length > 1) {
        const segs = pairs.map((p, i) => cleanEdge(line.slice(i ? pairs[i - 1].end : 0, p.start)));
        if (segs.every(Boolean) && !cleanEdge(line.slice(pairs[pairs.length - 1].end))) {
          pairs.forEach((p, i) => rows.push({ cells: [segs[i]].concat(split ? [p.a, p.b] : [p.raw]), lead: true }));
          continue;
        }
      }
      const cells = []; pos = 0;
      for (const p of pairs) {
        const t = cleanEdge(line.slice(pos, p.start)); if (t) cells.push(t.replace(/^(\d+)[.)]$/, '$1'));
        if (split) cells.push(p.a, p.b); else cells.push(p.raw);
        pos = p.end;
      }
      const tail = cleanEdge(line.slice(pos)); if (tail) cells.push(tail);
      rows.push({ cells, lead: cleanEdge(line.slice(0, pairs[0].start)) !== '' });
    }
    // выравнивание: если у части строк перед счётом есть подпись, у строк «только счёт» оставляем первый столбец пустым
    const anyLead = rows.some(r => r.lead && r.cells.length > (split ? 2 : 1));
    return normalize(rows.map(r => (!r.lead && anyLead) ? [''].concat(r.cells) : r.cells));
  }

  function isMdSeparator(line) { return /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/.test(line); }

  function parseText(text, kind, opts) {
    kind = kind || 'auto';
    const lines = String(text).replace(/\r\n?/g, '\n').split('\n').filter(l => l.trim() !== '' && !isMdSeparator(l));
    if (!lines.length) return { rows: [], delimiter: kind };
    let used = kind === 'auto' ? detectDelimiter(lines) : kind;
    if (kind === 'auto' && !['tab', 'pipe', 'semicolon'].includes(used) && lines.filter(hasPair).length >= lines.length * 0.4) used = 'free';
    if (used === 'free') return { rows: structureFree(lines.join('\n'), opts), delimiter: 'free' };
    const rows = lines.map(l => used === 'none' ? [l.trim()] : splitLine(l, used));
    return { rows: normalize(rows), delimiter: used };
  }

  function normalize(rows) {
    rows = rows.map(r => (Array.isArray(r) ? r : [r]).map(c => c == null ? '' : String(c).trim()));
    const w = Math.max(1, ...rows.map(r => r.length));
    rows = rows.map(r => { while (r.length < w) r.push(''); return r; });
    // убрать полностью пустые столбцы
    const keep = [];
    for (let j = 0; j < w; j++) if (rows.some(r => r[j] !== '')) keep.push(j);
    return rows.map(r => keep.map(j => r[j])).filter(r => r.some(c => c !== ''));
  }

  /* ---------- Раскладка слов OCR в таблицу ---------- */
  // lines: [{ words: [{ text, bbox:{x0,y0,x1,y1} }] }]
  // Слова OCR → строки текста (по высоте), для свободного режима
  function ocrText(lines) {
    const words = lines.flatMap(l => l.words || [l]).filter(w => w && w.text && w.text.trim() && w.bbox);
    if (!words.length) return '';
    const hs = words.map(w => w.bbox.y1 - w.bbox.y0).sort((a, b) => a - b);
    const h = hs[Math.floor(hs.length / 2)] || 10;
    const sorted = words.slice().sort((a, b) => (a.bbox.y0 + a.bbox.y1) - (b.bbox.y0 + b.bbox.y1));
    const groups = [];
    for (const w of sorted) {
      const cy = (w.bbox.y0 + w.bbox.y1) / 2, g = groups[groups.length - 1];
      if (g && Math.abs(cy - g.cy) <= h * 0.6) { g.words.push(w); g.cy = (g.cy * (g.words.length - 1) + cy) / g.words.length; }
      else groups.push({ cy, words: [w] });
    }
    return groups.map(g => {
      const ws = g.words.sort((a, b) => a.bbox.x0 - b.bbox.x0); let s = '';
      ws.forEach((w, i) => { if (i) s += (w.bbox.x0 - ws[i - 1].bbox.x1 > h * 1.6) ? '  ' : ' '; s += w.text; });
      return s;
    }).join('\n');
  }

  function ocrToRows(lines) {
    // OCR может выдать столбцы отдельными блоками — пересобираем строки по высоте
    const words = lines.flatMap(l => l.words || [l]).filter(w => w && w.text && w.text.trim() && w.bbox);
    if (!words.length) return [];
    const heights = words.map(w => w.bbox.y1 - w.bbox.y0).sort((a, b) => a - b);
    const h = heights[Math.floor(heights.length / 2)] || 10;
    const sorted = words.slice().sort((a, b) => (a.bbox.y0 + a.bbox.y1) - (b.bbox.y0 + b.bbox.y1));
    const groups = [];
    for (const w of sorted) {
      const cy = (w.bbox.y0 + w.bbox.y1) / 2;
      const g = groups[groups.length - 1];
      if (g && Math.abs(cy - g.cy) <= h * 0.6) { g.words.push(w); g.cy = (g.cy * (g.words.length - 1) + cy) / g.words.length; }
      else groups.push({ cy, words: [w] });
    }
    const ls = groups.map(g => g.words);
    const gapLimit = h * 1.1;
    // 1. каждая строка → ячейки по крупным промежуткам
    const rowCells = ls.map(ws => {
      ws = ws.slice().sort((a, b) => a.bbox.x0 - b.bbox.x0);
      const cells = []; let cur = null;
      for (const w of ws) {
        if (cur && w.bbox.x0 - cur.x1 <= gapLimit) { cur.text += ' ' + w.text; cur.x1 = w.bbox.x1; }
        else { cur = { text: w.text, x0: w.bbox.x0, x1: w.bbox.x1 }; cells.push(cur); }
      }
      return cells;
    });
    // 2. столбцы — кластеры по интервалам x
    const all = rowCells.flat().slice().sort((a, b) => a.x0 - b.x0);
    const cols = [];
    for (const c of all) {
      const hit = cols.find(k => c.x0 <= k.x1 + h * 0.5 && c.x1 >= k.x0 - h * 0.5);
      if (hit) { hit.x0 = Math.min(hit.x0, c.x0); hit.x1 = Math.max(hit.x1, c.x1); hit.n++; }
      else cols.push({ x0: c.x0, x1: c.x1, n: 1 });
    }
    // слить пересекающиеся кластеры
    cols.sort((a, b) => a.x0 - b.x0);
    const merged = [];
    for (const k of cols) {
      const last = merged[merged.length - 1];
      if (last && k.x0 <= last.x1) { last.x1 = Math.max(last.x1, k.x1); last.n += k.n; }
      else merged.push({ ...k });
    }
    const rows = rowCells.map(cells => {
      const r = merged.map(() => '');
      for (const c of cells) {
        const mid = (c.x0 + c.x1) / 2;
        let best = 0, bd = Infinity;
        merged.forEach((k, i) => {
          const d = mid < k.x0 ? k.x0 - mid : mid > k.x1 ? mid - k.x1 : 0;
          if (d < bd) { bd = d; best = i; }
        });
        r[best] = r[best] ? r[best] + ' ' + c.text : c.text;
      }
      return r;
    });
    return normalize(rows);
  }

  /* ---------- Значения ячеек ---------- */
  function toNumber(s) {
    const t = String(s).trim();
    if (!t || t.length > 20) return null;
    if (/^[+-]?0\d/.test(t.replace(/[\s ]/g, '')) && !/^[+-]?0[.,]/.test(t)) return null; // 007, телефоны
    let m = t.replace(/[\s ]/g, '');
    let pct = false;
    if (m.endsWith('%')) { pct = true; m = m.slice(0, -1); }
    if (/^[+-]?\d+([.,]\d+)?$/.test(m)) m = m.replace(',', '.');
    else if (/^[+-]?\d{1,3}(,\d{3})+(\.\d+)?$/.test(m)) m = m.replace(/,/g, '');
    else if (/^[+-]?\d{1,3}(\.\d{3})+(,\d+)?$/.test(m)) m = m.replace(/\./g, '').replace(',', '.');
    else return null;
    const digits = m.replace(/\D/g, '');
    if (digits.length > 15) return null;
    if (/^\+?\d{10,}$/.test(t)) return null; // телефоны, номера счетов — текстом
    const v = parseFloat(m);
    if (!isFinite(v)) return null;
    return pct ? { v: v / 100, pct: true } : { v, pct: false };
  }

  /* ---------- XLSX ---------- */
  const enc = typeof TextEncoder !== 'undefined' ? new TextEncoder() : null;
  const utf8 = s => enc.encode(s);
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]))
    .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/g, '');

  const CRC = (() => {
    const t = new Uint32Array(256);
    for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; }
    return t;
  })();
  function crc32(b) { let c = 0xFFFFFFFF; for (let i = 0; i < b.length; i++) c = CRC[(c ^ b[i]) & 0xFF] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; }

  function zip(files) { // [{name, data:Uint8Array}] — без сжатия (store)
    const parts = []; const central = []; let offset = 0;
    const d = new Date();
    const time = (d.getHours() << 11) | (d.getMinutes() << 5) | (d.getSeconds() >> 1);
    const date = ((d.getFullYear() - 1980) << 9) | ((d.getMonth() + 1) << 5) | d.getDate();
    for (const f of files) {
      const name = utf8(f.name); const crc = crc32(f.data); const size = f.data.length;
      const lh = new DataView(new ArrayBuffer(30));
      lh.setUint32(0, 0x04034b50, true); lh.setUint16(4, 20, true); lh.setUint16(6, 0x0800, true);
      lh.setUint16(8, 0, true); lh.setUint16(10, time, true); lh.setUint16(12, date, true);
      lh.setUint32(14, crc, true); lh.setUint32(18, size, true); lh.setUint32(22, size, true);
      lh.setUint16(26, name.length, true); lh.setUint16(28, 0, true);
      parts.push(new Uint8Array(lh.buffer), name, f.data);
      const ch = new DataView(new ArrayBuffer(46));
      ch.setUint32(0, 0x02014b50, true); ch.setUint16(4, 20, true); ch.setUint16(6, 20, true); ch.setUint16(8, 0x0800, true);
      ch.setUint16(10, 0, true); ch.setUint16(12, time, true); ch.setUint16(14, date, true);
      ch.setUint32(16, crc, true); ch.setUint32(20, size, true); ch.setUint32(24, size, true);
      ch.setUint16(28, name.length, true); ch.setUint32(42, offset, true);
      central.push(new Uint8Array(ch.buffer), name);
      offset += 30 + name.length + size;
    }
    const cdSize = central.reduce((s, p) => s + p.length, 0);
    const end = new DataView(new ArrayBuffer(22));
    end.setUint32(0, 0x06054b50, true); end.setUint16(8, files.length, true); end.setUint16(10, files.length, true);
    end.setUint32(12, cdSize, true); end.setUint32(16, offset, true);
    const all = [...parts, ...central, new Uint8Array(end.buffer)];
    const out = new Uint8Array(all.reduce((s, p) => s + p.length, 0));
    let p = 0; for (const a of all) { out.set(a, p); p += a.length; }
    return out;
  }

  function colName(i) { let s = ''; i++; while (i > 0) { const m = (i - 1) % 26; s = String.fromCharCode(65 + m) + s; i = Math.floor((i - 1) / 26); } return s; }

  function buildXlsx(rows, opts) {
    opts = opts || {};
    const header = opts.header !== false && rows.length > 1;
    const sheetName = esc((opts.sheetName || 'Лист1').replace(/[\\\/\?\*\[\]:]/g, ' ').slice(0, 31));
    const w = Math.max(1, ...rows.map(r => r.length));
    const widths = Array.from({ length: w }, (_, j) => {
      const m = Math.max(4, ...rows.map(r => String(r[j] || '').length));
      return Math.min(60, m + 2);
    });
    let sd = '';
    rows.forEach((r, i) => {
      sd += `<row r="${i + 1}">`;
      for (let j = 0; j < w; j++) {
        const ref = colName(j) + (i + 1); const val = r[j] == null ? '' : String(r[j]);
        const isHead = header && i === 0;
        const num = isHead ? null : toNumber(val);
        if (num) sd += `<c r="${ref}" s="${num.pct ? 4 : 3}"><v>${num.v}</v></c>`;
        else if (val === '') sd += `<c r="${ref}" s="${isHead ? 1 : 2}"/>`;
        else sd += `<c r="${ref}" s="${isHead ? 1 : 2}" t="inlineStr"><is><t xml:space="preserve">${esc(val)}</t></is></c>`;
      }
      sd += '</row>';
    });
    const last = colName(w - 1) + Math.max(1, rows.length);
    const sheet = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<dimension ref="A1:${last}"/>
<sheetViews><sheetView workbookViewId="0">${header ? '<pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>' : ''}</sheetView></sheetViews>
<sheetFormatPr defaultRowHeight="15"/>
<cols>${widths.map((x, j) => `<col min="${j + 1}" max="${j + 1}" width="${x}" customWidth="1"/>`).join('')}</cols>
<sheetData>${sd}</sheetData>
${header ? `<autoFilter ref="A1:${last}"/>` : ''}
</worksheet>`;
    const styles = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<numFmts count="1"><numFmt numFmtId="164" formatCode="0.##%"/></numFmts>
<fonts count="2"><font><sz val="11"/><color rgb="FF111111"/><name val="Calibri"/><family val="2"/></font><font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/><family val="2"/></font></fonts>
<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF111111"/><bgColor indexed="64"/></patternFill></fill></fills>
<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color rgb="FFD4D4D4"/></left><right style="thin"><color rgb="FFD4D4D4"/></right><top style="thin"><color rgb="FFD4D4D4"/></top><bottom style="thin"><color rgb="FFD4D4D4"/></bottom><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="5">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment vertical="center" wrapText="1"/></xf>
<xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="1" applyAlignment="1"><alignment vertical="top" wrapText="1"/></xf>
<xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="1" applyAlignment="1"><alignment vertical="top"/></xf>
<xf numFmtId="164" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1"/>
</cellXfs>
<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>`;
    const files = [
      ['[Content_Types].xml', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>`],
      ['_rels/.rels', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>`],
      ['docProps/core.xml', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:creator>Сетка</dc:creator><dcterms:created xsi:type="dcterms:W3CDTF">${new Date().toISOString().replace(/\.\d+Z$/, 'Z')}</dcterms:created></cp:coreProperties>`],
      ['docProps/app.xml', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Setka</Application></Properties>`],
      ['xl/workbook.xml', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="${sheetName}" sheetId="1" r:id="rId1"/></sheets>${header ? `<definedNames><definedName name="_xlnm._FilterDatabase" localSheetId="0" hidden="1">'${sheetName.replace(/'/g, "''")}'!$A$1:$${colName(w - 1)}$${rows.length}</definedName></definedNames>` : ''}</workbook>`],
      ['xl/_rels/workbook.xml.rels', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>`],
      ['xl/styles.xml', styles],
      ['xl/worksheets/sheet1.xml', sheet],
    ];
    return zip(files.map(([name, s]) => ({ name, data: utf8(s) })));
  }

  function toTSV(rows) {
    return rows.map(r => r.map(c => /[\t\n"]/.test(c) ? '"' + String(c).replace(/"/g, '""') + '"' : c).join('\t')).join('\n');
  }

  // Вытащить таблицу из ответа модели
  function extractRows(text) {
    const s = String(text);
    const tries = [];
    const fence = s.match(/```(?:json)?\s*([\s\S]*?)```/); if (fence) tries.push(fence[1]);
    const a = s.indexOf('{'), b = s.lastIndexOf('}'); if (a >= 0 && b > a) tries.push(s.slice(a, b + 1));
    const c = s.indexOf('['), d = s.lastIndexOf(']'); if (c >= 0 && d > c) tries.push(s.slice(c, d + 1));
    for (const t of tries) {
      try {
        const v = JSON.parse(t);
        const rows = Array.isArray(v) ? v : (v.rows || v.table || v.data);
        if (Array.isArray(rows)) {
          const out = normalize(rows.map(r => Array.isArray(r) ? r : Object.values(r)));
          if (v && typeof v.header === 'boolean') out.header = v.header;
          return out;
        }
      } catch (e) { /* дальше */ }
    }
    return parseText(s).rows;
  }

  const Core = { ocrText, structureFree, findPairs, hasPair, parseText, detectDelimiter, ocrToRows, normalize, toNumber, buildXlsx, toTSV, extractRows, colName, crc32 };
  root.Core = Core;
  if (typeof module !== 'undefined' && module.exports) module.exports = Core;
})(typeof window !== 'undefined' ? window : globalThis);
