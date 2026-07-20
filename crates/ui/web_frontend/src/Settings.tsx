import React, { useState, useEffect } from 'react';
import { 
  Save, 
  Settings2, 
  Server, 
  Cpu, 
  Mic, 
  Puzzle, 
  Check, 
  Loader2 
} from 'lucide-react';

interface Config {
  backend: string;
  dashboard_port: number;
  run_in_background: boolean;
  start_on_boot: boolean;
  debug_logging: boolean;
  gemini_api_key: string;
  model: string;
  system_prompt_override: string;
  max_output_tokens: number;
  stt_disabled_by_default: boolean;
  input_device: string;
  output_device: string;
  silence_threshold_rms: number;
  push_to_talk_key: string | null;
  auto_start_plugins: boolean;
  plugin_directory: string;
  allowed_mcp_servers: string[];
}

const default_config: Config = {
  backend: "OAuth",
  dashboard_port: 3000,
  run_in_background: false,
  start_on_boot: false,
  debug_logging: false,
  gemini_api_key: "",
  model: "models/gemini-3.1-flash-live-preview",
  system_prompt_override: "",
  max_output_tokens: 8192,
  stt_disabled_by_default: false,
  input_device: "default",
  output_device: "default",
  silence_threshold_rms: 0.01,
  push_to_talk_key: null,
  auto_start_plugins: true,
  plugin_directory: "./plugins",
  allowed_mcp_servers: ["*"]
};

