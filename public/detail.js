function getId() {
  const u = new URL(location.href);
  return Number(u.searchParams.get('id') || '0');
}

async function load() {
  const id = getId();
  if (!id) {
    document.body.textContent = 'IDが不正です';
    return;
  }
  const API_BASE = (location.port === '5500') ? 'http://localhost:5085' : '';
  try {
    const res = await fetch(`${API_BASE}/api/summaries/${id}`);
    if (!res.ok) throw new Error(await res.text());
    const data = await res.json();
    document.getElementById('meta').textContent = `#${data.item.id} / ${data.item.created_at}`;
    document.getElementById('summary').textContent = data.item.summary;
    document.getElementById('input').textContent = data.item.input_text;
  } catch (e) {
    document.body.textContent = '読み込みに失敗しました: ' + e.message;
  }
}

document.addEventListener('DOMContentLoaded', load);
