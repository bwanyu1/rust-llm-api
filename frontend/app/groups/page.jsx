'use client';

import { useEffect, useMemo, useState } from 'react';
import { useRouter } from 'next/navigation';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL ?? 'http://localhost:5085';
const ACCOUNT_KEY = 'sticky_account_id';

function readAccountId() {
  if (typeof window === 'undefined') return null;
  const raw = window.localStorage.getItem(ACCOUNT_KEY);
  if (!raw) return null;
  const id = Number(raw);
  return Number.isFinite(id) ? id : null;
}

export default function GroupsPage() {
  const router = useRouter();
  const [accountId, setAccountId] = useState(null);
  const [groups, setGroups] = useState([]);
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [createForm, setCreateForm] = useState({ group_name: '' });
  const [joinForm, setJoinForm] = useState({ group_id: '', role: 'member' });

  useEffect(() => {
    const id = readAccountId();
    if (!id) {
      router.replace('/');
      return;
    }
    setAccountId(id);
  }, [router]);

  useEffect(() => {
    if (!accountId) return;
    let mounted = true;
    async function load() {
      try {
        setLoading(true);
        const res = await fetch(`${API_BASE}/api/accounts/${accountId}/groups`);
        if (!res.ok) throw await parseError(res);
        const data = await res.json();
        if (!mounted) return;
        setGroups(data.groups ?? []);
        setStatus(`グループ数: ${(data.groups ?? []).length}`);
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
  }, [accountId]);

  const accountLabel = useMemo(() => {
    if (!accountId) return '';
    return `現在のアカウント ID: ${accountId}`;
  }, [accountId]);

  const handleCreate = async (event) => {
    event.preventDefault();
    if (!accountId) return;
    setStatus('グループ作成中...');
    setError('');
    try {
      const payload = { group_name: createForm.group_name, created_by: accountId };
      const res = await fetch(`${API_BASE}/api/groups`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw await parseError(res);
      const group = await res.json();
      setGroups((prev) => [...prev, { ...group, role: 'owner' }]);
      setCreateForm({ group_name: '' });
      setStatus(`グループ #${group.id} を作成しました`);
      setError('');
    } catch (err) {
      console.error(err);
      setError(err.message);
      setStatus('');
    }
  };

  const handleJoin = async (event) => {
    event.preventDefault();
    if (!accountId) return;
    const groupId = Number(joinForm.group_id);
    if (!groupId) {
      setError('グループIDを入力してください');
      return;
    }
    setStatus('参加処理中...');
    setError('');
    try {
      const res = await fetch(`${API_BASE}/api/groups/${groupId}/users`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: accountId, role: joinForm.role }),
      });
      if (!res.ok) throw await parseError(res);
      setJoinForm({ group_id: '', role: 'member' });
      setStatus(`グループ #${groupId} に参加しました`);
      await refreshGroups(accountId, setGroups, setStatus, setError, setLoading);
    } catch (err) {
      console.error(err);
      setError(err.message);
      setStatus('');
    }
  };

  const handleOpenBoard = (groupId) => {
    router.push(`/board/${groupId}`);
  };

  const handleRefresh = () => {
    if (accountId) {
      refreshGroups(accountId, setGroups, setStatus, setError, setLoading);
    }
  };

  return (
    <main style={styles.main}>
      <section style={styles.card}>
        <header style={{ display: 'grid', gap: 4 }}>
          <h1 style={styles.heading}>グループを選択</h1>
          <div style={styles.muted}>{accountLabel}</div>
          <p style={styles.lead}>
            参加できるグループの一覧です。ボードを開くグループを選んでください。新しいグループを作成したり、ID を指定して参加することもできます。
          </p>
        </header>

        <div style={styles.panel}>
          <div style={styles.panelHeader}>
            <h2 style={styles.subheading}>所属グループ</h2>
            <button type="button" style={styles.linkButton} onClick={handleRefresh}>
              更新
            </button>
          </div>
          {loading ? (
            <div style={styles.message}>読み込み中...</div>
          ) : groups.length === 0 ? (
            <div style={styles.message}>まだグループがありません。下のフォームから作成または参加できます。</div>
          ) : (
            <ul style={styles.list}>
              {groups.map((group) => (
                <li key={group.id} style={styles.listItem}>
                  <div>
                    <div style={{ fontWeight: 600 }}>{group.group_name}</div>
                    <div style={styles.muted}>
                      #{group.id} / role: {group.role}
                    </div>
                  </div>
                  <button style={styles.primaryButton} onClick={() => handleOpenBoard(group.id)}>
                    ボードへ
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <form onSubmit={handleCreate} style={styles.form}>
          <h2 style={styles.subheading}>新しいグループを作る</h2>
          <label style={styles.label}>
            グループ名
            <input
              style={styles.input}
              required
              value={createForm.group_name}
              onChange={(e) => setCreateForm({ group_name: e.target.value })}
              placeholder="例: プロジェクトA"
              maxLength={80}
            />
          </label>
          <button type="submit" style={styles.primaryButton}>
            作成する
          </button>
        </form>

        <form onSubmit={handleJoin} style={styles.form}>
          <h2 style={styles.subheading}>グループに参加（ID 指定）</h2>
          <label style={styles.label}>
            グループID
            <input
              style={styles.input}
              type="number"
              min={1}
              value={joinForm.group_id}
              onChange={(e) => setJoinForm((prev) => ({ ...prev, group_id: e.target.value }))}
              placeholder="数値で入力"
              required
            />
          </label>
          <label style={styles.label}>
            役割
            <select
              style={styles.input}
              value={joinForm.role}
              onChange={(e) => setJoinForm((prev) => ({ ...prev, role: e.target.value }))}
            >
              <option value="member">member</option>
              <option value="owner">owner</option>
            </select>
          </label>
          <button type="submit" style={styles.primaryButton}>
            参加する
          </button>
        </form>

        <Status status={status} error={error} />

        <div style={styles.bottomLinks}>
          <button type="button" onClick={() => router.push('/')} style={styles.linkButton}>
            ← アカウント選択に戻る
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

async function refreshGroups(accountId, setGroups, setStatus, setError, setLoading) {
  if (!accountId) return;
  try {
    setLoading(true);
    const res = await fetch(`${API_BASE}/api/accounts/${accountId}/groups`);
    if (!res.ok) throw await parseError(res);
    const data = await res.json();
    setGroups(data.groups ?? []);
    setStatus(`グループ数: ${(data.groups ?? []).length}`);
    setError('');
  } catch (err) {
    console.error(err);
    setError(err.message);
    setStatus('');
  } finally {
    setLoading(false);
  }
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
    fontSize: 26,
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
  panelHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
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
  message: {
    padding: '12px 0',
    color: '#64748b',
    fontSize: 14,
  },
  bottomLinks: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
};