export default function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);

  useEffect(() => {
    // Fetch initial config
    fetch('/api/config')
      .then((res) => {
        if (!res.ok) throw new Error('Failed to fetch config');
        return res.json();
      })
      .then((data: Config) => {
        setConfig(data);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Error fetching config:', err);
        // Fallback for UI demonstration if backend isn't up
        setConfig(default_config);
        setLoading(false);
      });
  }, []);

  const showToast = (message: string, type: 'success' | 'error') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  };

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    try {
      const res = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      });
      if (!res.ok) throw new Error('Failed to save config');
      showToast('Settings saved successfully', 'success');
    } catch (err) {
      console.error(err);
      showToast('Failed to save settings', 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const { name, value, type } = e.target;
    setConfig((prev) => {
      if (!prev) return prev;
      
      let parsedValue: any = value;
      
      if (type === 'checkbox') {
        parsedValue = (e.target as HTMLInputElement).checked;
      } else if (type === 'number') {
        parsedValue = value === '' ? '' : Number(value);
      }

      return {
        ...prev,
        [name]: parsedValue
      };
    });
  };

  const handleArrayChange = (e: React.ChangeEvent<HTMLInputElement>, field: keyof Config) => {
    const val = e.target.value.split(',').map(s => s.trim()).filter(Boolean);
    setConfig(prev => prev ? { ...prev, [field]: val.length > 0 ? val : ["*"] } : prev);
  };

  if (loading || !config) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-gray-950 text-gray-100">
        <div className="flex flex-col items-center space-y-4">
          <Loader2 className="h-10 w-10 animate-spin text-blue-500" />
          <p className="text-sm font-medium text-gray-400">Loading Configuration...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen w-full bg-[#0a0a0c] text-gray-200 p-6 md:p-12 font-sans selection:bg-blue-500/30">
      
      {/* Toast Notification */}
      {toast && (
        <div className={`fixed top-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-lg shadow-xl transition-all duration-300 animate-in fade-in slide-in-from-top-4 ${
          toast.type === 'success' ? 'bg-emerald-500/10 border border-emerald-500/20 text-emerald-400' : 'bg-red-500/10 border border-red-500/20 text-red-400'
        }`}>
          {toast.type === 'success' ? <Check className="h-5 w-5" /> : <Settings2 className="h-5 w-5" />}
          <span className="font-medium">{toast.message}</span>
        </div>
      )}

      <div className="max-w-4xl mx-auto">
        <header className="mb-10 flex flex-col md:flex-row md:items-end justify-between gap-6 border-b border-white/5 pb-6">
          <div>
            <h1 className="text-3xl font-bold tracking-tight text-white flex items-center gap-3">
              <Settings2 className="h-8 w-8 text-blue-500" />
              Settings
            </h1>
            <p className="text-gray-400 mt-2 text-sm">Manage your application configuration and preferences.</p>
          </div>
          
          <button
            onClick={handleSave}
            disabled={saving}
            className="flex items-center gap-2 px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white font-medium rounded-lg shadow-[0_0_20px_-5px_rgba(37,99,235,0.4)] transition-all duration-200 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
          >
            {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
            {saving ? 'Saving...' : 'Save Settings'}
          </button>
        </header>

        <div className="space-y-12">
          
          {/* Core Settings */}
          <section>
            <div className="flex items-center gap-3 mb-6">
              <div className="p-2 bg-indigo-500/10 rounded-md border border-indigo-500/20 text-indigo-400">
                <Server className="h-5 w-5" />
              </div>
              <h2 className="text-xl font-semibold text-white">Core</h2>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 bg-white/[0.02] border border-white/5 p-6 rounded-2xl">
              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Backend Type</label>
                <select 
                  name="backend"
                  value={config.backend}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                >
                  <option value="OAuth">OAuth</option>
                  <option value="API">API</option>
                </select>
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Dashboard Port</label>
                <input 
                  type="number" 
                  name="dashboard_port"
                  value={config.dashboard_port}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                />
              </div>

              <div className="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5">
                <div>
                  <h3 className="text-sm font-medium text-gray-200">Run in Background</h3>
                  <p className="text-xs text-gray-500 mt-1">Keep app alive after closing window</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" name="run_in_background" checked={config.run_in_background} onChange={handleChange} className="sr-only peer" />
                  <div className="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
              </div>

              <div className="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5">
                <div>
                  <h3 className="text-sm font-medium text-gray-200">Start on Boot</h3>
                  <p className="text-xs text-gray-500 mt-1">Launch automatically on startup</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" name="start_on_boot" checked={config.start_on_boot} onChange={handleChange} className="sr-only peer" />
                  <div className="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
              </div>

              <div className="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5 md:col-span-2">
                <div>
                  <h3 className="text-sm font-medium text-gray-200">Debug Logging</h3>
                  <p className="text-xs text-gray-500 mt-1">Enable verbose logs for troubleshooting</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" name="debug_logging" checked={config.debug_logging} onChange={handleChange} className="sr-only peer" />
                  <div className="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
              </div>
            </div>
          </section>

          {/* AI Settings */}
          <section>
            <div className="flex items-center gap-3 mb-6">
              <div className="p-2 bg-emerald-500/10 rounded-md border border-emerald-500/20 text-emerald-400">
                <Cpu className="h-5 w-5" />
              </div>
              <h2 className="text-xl font-semibold text-white">AI Configuration</h2>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 bg-white/[0.02] border border-white/5 p-6 rounded-2xl">
              <div className="space-y-2 md:col-span-2">
                <label className="text-sm font-medium text-gray-300">Gemini API Key</label>
                <input 
                  type="password" 
                  name="gemini_api_key"
                  value={config.gemini_api_key}
                  onChange={handleChange}
                  placeholder="AIza..."
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors font-mono placeholder:text-gray-600"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Model</label>
                <input 
                  type="text" 
                  name="model"
                  value={config.model}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors font-mono"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Max Output Tokens</label>
                <input 
                  type="number" 
                  name="max_output_tokens"
                  value={config.max_output_tokens}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                />
              </div>

              <div className="space-y-2 md:col-span-2">
                <label className="text-sm font-medium text-gray-300">System Prompt Override</label>
                <textarea 
                  name="system_prompt_override"
                  value={config.system_prompt_override}
                  onChange={handleChange}
                  rows={4}
                  placeholder="Leave empty to use default prompt..."
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-3 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors resize-none placeholder:text-gray-600"
                />
              </div>
            </div>
          </section>

          {/* Audio Settings */}
          <section>
            <div className="flex items-center gap-3 mb-6">
              <div className="p-2 bg-amber-500/10 rounded-md border border-amber-500/20 text-amber-400">
                <Mic className="h-5 w-5" />
              </div>
              <h2 className="text-xl font-semibold text-white">Audio & Speech</h2>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 bg-white/[0.02] border border-white/5 p-6 rounded-2xl">
              <div className="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5 md:col-span-2">
                <div>
                  <h3 className="text-sm font-medium text-gray-200">Disable STT by Default</h3>
                  <p className="text-xs text-gray-500 mt-1">Start with Speech-to-Text muted</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" name="stt_disabled_by_default" checked={config.stt_disabled_by_default} onChange={handleChange} className="sr-only peer" />
                  <div className="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Input Device</label>
                <input 
                  type="text" 
                  name="input_device"
                  value={config.input_device}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Output Device</label>
                <input 
                  type="text" 
                  name="output_device"
                  value={config.output_device}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Silence Threshold RMS</label>
                <input 
                  type="number" 
                  step="0.001"
                  name="silence_threshold_rms"
                  value={config.silence_threshold_rms}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Push to Talk Key</label>
                <input 
                  type="text" 
                  name="push_to_talk_key"
                  value={config.push_to_talk_key || ''}
                  onChange={handleChange}
                  placeholder="e.g. F12 or leave empty"
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors"
                />
              </div>
            </div>
          </section>

          {/* Plugins */}
          <section>
            <div className="flex items-center gap-3 mb-6">
              <div className="p-2 bg-pink-500/10 rounded-md border border-pink-500/20 text-pink-400">
                <Puzzle className="h-5 w-5" />
              </div>
              <h2 className="text-xl font-semibold text-white">Plugins & MCP</h2>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 bg-white/[0.02] border border-white/5 p-6 rounded-2xl">
              <div className="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5 md:col-span-2">
                <div>
                  <h3 className="text-sm font-medium text-gray-200">Auto-start Plugins</h3>
                  <p className="text-xs text-gray-500 mt-1">Load and initialize plugins on startup</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" name="auto_start_plugins" checked={config.auto_start_plugins} onChange={handleChange} className="sr-only peer" />
                  <div className="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Plugin Directory</label>
                <input 
                  type="text" 
                  name="plugin_directory"
                  value={config.plugin_directory}
                  onChange={handleChange}
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors font-mono"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-gray-300">Allowed MCP Servers (comma separated)</label>
                <input 
                  type="text" 
                  name="allowed_mcp_servers"
                  value={config.allowed_mcp_servers.join(', ')}
                  onChange={(e) => handleArrayChange(e, 'allowed_mcp_servers')}
                  placeholder="*, file-server, ..."
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-colors font-mono"
                />
              </div>
            </div>
          </section>

        </div>
      </div>
    </div>
  );
}
