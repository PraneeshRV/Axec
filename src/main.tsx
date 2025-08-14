import React, { useEffect, useMemo, useState } from 'react';
import ReactDOM from 'react-dom/client';
import './styles.css';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

type AppImageEntry = {
  id: string;
  name: string;
  path: string;
  icon_path?: string | null;
  desktop_file: string;
};

function App() {
  const [apps, setApps] = useState<AppImageEntry[]>([]);
  const [query, setQuery] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<AppImageEntry[]>('list_apps');
      setApps(result);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return apps;
    return apps.filter((a) => a.name.toLowerCase().includes(q));
  }, [apps, query]);

  const handleAdd = async () => {
    setError(null);
    try {
      const selection = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: 'AppImage', extensions: ['appimage', 'AppImage'] },
          { name: 'All files', extensions: ['*'] },
        ],
      });
      if (!selection || Array.isArray(selection)) return;
      setBusy(true);
      await invoke<AppImageEntry>('add_appimage', { filePath: selection });
      await load();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleLaunch = async (id: string) => {
    try {
      await invoke('launch_app', { id });
    } catch (e: any) {
      setError(String(e));
    }
  };

  const handleRemove = async (id: string) => {
    try {
      setBusy(true);
      await invoke('remove_app', { id });
      await load();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <header className="border-b border-slate-800 bg-slate-900/60 backdrop-blur sticky top-0 z-10">
        <div className="max-w-4xl mx-auto px-4 py-3 flex items-center gap-3">
          <div className="text-xl font-semibold">Axec</div>
          <div className="ml-auto flex items-center gap-2">
            <input
              className="px-3 py-2 rounded-md bg-slate-800 border border-slate-700 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500"
              placeholder="Search apps…"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <button
              className="px-3 py-2 rounded-md bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 disabled:opacity-50"
              onClick={handleAdd}
              disabled={busy}
            >
              Add AppImage
            </button>
          </div>
        </div>
      </header>

      <main className="max-w-4xl mx-auto p-4">
        {error && (
          <div className="mb-3 rounded-md border border-red-700 bg-red-900/30 text-red-200 px-3 py-2">
            {error}
          </div>
        )}
        {busy && (
          <div className="mb-3 text-slate-400">Working…</div>
        )}

        {filtered.length === 0 ? (
          <div className="mt-16 text-center text-slate-400">
            <div className="text-2xl mb-2">No AppImages yet</div>
            <div>Click “Add AppImage” to import one.</div>
          </div>
        ) : (
          <ul className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {filtered.map((app) => (
              <li
                key={app.id}
                className="rounded-lg border border-slate-800 bg-slate-900/40 p-3 flex items-center gap-3"
              >
                <img
                  src={app.icon_path ? convertFileSrc(app.icon_path) : '/src/assets/tauri.svg'}
                  alt="icon"
                  className="w-10 h-10 rounded"
                  onError={(e) => {
                    (e.target as HTMLImageElement).src = '/src/assets/tauri.svg';
                  }}
                />
                <div className="flex-1 min-w-0">
                  <div className="font-medium truncate">{app.name}</div>
                  <div className="text-xs text-slate-400 truncate" title={app.path}>
                    {app.path}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    className="px-2.5 py-1.5 rounded-md bg-emerald-600 hover:bg-emerald-500 text-sm"
                    onClick={() => handleLaunch(app.id)}
                  >
                    Launch
                  </button>
                  <Menu onRemove={() => handleRemove(app.id)} />
                </div>
              </li>
            ))}
          </ul>
        )}
      </main>
    </div>
  );
}

function Menu({ onRemove }: { onRemove: () => void }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <button
        className="w-9 h-9 grid place-items-center rounded-md border border-slate-700 hover:bg-slate-800"
        onClick={() => setOpen((v) => !v)}
      >
        ⋮
      </button>
      {open && (
        <div className="absolute right-0 mt-1 w-40 rounded-md border border-slate-700 bg-slate-900 shadow-lg z-20">
          <button
            className="w-full text-left px-3 py-2 text-red-300 hover:bg-red-900/30"
            onClick={() => {
              setOpen(false);
              onRemove();
            }}
          >
            Remove
          </button>
        </div>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
