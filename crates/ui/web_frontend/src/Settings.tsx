import React, { useState, useEffect } from "react";
import { motion } from "motion/react";
import { apiFetch } from "./api";
import {
  ColorArea,
  ColorPicker,
  ColorSlider,
  ColorSwatch,
  Label,
} from "@heroui/react";
import { Button } from "@heroui/react";
import { Switch } from '@heroui/react';
import { CircleDollar } from "@gravity-ui/icons";
import { Card, Link } from "@heroui/react";

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
};

const MotionButton = motion.create(Button);

function InteractiveButton({
  onClick,
  disabled,
  children,
}: {
  onClick: () => void;
  disabled: boolean;
  children: React.ReactNode;
}) {
  return (
    <MotionButton
      whileHover={{ scale: 1.2 }}
      whileTap={{ scale: 0.8 }}
      style={rectangle}
      onClick={onClick}
      isDisabled={disabled}
      variant="primary"
    >
      {children}
    </MotionButton>
  );
}

function ThemeColorPicker() {
  const [color, setColor] = useState(() => {
    return localStorage.getItem("user-theme-color") || "#0485F7";
  });
  const colorChangeHandler = (colorObject: any) => {
    setColor(colorObject.toString("hsla"));
    const hexColor = colorObject.toString("hex");
    document.documentElement.style.setProperty("--accent", hexColor);
    const hslString = colorObject.toFormat("hsl").toString();
    const formattedHsl = hslString.replace(/hsl\(|\)/g, '').replace(/,/g, '');
    document.documentElement.style.setProperty("--heroui-primary", formattedHsl);
  };
  return (
    <ColorPicker value={color} onChange={colorChangeHandler}>
      <ColorPicker.Trigger>
        <ColorSwatch size="lg" />
        <Label>Pick a theme color</Label>
      </ColorPicker.Trigger>
      <ColorPicker.Popover>
        <ColorArea
          aria-label="Color area"
          className="max-w-full"
          colorSpace="hsb"
          xChannel="saturation"
          yChannel="brightness"
        >
          <ColorArea.Thumb />
        </ColorArea>
        <ColorSlider channel="hue" className="gap-1 px-1" colorSpace="hsb">
          <Label>Hue</Label>
          <ColorSlider.Output className="text-muted" />
          <ColorSlider.Track>
            <ColorSlider.Thumb />
          </ColorSlider.Track>
        </ColorSlider>
      </ColorPicker.Popover>
    </ColorPicker>
  );
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
    const currentHex = document.documentElement.style.getPropertyValue("--accent");
    const currentHsl = document.documentElement.style.getPropertyValue("--heroui-primary");
    if (currentHex) localStorage.setItem("user-theme-color", currentHex);
    if (currentHsl) localStorage.setItem("user-theme-hsl", currentHsl);
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
  };

  useEffect(() => {
    apiFetch("/api/config")
      .then((response) => response.json())
      .then((data) => {
        setConfig(data);
        setLoading(false);
      })
      .catch((err) => {
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
  return (
    <div className="pointer-events-auto flex flex-col items-center justify-center h-full w-fullflex w-full">
    <Card className="w-[400px]">
      <CircleDollar aria-label="Dollar sign icon" className="text-primary size-6" role="img" />
      <Card.Header>
        <Card.Title>Become an Acme Creator!</Card.Title>
        <Card.Description>
          Visit the Acme Creator Hub to sign up today and start earning credits from your fans and
          followers.
        </Card.Description>
      </Card.Header>
      <Card.Footer>
        <Link
          aria-label="Go to Acme Creator Hub (opens in new tab)"
          href="https://heroui.com"
          rel="noopener noreferrer"
          target="_blank"
        >
          Creator Hub
          <Link.Icon aria-hidden="true" />
        </Link>
      </Card.Footer>
    </Card>
      <Switch
        isSelected={config?.run_in_background || false}
        onChange={(value) => changeHandler("run_in_background", value)}
      ><Switch.Content>
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          Background Startup
        </Switch.Content>
      </Switch>
      <Switch
        isSelected={config?.debug_logging || false}
        onChange={(value) => changeHandler("debug_logging", value)}
      ><Switch.Content>
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          Debug Logging
        </Switch.Content>
      </Switch>
      <Switch
        isSelected={config?.stt_disabled_by_default || false}
        onChange={(value) => changeHandler("stt_disabled_by_default", value)}
      ><Switch.Content>
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          STT Disabled on Startup
        </Switch.Content>
      </Switch>
      <Switch
        isSelected={config?.auto_start_plugins || false}
        onChange={(value) => changeHandler("auto_start_plugins", value)}
      ><Switch.Content>
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
          Auto Start Plugins with System
        </Switch.Content>
      </Switch>
      <div className="mt-8 mb-4">
        <ThemeColorPicker />
      </div>
      <InteractiveButton onClick={saveSettings} disabled={saving}>
        {saving ? "Saving..." : "Save Settings"}
      </InteractiveButton>
    </div>
  );
}
