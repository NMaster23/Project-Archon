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
                alert(err);
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
        })
    }
    
    const handleSave = async () => {
        setSaving(true);
        await fetch('/api/config', {
            method: 'POST',
            body: JSON.stringify(config) 
        });
        setSaving(false);
    };
    
    if (loading || !config) {
        return(
            <Bouncy
            size="45"
            speed="1.75"
            color="black" 
            />
        );
    }
    
    return (
        <div className="min-h-screen bg-black text-gray-200 p-8 font-sans">
            <div className="max-w-6xl mx-auto flex justify-between items-end border-b border-white/10 pb-4 mb-8">
                <h1 className="text-3xl font-bold text-white">Account Settings</h1>
                <button 
                    onClick={handleSave} 
                    disabled={saving}
                    className="bg-blue-600 hover:bg-blue-500 text-white px-4 py-2 rounded-lg font-medium shadow-sm transition disabled:opacity-50"
                >
                    {saving ? "Saving..." : "Save Settings"}
                </button>
            </div>
        <div className="max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-3 gap-8">
            <div className="md:col-span-1">
                <div className="sticky top-8">
                    <ProfileCard
                        name=" Talos Admin"
                        title="System Administrator"
                        handle="talos_admin"
                        status="Online"
                        contactText="View Logs"
                        avatarUrl="https://img.icons8.com/?size=100&id=tZuAOUGm9AuS&format=png&color=000000"
                        showUserInfo
                        enableTilt={true}
                        behindGlowColor="rgba(125, 190, 255, 0.67)"
                        behindGlowEnabled
                    />
                </div>
            </div>
            <div className="md:col-span-2 space-y-8">
                <div className="bg-white/5 border border-white/10 rounded-xl overflow-hidden shadow-lg">
                    <div className="px-6 py-4 border-b border-white/10">
                        <h2 className="text-lg font-semibold text-white">Core Configuration</h2>
                    </div>
                    <div className="p-6">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                            
                            <div className="flex flex-col space-y-2">
                                <label className="text-sm font-medium text-gray-300">Backend Type</label>
                                <input
                                    type="text"
                                    name="backend"
                                    value={config.backend}
                                    onChange={handleChange}
                                    className="bg-black/40 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                />
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

                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    );
}