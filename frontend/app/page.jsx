'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL ?? 'http://localhost:5085';
const STORAGE_KEY = 'sticky_account_id';

function readStoredAccountId() {
  if (typeof window === 'undefined') return null;
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  const id = Number(raw);
  return Number.isFinite(id) ? id : null;
}

function storeAccountId(id) {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(STORAGE_KEY, String(id));
}

function clearAccountId() {
  if (typeof window === 'undefined') return;
  window.localStorage.removeItem(STORAGE_KEY);
}

export default function AccountPage() {
  const router = useRouter();
  const [accounts, setAccounts] = useState([]);
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [form, setForm] = useState({ name: '', email: '', password: '' });

  useEffect(() => {
    let mounted = true;
    async function load() {
      try {
        setLoading(true);
        const res = await fetch(`${API_BASE}/api/accounts`);
        if (!res.ok) throw await parseError(res);
        const data = await res.json();
        if (!mounted) return;
        setAccounts(data.accounts ?? []);
        setStatus(`アカウント数: ${(data.accounts ?? []).length}`);
        setError('');
        const stored = readStoredAccountId();
        if (stored) {
          router.push('/groups');
        }
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
  }, [router]);

  const handleSubmit = async (event) => {
    event.preventDefault();
    setError('');
    setStatus('登録中...');
    try {
      const res = await fetch(`${API_BASE}/api/accounts`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(form),
      });
      if (!res.ok) throw await parseError(res);
      const account = await res.json();
      setAccounts((prev) => [...prev, account]);
      storeAccountId(account.id);
      setForm({ name: '', email: '', password: '' });
      setStatus(`ようこそ、${account.name} さん！`);
      setError('');
      router.push('/groups');
    } catch (err) {
      console.error(err);
      setError(err.message);
      setStatus('');
    }
  };

  const handleSelectAccount = (id) => {
    storeAccountId(id);
    router.push('/groups');
  };

  const handleReset = () => {
    clearAccountId();
    setStatus('選択済みアカウントをクリアしました。');
  };

  return (
    <main style={styles.main}>
      <section style={styles.card}>
        <h1 style={styles.heading}>アカウントを選択 / 作成</h1>
        <p style={styles.lead}>
          まずは共有に使うアカウントを選んでください。既存のアカウントを選ぶか、新しく作成できます。
        </p>

        <div style={styles.panel}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h2 style={styles.subheading}>既存アカウント</h2>
            <button type="button" onClick={handleReset} style={styles.linkButton}>
              選択解除
            </button>
          </div>
          {loading ? (
            <div style={styles.message}>読み込み中...</div>
          ) : accounts.length === 0 ? (
            <div style={styles.message}>まだアカウントがありません。下のフォームから作成してください。</div>
          ) : (
            <ul style={styles.list}>
              {accounts.map((account) => (
                <li key={account.id} style={styles.listItem}>
                  <div>
                    <div style={{ fontWeight: 600 }}>{account.name}</div>
                    <div style={styles.muted}>{account.email}</div>
                  </div>
                  <button style={styles.primaryButton} onClick={() => handleSelectAccount(account.id)}>
                    これを使う
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <form onSubmit={handleSubmit} style={styles.form}>
          <h2 style={styles.subheading}>新規アカウント作成</h2>
          <label style={styles.label}>
            名前
            <input
              style={styles.input}
              name="name"
              value={form.name}
              onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
              required
              maxLength={40}
              placeholder="例: さとし"
            />
          </label>
          <label style={styles.label}>
            メールアドレス
            <input
              style={styles.input}
              type="email"
              name="email"
              value={form.email}
              onChange={(e) => setForm((prev) => ({ ...prev, email: e.target.value }))}
              required
              placeholder="you@example.com"
            />
          </label>
          <label style={styles.label}>
            パスワード
            <input
              style={styles.input}
              type="password"
              name="password"
              minLength={6}
              value={form.password}
              onChange={(e) => setForm((prev) => ({ ...prev, password: e.target.value }))}
              required
              placeholder="6文字以上"
            />
          </label>
          <button type="submit" style={{ ...styles.primaryButton, width: '100%' }}>
            アカウント登録
          </button>
        </form>

        <Status status={status} error={error} />
        <div style={styles.bottomLink}>
          <button type="button" style={styles.linkButton} onClick={() => router.push('/groups')}>
            グループ一覧へ進む →
          </button>
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
    width: 'min(880px, 100%)',
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
    fontSize: 28,
  },
  subheading: {
    margin: '0 0 8px',
    fontSize: 18,
  },
  lead: {
    margin: 0,
    color: '#475569',
  },
  panel: {
    border: '1px dashed #ccd1da',
    borderRadius: 14,
    padding: 16,
    display: 'grid',
    gap: 12,
  },
  list: {
    listStyle: 'none',
    margin: 0,
    padding: 0,
    display: 'grid',
    gap: 10,
  },
  listItem: {
    borderRadius: 12,
    border: '1px solid #e2e8f0',
    padding: 12,
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: 12,
  },
  message: {
    padding: '12px 0',
    color: '#64748b',
    fontSize: 14,
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
  bottomLink: {
    display: 'flex',
    justifyContent: 'flex-end',
    marginTop: 8,
  },
};
