import React, { useState, useEffect } from 'react';
import { motion } from "motion/react";
import { apiFetch } from './api';

/*
PAGE SettingsPage

DATA:
  Config (Object containing all user settings)
  IsLoading (Boolean, initially true)
  IsSaving (Boolean, initially false)

ON INITIALIZE:
  Send GET request to "/api/config"
  Upon success:
    Set Config data
    Set IsLoading to false

USER ACTIONS:
  ON Field Change (fieldName, newValue):
    Update Config[fieldName] with newValue
    
  ON Save Button Click:
    Set IsSaving to true
    Send POST request to "/api/config" with Config data
    Upon success:
      Set IsSaving to false

UI LAYOUT:
  IF IsLoading is true:
    RENDER LoadingSpinner
    RETURN

  RENDER Main Page Container
    RENDER Header
      Title: "Account Settings"
      Description: "Configure system parameters, AI, and plugins."
      Button: "Save Settings" (Trigger Save Button Click)

    RENDER Grid Layout (3 Columns)
      // Left Sidebar
      RENDER Column 1
        RENDER ProfileCard component

      // Main Settings Area
      RENDER Columns 2 & 3
      
        SECTION "Core Configuration"
          Dropdown: Backend Type (AGY, Local, API)
          Number Input: Dashboard Port
          Toggle: Run in Background
          Toggle: Start on Boot
          Toggle: Debug Logging

        SECTION "AI Settings"
          Password Input: Gemini API Key
          Text Input: Model Name
          Number Input: Max Output Tokens
          Text Area: System Prompt Override
          Text Input: AI Permissions (comma-separated list)

        SECTION "Audio & Speech"
          Toggle: Disable STT by Default
          Text Input: Input Device
          Text Input: Output Device
          Number Input: Silence Threshold RMS
          Text Input: Push to Talk Key

        SECTION "Plugins & MCP"
          Toggle: Auto-start Plugins
          Text Input: Plugin Directory
          Text Input: Allowed MCP Servers (comma-separated list)
*/

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



const rectangle = {
    width: 100,
    height: 50,
    backgroundColor: "var(--hue-3)",
    borderRadius: 5,
}

function ToggleSwitch({ isOn, setIsOn }: { isOn: boolean; setIsOn: (value: boolean) => void }) {
    const toggleSwitch = () => setIsOn(!isOn)

    return (
        <button
            className={`flex w-24 p-1 cursor-pointer rounded-full bg-indigo-500/30 transition-colors ${
                isOn ? "justify-start" : "justify-end"
            }`}
            onClick={toggleSwitch}
        >
            <motion.div
                className="w-10 h-10 rounded-full bg-indigo-500 shadow-sm"
                layout
                transition={{
                    type: "spring",
                    visualDuration: 0.2,
                    bounce: 0.2,
                }}
            />
        </button>
    )
}

function InteractiveButton({ onClick, disabled, children }: { onClick: () => void; disabled: boolean; children: React.ReactNode }) {
    return (
        <motion.button
            whileHover={{ scale: 1.2 }}
            whileTap={{ scale: 0.8 }}
            style={rectangle}
            onClick={onClick}
            disabled={disabled}
        >
            {children}
        </motion.button>
    )
}

export default function SettingsPage() {
    const [config, setConfig] = useState<Config | null>(null);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const changeHandler = (fieldName: string, newValue: any) => {
        if (config) {
            setConfig({ ...config, [fieldName]: newValue });
        }
    };

    const saveSettings = () => {
        setSaving(true);
        apiFetch("/api/config", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(config),
        })
        .then((response) => {
            if (!response.ok) {
                throw new Error("Failed to save settings");
            }
            return response.json();
        })
        .then((data) => {
            setConfig(data);
            setSaving(false);
        })
        .catch((err) => {
            console.error("Failed to save settings:", err);
            setSaving(false);
        });
    }

    useEffect(() => {
        apiFetch("/api/config")
            .then((response) => response.json())
            .then((data) => {
                setConfig(data);
                setLoading(false);
            }).catch((err) => {
                console.error("Failed to load settings:", err);
                setLoading(false);
            });
    }, []);
    if (loading) {
        return <div>Loading...</div>;
    }
    if (!config) {
        return <div>Error loading settings.</div>;
    }
    return(
        <div>
            <ToggleSwitch isOn={config?.run_in_background || false} setIsOn={(value) => changeHandler('run_in_background', value)} />
            <InteractiveButton onClick={saveSettings} disabled={saving}>
                {saving ? "Saving..." : "Save Settings"}
            </InteractiveButton>
        </div>
    );
}