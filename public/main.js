const el = (id) => document.getElementById(id);

async function loadList() {
  const ul = el('list');
  const status = document.getElementById('list-status');
  status.textContent = '読み込み中…';
  try {
    const res = await fetch('/api/summaries');
    if (!res.ok) throw new Error(await res.text());
    const data = await res.json();
    status.textContent = '';
    if (!data.items || data.items.length === 0) {
      status.textContent = 'まだ要約はありません。';
      return;
    }
    for (const it of data.items) {
      const a = document.createElement('a');
      a.className = 'item';
      a.href = `/detail.html?id=${it.id}`;
      a.innerHTML = `
        <div class="item-title">
          <span class="chev"></span>
          <span>#${it.id}</span>
          <span>${escapeHtml(it.summary_preview)}</span>
        </div>
        <div class="item-sub">${it.created_at}</div>
      `;
      ul.appendChild(a);
    }
  } catch (e) {
    status.textContent = '読み込みに失敗しました';
    console.error(e);
  }
}

async function summarize() {
  const text = el('text').value;
  const status = el('status');
  const result = el('result');
  status.textContent = '要約中…';
  result.textContent = '';
  el('submit').disabled = true;
  document.getElementById('spinner').style.display = 'inline-block';
  try {
    const res = await fetch('/api/summarize', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
      body: text,
    });
    if (!res.ok) throw new Error(await res.text());
    const data = await res.json();
    result.textContent = data.summary || '';
    status.innerHTML = `保存しました: <a href="/detail.html?id=${data.id}">#${data.id}</a>`;
    el('text').value = '';
    // refresh list
    loadList();
  } catch (e) {
    status.textContent = 'エラー: ' + e.message;
  } finally {
    el('submit').disabled = false;
    document.getElementById('spinner').style.display = 'none';
  }
}

document.addEventListener('DOMContentLoaded', () => {
  el('submit').addEventListener('click', summarize);
  loadList();
});

function escapeHtml(s) {
  return s.replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
}
