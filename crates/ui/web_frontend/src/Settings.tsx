import React, { useState, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { apiFetch } from "./api";
import {
  ColorArea,
  ColorPicker,
  ColorSlider,
  ColorSwatch,
  Input,
  Label,
  NumberField,
} from "@heroui/react";
import { Switch } from '@heroui/react';
import { Card } from "@heroui/react";
import { useAuthStore } from "./authStore";
import ConfigEditor from "./ConfigEditor";
import { Button, CloseButton } from "@heroui/react";
import {PencilToSquare} from '@gravity-ui/icons';
import {Person} from '@gravity-ui/icons';
import { AnimatePresence } from "motion/react";

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

interface ServerConfig {
  ai_permissions: string[];
  dashboard_port: number;
  run_in_background: boolean;
  cloudflare_token: string | null;
  auto_start_plugins: boolean;
  start_on_boot: boolean;
  server_port: number;
  plugin_directory: string;
  allowed_mcp_servers: string[];
  custom_settings: Record<string, any>;
}

interface ClientConfig {
  stt_disabled_by_default: boolean;
  input_device: string;
  output_device: string;
  silence_threshold_rms: number;
  push_to_talk_key: string | null;
}

interface UserConfig {
  backend: string;
  model: string;
  system_prompt_override: string;
  max_output_tokens: number;
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
  const [serverConfig, setServerConfig] = useState<ServerConfig | null>(null);
  const [userConfig, setUserConfig] = useState<UserConfig | null>(null);
  const [clientConfig, setClientConfig] = useState<ClientConfig | null>(null);
  const [isEditorVisible, setIsEditorVisible] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const accounts = useAuthStore((state) => state.accounts) || [];
  const activeEmail = useAuthStore((state) => state.activeEmail);
  const account = accounts.find((acc) => acc.email === activeEmail);
  const username = account?.username || "User";
  const profileIconKey = `profile_icon_${username}`
  const [profileIcon, setProfileIcon] = useState(() => {
    return localStorage.getItem(profileIconKey) || null;
  });
  const fileInput = useRef<HTMLInputElement>(null);
  const imageChangeHandler = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = () => {
        if (typeof reader.result === 'string') {
          const base64string = reader.result;
          setProfileIcon(base64string);
          localStorage.setItem(profileIconKey, base64string);
        }
      };
      reader.readAsDataURL(file);
    }
  }
  const changeHandler = (type: 'server' | 'user' | 'client', fieldName: string, newValue: any) => {
    if (type === 'server' && serverConfig) {
      setServerConfig({ ...serverConfig, [fieldName]: newValue });
    } else if (type === 'user' && userConfig) {
      setUserConfig({ ...userConfig, [fieldName]: newValue });
    } else if (type === 'client' && clientConfig) {
      setClientConfig({ ...clientConfig, [fieldName]: newValue });
    }
  };

  const saveSettings = async () => {
    setSaving(true);
    const currentHex = document.documentElement.style.getPropertyValue("--accent");
    const currentHsl = document.documentElement.style.getPropertyValue("--heroui-primary");
    if (currentHex) localStorage.setItem("user-theme-color", currentHex);
    if (currentHsl) localStorage.setItem("user-theme-hsl", currentHsl);
    try {
      await Promise.all([
        apiFetch("/api/config", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(serverConfig),
        }),
        apiFetch("/api/user/prefs", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(userConfig),
        })
      ]);
      setSaving(false);
    } catch (err) {
      console.error("Failed to save settings: ", err);
      setSaving(false);
    }
  };

  useEffect(() => {
    Promise.all([
      apiFetch("/api/config").then(res => res.json()),
      apiFetch("/api/user/prefs").then(res => res.json())
    ])
    .then(([serverData, userData]) => {
      setServerConfig(serverData);
      setUserConfig(userData);
      setLoading(false);
    });
  }, []);
  if (loading) {
    return <div>Loading...</div>;
  }
  if (!userConfig) {
    return <div>Error loading settings.</div>;
  }
  return (
    <div className="pointer-events-auto flex flex-row h-full gap-4">
      <div className="ml-12 w-1/3 h-full pt-24 flex flex-col gap-4">
        <Card className="w-full right-6">
          <Card.Header>
            <Card.Title>{username}</Card.Title>
          </Card.Header>
          <Card.Footer>
            {profileIcon ? (
              <img
              src={profileIcon || undefined}
              alt="Avatar"
              className="w-32 h-32 rounded-full object-cover border-4 border-zinc-800"
              />
            ) : (
              <div className="w-32 h-32 rounded-full bg-zinc-800 border-4 border-zinc-700 flex items-center justify-center">
                <Person className="w-16 h-16" color="white" />
              </div>
            )}
            <input
              type="file"
              accept="image/*"
              ref={fileInput}
              className="hidden"
              onChange={imageChangeHandler}
              />
              <Button
                className="left-10 bottom-1"
                onClick={() =>
                  fileInput.current?.click()
                }
              >
                Upload Avatar
              </Button>
          </Card.Footer>
        </Card>
        <div className="mt-8 mb-4">
          <ThemeColorPicker />
        </div>
        <div className="absolute top-4 left-18 z-50">
          <CloseButton
            className="w-12 h-12"
            onClick={ (() => setIsEditorVisible(!isEditorVisible))}
            >
            <PencilToSquare color="white"/>
          </CloseButton>
        </div>
      </div>
      <div className="backdrop-blur-md flex-1 items-start justify-center h-full flex flex-col p-6 gap-4 bg-blue-300/5 border border-white/10 rounded-xl overflow-y-auto">
        <Input
          className="bg-zinc-800/50 border border-zinc-700 text-white placeholder:text-zinc-400"
          placeholder="Enter your cloudflare token"
          value={serverConfig?.cloudflare_token || ""}
          onChange={(e) => changeHandler("server", "cloudflare_token", e.target.value)}
        />
        <div className="flex flex-col gap-1 w-full">
          <Label>AI Permissions</Label>
          <Input 
            className="bg-zinc-800/50 border border-zinc-700 text-white placeholder:text-zinc-400"
            value={serverConfig?.ai_permissions?.join(", ") || ""}
            onChange={(e) => {
              const val = e.target.value;
              const finalArray = val === "" ? [] : val.split(",").map(s => s.trim()).filter(s => s.length > 0);
              changeHandler("server", "ai_permissions", finalArray);
            }}
          />
        </div>
        <div className="flex flex-col gap-1 w-full">
          <Label>Allowed MCP Servers</Label>
          <Input
            className="bg-zinc-800/50 border border-zinc-700 text-white placeholder:text-zinc-400"
            value={serverConfig?.allowed_mcp_servers?.join(", ") || ""}
            onChange={(e) => {
              const val = e.target.value;
              const finalArray = val === "" ? [] : val.split(",").map(s => s.trim()).filter(s => s.length > 0);
              changeHandler("server", "allowed_mcp_servers", finalArray);
            }}
          />
        </div>
        <Switch
          className="w-full"
          isSelected={serverConfig?.run_in_background || false}
          onChange={(value) => changeHandler("server", "run_in_background", value)}
          size="lg"
        ><Switch.Content 
          className="w-full flex items-center">
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            Background Startup
          </Switch.Content>
        </Switch>
        <Switch
          isSelected={clientConfig?.stt_disabled_by_default || false}
          onChange={(value) => changeHandler("client", "stt_disabled_by_default", value)}
          size="lg"
        ><Switch.Content>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            STT Disabled on Startup
          </Switch.Content>
        </Switch>
        <Switch
          isSelected={serverConfig?.auto_start_plugins || false}
          onChange={(value) => changeHandler("server", "auto_start_plugins", value)}
          size="lg"
        ><Switch.Content>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            Auto Start Plugins with System
          </Switch.Content>
        </Switch>
        <NumberField
          formatOptions={{ useGrouping: false }}
          value={serverConfig?.server_port || 9090}
          onChange={(value) => changeHandler("server", "server_port", value)}
        >
          <Label>Server Port</Label>
          <NumberField.Group>
            <NumberField.DecrementButton />
            <NumberField.Input />
            <NumberField.IncrementButton />
          </NumberField.Group>
        </NumberField>
        <NumberField
          formatOptions={{ useGrouping: false }}
          value={serverConfig?.dashboard_port || 3030}
          onChange={(value) => changeHandler("server", "dashboard_port", value)}
        >
          <Label>Dashboard Port</Label>
          <NumberField.Group>
            <NumberField.DecrementButton />
            <NumberField.Input />
            <NumberField.IncrementButton />
          </NumberField.Group>
        </NumberField>
        <InteractiveButton onClick={saveSettings} disabled={saving}>
          {saving ? "Saving..." : "Save Settings"}
        </InteractiveButton>
      </div>
      <AnimatePresence>
        {isEditorVisible && (
          <motion.div
            initial={{ y: "-100%", opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: "-100%", opacity: 0 }}
            transition={{
              type: "spring",
              stiffness: 300,
              damping: 30
            }}
            className="flex-1 min-h-0 mr-8 mb-4 w-full h-full"
          >
            <ConfigEditor
              config={serverConfig}
              onChange={(newConfig) => {
                // Assuming ConfigEditor returns a complete serverConfig object
                if (newConfig) setServerConfig(newConfig);
              }}
            />
          </motion.div>
      )}
      </AnimatePresence>
    </div>
  );
}
