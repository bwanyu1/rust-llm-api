'use client';

import { useEffect, useMemo, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL ?? 'http://localhost:5085';
const ACCOUNT_KEY = 'sticky_account_id';

function readAccountId() {
  if (typeof window === 'undefined') return null;
  const raw = window.localStorage.getItem(ACCOUNT_KEY);
  if (!raw) return null;
  const id = Number(raw);
  return Number.isFinite(id) ? id : null;
}

export default function BoardPage() {
  const { groupId } = useParams();
  const router = useRouter();
  const numericGroupId = useMemo(() => Number(groupId), [groupId]);

  const [accountId, setAccountId] = useState(null);
  const [notes, setNotes] = useState([]);
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [form, setForm] = useState({
    title: '',
    content: '',
    color: '#FFFF88',
  });

  useEffect(() => {
    const id = readAccountId();
    if (!id) {
      router.replace('/');
      return;
    }
    setAccountId(id);
  }, [router]);

  useEffect(() => {
    if (!numericGroupId || Number.isNaN(numericGroupId)) {
      setError('グループIDが不正です');
      setLoading(false);
      return;
    }
    let mounted = true;
    async function load() {
      try {
        setLoading(true);
        const res = await fetch(`${API_BASE}/api/groups/${numericGroupId}/notes`);
        if (!res.ok) throw await parseError(res);
        const data = await res.json();
        if (!mounted) return;
        setNotes(data.notes ?? []);
        setStatus(`付箋数: ${(data.notes ?? []).length}`);
        setError('');
      } catch (err) {
        console.error(err);
        if (!mounted) return;
        setError(err.message);
        setStatus('');
      } finally {
        if (mounted) setLoading(false);
      }
    }
    load();
    return () => {
      mounted = false;
    };
  }, [numericGroupId]);

  const handleSubmit = async (event) => {
    event.preventDefault();
    if (!accountId) {
      setError('アカウントを選択してください');
      return;
    }
    if (!numericGroupId) return;
    setStatus('付箋を追加しています...');
    setError('');
    try {
      const payload = {
        ...form,
        x: nextPosition(notes.length, 'x'),
        y: nextPosition(notes.length, 'y'),
        width: 220,
        height: 160,
        z_index: notes.length + 1,
        created_by: accountId,
        can_edit: true,
      };
      const res = await fetch(`${API_BASE}/api/groups/${numericGroupId}/notes`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw await parseError(res);
      setForm({ title: '', content: '', color: '#FFFF88' });
      await reloadNotes(numericGroupId, setNotes, setStatus, setError, setLoading);
    } catch (err) {
      console.error(err);
      setError(err.message);
      setStatus('');
    }
  };

  const handleDelete = async (noteId) => {
    if (!window.confirm('この付箋を削除しますか？')) return;
    try {
      const res = await fetch(`${API_BASE}/api/notes/${noteId}`, { method: 'DELETE' });
      if (!res.ok) throw await parseError(res);
      await reloadNotes(numericGroupId, setNotes, setStatus, setError, setLoading);
    } catch (err) {
      console.error(err);
      setError(err.message);
      setStatus('');
    }
  };

  const handleClear = async () => {
    if (!numericGroupId) return;
    if (!window.confirm('このグループの付箋をすべて削除しますか？')) return;
    try {
      const res = await fetch(`${API_BASE}/api/groups/${numericGroupId}/notes`, { method: 'DELETE' });
      if (!res.ok) throw await parseError(res);
      await reloadNotes(numericGroupId, setNotes, setStatus, setError, setLoading);
    } catch (err) {
      console.error(err);
      setError(err.message);
      setStatus('');
    }
  };

  return (
    <main style={styles.main}>
      <section style={styles.card}>
        <header style={{ display: 'grid', gap: 4 }}>
          <h1 style={styles.heading}>グループ #{numericGroupId} の付箋ボード</h1>
          <div style={styles.muted}>現在のアカウント ID: {accountId ?? '未選択'}</div>
        </header>

        <div style={styles.toolbar}>
          <button type="button" style={styles.linkButton} onClick={() => router.push('/groups')}>
            ← グループ一覧に戻る
          </button>
          <button type="button" style={styles.dangerButton} onClick={handleClear}>
            すべて削除
          </button>
        </div>

        <form onSubmit={handleSubmit} style={styles.form}>
          <h2 style={styles.subheading}>付箋を追加</h2>
          <label style={styles.label}>
            タイトル
            <input
              style={styles.input}
              value={form.title}
              onChange={(e) => setForm((prev) => ({ ...prev, title: e.target.value }))}
              placeholder="例: 明日のToDo"
              maxLength={80}
            />
          </label>
          <label style={styles.label}>
            本文
            <textarea
              style={{ ...styles.input, minHeight: 120, resize: 'vertical' }}
              value={form.content}
              onChange={(e) => setForm((prev) => ({ ...prev, content: e.target.value }))}
              placeholder="内容を入力してください（500文字まで）"
              maxLength={500}
            />
          </label>
          <label style={styles.label}>
            カラー
            <select
              style={styles.input}
              value={form.color}
              onChange={(e) => setForm((prev) => ({ ...prev, color: e.target.value }))}
            >
              <option value="#FFFF88">イエロー</option>
              <option value="#FBCFE8">ピンク</option>
              <option value="#BBF7D0">グリーン</option>
              <option value="#BFDBFE">ブルー</option>
              <option value="#FED7AA">オレンジ</option>
              <option value="#E9D5FF">パープル</option>
            </select>
          </label>
          <button type="submit" style={styles.primaryButton}>
            追加する
          </button>
        </form>

        <Status status={status} error={error} />

        <div style={styles.board}>
          {loading ? (
            <div style={styles.message}>読み込み中...</div>
          ) : notes.length === 0 ? (
            <div style={styles.message}>付箋はまだありません。上のフォームから追加してください。</div>
          ) : (
            <div style={styles.grid}>
              {notes.map((note) => (
                <article key={note.id} style={{ ...styles.note, background: note.color ?? '#FFFF88' }}>
                  <header style={styles.noteHeader}>
                    <div style={{ fontWeight: 600 }}>{note.title?.trim() || `付箋 #${note.id}`}</div>
                    <div style={styles.noteMeta}>{formatTime(note.updated_at)}</div>
                  </header>
                  <pre style={styles.noteBody}>{note.content?.trim() || '（本文なし）'}</pre>
                  <footer style={styles.noteFooter}>
                    <button type="button" style={styles.noteDangerButton} onClick={() => handleDelete(note.id)}>
                      削除
                    </button>
                  </footer>
                </article>
              ))}
            </div>
          )}
        </div>
      </section>
    </main>
  );
}

function Status({ status, error }) {
  if (!status && !error) return null;
  return (
    <div
      style={{
        marginTop: 16,
        padding: 12,
        borderRadius: 10,
        background: error ? '#fee2e2' : '#e0f2fe',
        color: error ? '#b91c1c' : '#0c4a6e',
        border: `1px solid ${error ? '#fecaca' : '#bae6fd'}`,
      }}
    >
      {error || status}
    </div>
  );
}

async function reloadNotes(groupId, setNotes, setStatus, setError, setLoading) {
  try {
    setLoading(true);
    const res = await fetch(`${API_BASE}/api/groups/${groupId}/notes`);
    if (!res.ok) throw await parseError(res);
    const data = await res.json();
    setNotes(data.notes ?? []);
    setStatus(`付箋数: ${(data.notes ?? []).length}`);
    setError('');
  } catch (err) {
    console.error(err);
    setError(err.message);
    setStatus('');
  } finally {
    setLoading(false);
  }
}

function nextPosition(index, axis) {
  const base = axis === 'x' ? 30 : 30;
  const step = axis === 'x' ? 240 : 180;
  return base + ((index * step) % 720);
}

function formatTime(isoLike) {
  if (!isoLike) return '';
  const date = new Date(isoLike.replace(' ', 'T') + 'Z');
  if (Number.isNaN(date.getTime())) return isoLike;
  return date.toLocaleString('ja-JP', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

async function parseError(res) {
  try {
    const data = await res.json();
    if (data && data.message) return new Error(data.message);
  } catch (e) {
    // ignore
  }
  return new Error(res.statusText || 'エラーが発生しました');
}

const styles = {
  main: {
    minHeight: '100vh',
    display: 'grid',
    placeItems: 'center',
    padding: '40px 16px',
    background: '#f8fafc',
  },
  card: {
    width: 'min(1080px, 100%)',
    background: '#fff',
    borderRadius: 18,
    padding: 28,
    boxShadow: '0 24px 40px -30px rgba(15,23,42,0.45)',
    border: '1px solid #d6d9e0',
    display: 'grid',
    gap: 20,
  },
  heading: {
    margin: 0,
    fontSize: 26,
  },
  toolbar: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  subheading: {
    margin: '0 0 8px',
    fontSize: 18,
  },
  form: {
    display: 'grid',
    gap: 12,
  },
  label: {
    display: 'grid',
    gap: 6,
    fontWeight: 600,
    fontSize: 14,
  },
  input: {
    borderRadius: 10,
    border: '1px solid #d6d9e0',
    padding: '10px 12px',
    fontFamily: 'inherit',
  },
  primaryButton: {
    background: '#2563eb',
    color: '#fff',
    border: '1px solid #2563eb',
    borderRadius: 10,
    padding: '10px 16px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  dangerButton: {
    background: '#f43f5e',
    color: '#fff',
    border: '1px solid #f43f5e',
    borderRadius: 10,
    padding: '10px 16px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  linkButton: {
    background: 'transparent',
    color: '#2563eb',
    border: 'none',
    cursor: 'pointer',
    fontWeight: 600,
  },
  muted: {
    fontSize: 13,
    color: '#6b7280',
  },
  board: {
    border: '1px dashed #ccd1da',
    borderRadius: 14,
    padding: 16,
    minHeight: 320,
    background: '#fdfdfd',
  },
  message: {
    padding: '12px 0',
    color: '#64748b',
    fontSize: 14,
  },
  grid: {
    display: 'grid',
    gap: 16,
    gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
  },
  note: {
    borderRadius: 16,
    padding: 16,
    boxShadow: '0 18px 24px -22px rgba(15,23,42,0.6)',
    display: 'grid',
    gap: 12,
    minHeight: 180,
  },
  noteHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: 12,
  },
  noteMeta: {
    fontSize: 12,
    color: 'rgba(15,23,42,0.6)',
  },
  noteBody: {
    margin: 0,
    whiteSpace: 'pre-wrap',
    fontSize: 14,
    lineHeight: 1.5,
  },
  noteFooter: {
    display: 'flex',
    justifyItems: 'flex-end',
    justifyContent: 'flex-end',
  },
  noteDangerButton: {
    background: 'rgba(239,68,68,0.2)',
    color: '#b91c1c',
    border: 'none',
    borderRadius: 8,
    padding: '6px 10px',
    cursor: 'pointer',
    fontWeight: 600,
  },
};
