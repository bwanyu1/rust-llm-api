const el = (id) => document.getElementById(id);
// If served from Live Server (:5500), call the API on the Docker-exposed port.
const API_BASE = (location.port === '5500') ? 'http://localhost:5085' : '';

async function loadList() {
  const ul = el('list');
  ul.textContent = '読み込み中...';
  try {
    const res = await fetch(`${API_BASE}/api/summaries`);
    if (!res.ok) throw new Error(await res.text());
    const data = await res.json();
    ul.textContent = '';
    if (!data.items || data.items.length === 0) {
      ul.textContent = 'まだ要約はありません。';
      return;
    }
    for (const it of data.items) {
      const li = document.createElement('li');
      const a = document.createElement('a');
      a.href = `detail.html?id=${it.id}`;
      a.textContent = `#${it.id} ${it.summary_preview}`;
      const small = document.createElement('div');
      small.className = 'muted';
      small.textContent = it.created_at;
      li.appendChild(a);
      li.appendChild(small);
      ul.appendChild(li);
    }
  } catch (e) {
    ul.textContent = '読み込みに失敗しました';
    console.error(e);
  }
}

async function summarize() {
  const text = el('text').value;
  const status = el('status');
  const result = el('result');
  status.textContent = '要約中...';
  result.textContent = '';
  el('submit').disabled = true;
  try {
    const res = await fetch(`${API_BASE}/api/summarize`, {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
      body: text,
    });
    if (!res.ok) throw new Error(await res.text());
    const data = await res.json();
    result.textContent = data.summary || '';
    status.innerHTML = `保存しました: <a href="detail.html?id=${data.id}">#${data.id}</a>`;
    el('text').value = '';
    // refresh list
    loadList();
  } catch (e) {
    status.textContent = 'エラー: ' + e.message;
  } finally {
    el('submit').disabled = false;
  }
}

document.addEventListener('DOMContentLoaded', () => {
  el('submit').addEventListener('click', summarize);
  loadList();
});
