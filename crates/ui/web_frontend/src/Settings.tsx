import React, { useState, useEffect } from 'react';
import { Bouncy } from 'ldrs/react'
import ProfileCard from '../components/ProfileCard'

interface Config {
    ai_permissions: string[];
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
    custom_settings: Record<string, any>;
}

export default function Settings() {
    const [config, setConfig] = useState<Config | null>(null);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    
    useEffect(() => {
        fetch("/api/config")
            .then((response) => response.json())
            .then((data) => {
                setConfig(data);
                setLoading(false);
            }).catch((err) => {
                console.error("Failed to load settings:", err);
                setLoading(false);
            });
    }, []);
    
    const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
        const { name, value, type } = e.target;
        setConfig((prevConfig) => {
            if (!prevConfig) return null;
            let newValue: any = value;
            if (type === 'checkbox') {
                newValue = (e.target as HTMLInputElement).checked;
            } else if (type === 'number') {
                newValue = Number(value);
            }
            return {
                ...prevConfig,
                [name]: newValue
            };
        });
    };

    const handleArrayChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
        const { name, value } = e.target;
        const arrayValue = value.split(',').map(s => s.trim()).filter(Boolean);
        setConfig((prevConfig) => {
            if (!prevConfig) return null;
            return {
                ...prevConfig,
                [name]: arrayValue.length > 0 ? arrayValue : []
            };
        });
    };
    
    const handleSave = async () => {
        setSaving(true);
        try {
            await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config) 
            });
        } catch (err) {
            console.error("Failed to save settings:", err);
        }
        setSaving(false);
    };
    
    if (loading || !config) {
        return(
            <div className="flex h-screen items-center justify-center bg-black">
                <Bouncy size="45" speed="1.75" color="#A855F7" />
            </div>
        );
    }
    
    // Helper to render a nice looking toggle switch
    const renderToggle = (label: string, name: keyof Config, description?: string) => (
        <div className="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5 md:col-span-2">
            <div>
                <h3 className="text-sm font-medium text-gray-200">{label}</h3>
                {description && <p className="text-xs text-gray-500 mt-1">{description}</p>}
            </div>
            <label className="relative inline-flex items-center cursor-pointer">
                <input 
                    type="checkbox" 
                    name={name} 
                    checked={config[name] as boolean} 
                    onChange={handleChange} 
                    className="sr-only peer" 
                />
                <div className="w-11 h-6 bg-white/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
            </label>
        </div>
    );

    return (
        <div className="min-h-screen bg-black text-gray-200 p-8 font-sans overflow-y-auto">
            
            {/* Header */}
            <div className="max-w-6xl mx-auto flex flex-col md:flex-row justify-between items-start md:items-end border-b border-white/10 pb-4 mb-8 gap-4">
                <div>
                    <h1 className="text-3xl font-bold text-white">Account Settings</h1>
                    <p className="text-gray-400 mt-2 text-sm">Configure system parameters, AI, and plugins.</p>
                </div>
                <button 
                    onClick={handleSave} 
                    disabled={saving}
                    className="bg-blue-600 hover:bg-blue-500 text-white px-6 py-2.5 rounded-lg font-medium shadow-[0_0_15px_-3px_rgba(37,99,235,0.4)] transition-all disabled:opacity-50 flex items-center gap-2"
                >
                    {saving ? "Saving..." : "Save Settings"}
                </button>
            </div>

            <div className="max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-3 gap-8">
                
                {/* Profile Card Sidebar */}
                <div className="md:col-span-1 hidden md:block">
                    <div className="sticky top-8">
                        <ProfileCard
                            name="Talos Admin"
                            title="System Administrator"
                            handle="talos_admin"
                            status="Online"
                            contactText="View Logs"
                            avatarUrl="https://github.com/github.png"
                            showUserInfo
                            enableTilt={true}
                            behindGlowColor="rgba(125, 190, 255, 0.67)"
                            behindGlowEnabled
                        />
                    </div>
                </div>

                {/* Settings Panels */}
                <div className="md:col-span-2 space-y-8">
                    
                    {/* Core Configuration */}
                    <div className="bg-white/5 border border-white/10 rounded-xl overflow-hidden shadow-lg">
                        <div className="px-6 py-4 border-b border-white/10 bg-white/[0.02]">
                            <h2 className="text-lg font-semibold text-white">Core Configuration</h2>
                        </div>
                        <div className="p-6">
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Backend Type</label>
                                    <select 
                                        name="backend" 
                                        value={config.backend} 
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    >
                                        <option value="AGY">OAuth</option>
                                        <option value="LOCAL">Local</option>
                                        <option value="API">API</option>
                                    </select>
                                </div>
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Dashboard Port</label>
                                    <input
                                        type="number"
                                        name="dashboard_port"
                                        value={config.dashboard_port}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                                {renderToggle("Run in Background", "run_in_background", "Keep app alive after closing window")}
                                {renderToggle("Start on Boot", "start_on_boot", "Launch automatically on startup")}
                                {renderToggle("Debug Logging", "debug_logging", "Enable verbose logs for troubleshooting")}
                            </div>
                        </div>
                    </div>

                    {/* AI Configuration */}
                    <div className="bg-white/5 border border-white/10 rounded-xl overflow-hidden shadow-lg">
                        <div className="px-6 py-4 border-b border-white/10 bg-white/[0.02]">
                            <h2 className="text-lg font-semibold text-white">AI Settings</h2>
                        </div>
                        <div className="p-6">
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div className="flex flex-col space-y-2 md:col-span-2">
                                    <label className="text-sm font-medium text-gray-300">Gemini API Key</label>
                                    <input
                                        type="password"
                                        name="gemini_api_key"
                                        value={config.gemini_api_key}
                                        onChange={handleChange}
                                        placeholder="AIza..."
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Model</label>
                                    <input
                                        type="text"
                                        name="model"
                                        value={config.model}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Max Output Tokens</label>
                                    <input
                                        type="number"
                                        name="max_output_tokens"
                                        value={config.max_output_tokens}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2 md:col-span-2">
                                    <label className="text-sm font-medium text-gray-300">System Prompt Override</label>
                                    <textarea
                                        name="system_prompt_override"
                                        value={config.system_prompt_override}
                                        onChange={handleChange}
                                        rows={4}
                                        placeholder="Leave empty for default prompt..."
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2 md:col-span-2">
                                    <label className="text-sm font-medium text-gray-300">AI Permissions (comma separated)</label>
                                    <input
                                        type="text"
                                        name="ai_permissions"
                                        value={config.ai_permissions.join(', ')}
                                        onChange={handleArrayChange}
                                        placeholder="read_files, execute_commands, ..."
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Audio Configuration */}
                    <div className="bg-white/5 border border-white/10 rounded-xl overflow-hidden shadow-lg">
                        <div className="px-6 py-4 border-b border-white/10 bg-white/[0.02]">
                            <h2 className="text-lg font-semibold text-white">Audio & Speech</h2>
                        </div>
                        <div className="p-6">
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                {renderToggle("Disable STT by Default", "stt_disabled_by_default", "Start with Speech-to-Text muted")}
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Input Device</label>
                                    <input
                                        type="text"
                                        name="input_device"
                                        value={config.input_device}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Output Device</label>
                                    <input
                                        type="text"
                                        name="output_device"
                                        value={config.output_device}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Silence Threshold RMS</label>
                                    <input
                                        type="number"
                                        step="0.001"
                                        name="silence_threshold_rms"
                                        value={config.silence_threshold_rms}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Push to Talk Key</label>
                                    <input
                                        type="text"
                                        name="push_to_talk_key"
                                        value={config.push_to_talk_key || ''}
                                        onChange={handleChange}
                                        placeholder="e.g. F12 or leave empty"
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Plugins Configuration */}
                    <div className="bg-white/5 border border-white/10 rounded-xl overflow-hidden shadow-lg">
                        <div className="px-6 py-4 border-b border-white/10 bg-white/[0.02]">
                            <h2 className="text-lg font-semibold text-white">Plugins & MCP</h2>
                        </div>
                        <div className="p-6">
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                {renderToggle("Auto-start Plugins", "auto_start_plugins", "Load and initialize plugins on startup")}
                                <div className="flex flex-col space-y-2">
                                    <label className="text-sm font-medium text-gray-300">Plugin Directory</label>
                                    <input
                                        type="text"
                                        name="plugin_directory"
                                        value={config.plugin_directory}
                                        onChange={handleChange}
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                                    />
                                </div>
                                <div className="flex flex-col space-y-2 md:col-span-2">
                                    <label className="text-sm font-medium text-gray-300">Allowed MCP Servers (comma separated)</label>
                                    <input
                                        type="text"
                                        name="allowed_mcp_servers"
                                        value={config.allowed_mcp_servers.join(', ')}
                                        onChange={handleArrayChange}
                                        placeholder="*, file-server, ..."
                                        className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>

                </div>
            </div>
        </div>
    );
}