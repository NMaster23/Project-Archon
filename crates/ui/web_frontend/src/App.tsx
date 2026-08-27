import { useState, useEffect, useRef } from "react";
import { apiFetch } from "./api";
import { BackgroundRippleEffect } from "./components/ui/background-ripple-effect";
import MeteorShower from "./components/meteor-shower-animation/meteor-shower";
import { useAuthStore } from "./authStore";
import { Button, CloseButton } from "@heroui/react";
import { Spotlight } from "./components/ui/spotlight";
import { motion } from "motion/react";
import {Bars, Binoculars, HandPointUp, HouseFill, Puzzle} from '@gravity-ui/icons';
import { ListBox } from "@heroui/react";
import { House } from '@gravity-ui/icons';
import { Gear } from '@gravity-ui/icons';
import { AnimatePresence } from "motion/react";
import { alertTrigger, GlobalAlert } from "./alert";
import { StatusDot } from "./StatusDot";

import Page1 from "./Page1";
import Page2 from "./Page2";
import Page3 from "./Page3";
import Page4 from "./Page4";
import Settings from "./Settings";

import SignIn from "./SignIn";
import SignIn2 from "./SignIn2";
import SignUp from "./SignUp";

interface ServerStatus {
  uptime: number;
  status: number;
}

const MotionButton = motion.create(Button);

function SideBar({ activeIndex, setActiveIndex }: { activeIndex: number | null, setActiveIndex:React.Dispatch<React.SetStateAction<number | null>> }) {
  return  (
    <ListBox aria-label="Sidebar" className="w-full" selectionMode="single">
      <ListBox.Item
        id="1"
        textValue="Dashboard"
        onClick={() =>
          setActiveIndex(0)
        }>
          {activeIndex === 0 ? (
            <HouseFill />
          ) : (
            <House />
          )}
          Dashboard
      </ListBox.Item>
      <ListBox.Item id="2" textValue="x" onClick={() => setActiveIndex(1)}>
        <Puzzle />
        Plugins
      </ListBox.Item>
      <ListBox.Item id="3" textValue="x" onClick={() => setActiveIndex(2)}>
        <HandPointUp />
        Interact
      </ListBox.Item>
      <ListBox.Item id="4" textValue="x" onClick={() => setActiveIndex(3)}>
        <Binoculars />
        Sessions
      </ListBox.Item>
      <ListBox.Item id="12" textValue="Settings" onClick={() => setActiveIndex(11)}>
        <Gear color="white"/>
        Settings
      </ListBox.Item>
    </ListBox>
  )
}

function HandleSessionTokenUpload() {
  const addAccount = useAuthStore((state) => state.addAccount);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const handleFileUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
  }
  return (
    <div
      style={{
        position: "absolute",
        top: "calc(50% + 190px)",
        left: "50%",
        transform: "translate(-50%, -50%)",
        pointerEvents: "auto",
        width: "250px",
        height: "90px",
        display: "flex",
        justifyContent: "center",
        alignItems: "center"
      }}
    > 
      <input
        type="file"
        accept=".token,*"
        ref={fileInputRef}
        style={{ display: "none" }}
        onChange={handleFileUpload}
      />
      <MotionButton
        size="md"
        variant="outline"
        className="px-8 font-medium text-zinc-300 border-zinc-600 hover:bg-zinc-800"
        onClick={() =>
        }
      >
        Upload Session Token
      </MotionButton>
    </div>
  )
}

export default function App() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const token = useAuthStore((state) => state.getActiveToken());
  const [isSidebarVisible, setIsSidebarVisible] = useState(false);
  const isLoggedIn =
    activeIndex !== null && activeIndex >= 0 && activeIndex <= 11;
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [chatHistory, setChatHistory] = useState<string[]>([]);
  const [clientHistory, setClientHistory] = useState<string[]>([]);
  const [serverHistory, setServerHistory] = useState<string[]>([]);
  const [toolHistory, setToolHistory] = useState<string[]>([]);
  const [websocket, setWebsocket] = useState<WebSocket | null>(null);
  const failCountRef = useRef(0);
  const MAX_RETRIES = 100;
  useEffect(() => {
    const hsl = localStorage.getItem("user-theme-hsl");
    if (hsl) {
      document.documentElement.style.setProperty("--heroui-primary", hsl);
    }
  }, []);
  useEffect(() => {
    const fetchConfig = async () => {
      if (!token) {
        return;
      }
      try {
        const response = await apiFetch("/api/config", { method: "GET" });
        if (!response.ok) {
          console.log("Error Code:", response.status);
          const errorMessage = await response.text();
          console.error("Error Message:", errorMessage);
          alertTrigger.danger(
            "Unable to connect to server",
            <>
              Server Error ${response.status}: ${errorMessage}
                We're experiencing connection issues. Please try the following:
                <ul className="mt-2 list-inside list-disc space-y-1 text-sm">
                  <li>Check your internet connection</li>
                  <li>Refresh the page</li>
                  <li>Clear your browser cache</li>
                </ul>
            </>
          );
          return;
        }
        const config = await response.json();
        console.log("Fetched config:", config);
      } catch (error) {
        console.error("Error fetching config:", error);
        alertTrigger.danger(
          "Error fetching config. Please check the console for details",
          <>
            We're experiencing connection issues. Please try the following:
            <ul className="mt-2 list-inside list-disc space-y-1 text-sm">
              <li>Check your internet connection</li>
              <li>Refresh the page</li>
              <li>Clear your browser cache</li>
            </ul>
          </>
        );
      }
    };

    fetchConfig();
  }, [isLoggedIn]);
  useEffect(() => {
    const checkServer = () => {
      fetch("/api/status")
        .then((res) => {
          if (!res.ok) throw new Error("Network response was not ok");
          return res.json();
        })
        .then((data) => {
          setServerStatus(data);
          failCountRef.current = 0;
        })
        .catch((err) => {
          console.error("Failed to fetch from rust backend:", err);
          failCountRef.current += 1;

          if (failCountRef.current > MAX_RETRIES) {
            alert(
              "Unable to connect to the backend after multiple attempts. Please check your server and attempt to reload this page.",
            );
          }
        });
    };

    checkServer();
    const backend_fetch_timer = setInterval(checkServer, 5000);

    return () => clearInterval(backend_fetch_timer);
  }, []);

  useEffect(() => {
    const token = useAuthStore.getState().getActiveToken();
    if (!token) {
      return;
    }
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/api/talosbus?token=${token}`;
    const socket = new WebSocket(wsUrl);
    setWebsocket(socket);

    socket.onopen = () => {
      console.log("Connected to TalosBus");
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log("TalosBus Data Received:", data);

        if (data.BusEvent) {
          const busData = data.BusEvent;
          if (busData.TerminalOutput) {
            setChatHistory((prev) => [
              ...prev,
              `Talos: ${busData.TerminalOutput}`,
            ]);
          } else if (busData.AiResponse) {
            setChatHistory((prev) => [...prev, `Talos: ${busData.AiResponse}`]);
          } else if (busData.VoiceTranscript) {
            setChatHistory((prev) => [
              ...prev,
              `You: ${busData.VoiceTranscript}`,
            ]);
          }
        } else if (data.ClientEvent) {
          setClientHistory((prev) => [
            ...prev,
            `Client Event: ${JSON.stringify(data.ClientEvent)}`,
          ]);
        } else if (data.ServerEvent) {
          setServerHistory((prev) => [
            ...prev,
            `Server Event: ${JSON.stringify(data.ServerEvent)}`,
          ]);
        } else if (data.ToolsUpdate) {
          setToolHistory((prev) => [
            ...prev,
            `Tools Update: ${JSON.stringify(data.ToolsUpdate)}`,
          ]);
        }
      } catch (e) {
        setChatHistory((prev) => [...prev, event.data]);
        console.log("TalosBus Data (Text):", event.data);
      }
    };

    socket.onerror = (error) => {
      console.error("TalosBus WebSocket Error:", error);
    };

    socket.onclose = () => {
      console.log("Disconnected from TalosBus");
    };

    return () => {
      setWebsocket(null);
      socket.close();
    };
  }, [token]);

  const renderContent = () => {
    switch (activeIndex) {
      case 0:
        return (
          <Page1
            clientHistory={clientHistory}
            serverHistory={serverHistory}
            toolHistory={toolHistory}
          />
        );
      case 1:
        return <Page2 />;
      case 2:
        return <Page3 chatHistory={chatHistory} websocket={websocket} />;
      case 3:
        return <Page4 websocket={websocket} />;
      case 11:
        return <Settings />;
      case 12:
        return <SignUp setActiveIndex={setActiveIndex} />;
      case 13:
        return <SignIn setActiveIndex={setActiveIndex} />;
      case 14:
        return <SignIn2 setActiveIndex={setActiveIndex} />;
      default:
        return (
          <>
          <div className="relative flex w-full h-full overflow-hidden rounded-md antialiased md:items-center md:justify-center">
             <Spotlight
                className="-top-40 left-10 md:-top-20 md:left-1/4 z-50 mix-blend-overlay"
               fill="white"
             />
              <div
                className="font-mono font-bold text-7xl tracking-tighter"
                style={{
                  position: "absolute",
                  top: "50%",
                  left: "50%",
                  transform: "translate(-50%, -50%)",
                  pointerEvents: "none",
                  fontSize: 150,
                  fontWeight: 100,
                  background:
                    "linear-gradient(to bottom, #93C5FD, #60A5FA)",
                  WebkitBackgroundClip: "text",
                  WebkitTextFillColor: "transparent",
                  fontFamily: '"FiraCode", -apple-system, sans-serif',
                  userSelect: "none",
                  filter: "drop-shadow(0 0 40px rgba(79, 172, 254, 0.3))",
                  letterSpacing: "-0.05em",
                  textAlign: "center",
                  lineHeight: 1,
                }}
              >
                Project Archon
              </div>
              <div
                style={{
                  position: "absolute",
                  top: "calc(50% + 120px)",
                  left: "50%",
                  transform: "translate(-50%, -50%)",
                  pointerEvents: "auto",
                  width: "320px",
                  height: "120px",
                  display: "flex",
                  justifyContent: "center",
                  alignItems: "center"
                }}
              >
              <MotionButton
                size="lg"
                whileHover={{ scale: 1.2 }}
                whileTap={{ scale: 0.9 }}
                onHoverStart={() => console.log('hover started!')}
                variant="primary"
                className="px-8 font-medium"
                onClick={() =>
                  setActiveIndex(12)
                }
              >
                Sign Up
              </MotionButton>
            </div>
            <div
              style={{
                position: "absolute",
                top: "calc(50% + 190px)",
                left: "50%",
                transform: "translate(-50%, -50%)",
                pointerEvents: "auto",
                width: "250px",
                height: "90px",
                display: "flex",
                justifyContent: "center",
                alignItems: "center"
              }}
            >
              <MotionButton
                size="md"
                variant="outline"
                className="px-8 font-medium text-zinc-300 border-zinc-600 hover:bg-zinc-800"
                onClick={() =>
                  setActiveIndex(13)
                }
              >
                Sign In
              </MotionButton>
            </div>
            <HandleSessionTokenUpload />
          </div>
          </>
        );
    }
  };

  return (
    <div
      ref={containerRef}
      className="w-screen h-screen relative overflow-hidden bg-black"
    >
      <GlobalAlert />
      {isLoggedIn && (
        <div className="absolute top-4 left-4 z-50">
          <CloseButton
            className="w-12 h-12"
            onClick={ (() => setIsSidebarVisible(!isSidebarVisible))}
            >
            <Bars color="white" />
          </CloseButton>
        </div>
      )}
      <AnimatePresence>
        {isSidebarVisible && (
          <motion.div
            initial={{ x: "-100%", opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: "-100%", opacity: 0 }}
            transition={{
              type: "spring",
              stiffness: 300,
              damping: 30
            }}
            className="absolute left-0 top-0 bottom-0 w-[320px] z-40 bg-zinc-900/40 backdrop-blur-xl border-r border-white/10 p-4 pt-20 shadow-2xl"
            >
            <SideBar activeIndex={activeIndex} setActiveIndex={setActiveIndex}/>
            </motion.div>
        )}
      </AnimatePresence>
      <div className="absolute bottom-4 left-4 z-100 text-white/50 font-sans">
        {serverStatus ? (
          <span className="flex items-center gap-2">
            <StatusDot status="online" />
            Status {serverStatus.status} (Uptime {serverStatus.uptime}s)
          </span>
        ) : (
          <span className="flex items-center gap-2">
            🔴 Connecting to backend...
          </span>
        )}
      </div>
      <div className="absolute inset-0 z-0">
        {!isLoggedIn ? (
          <BackgroundRippleEffect />
        ) : (
          <MeteorShower className="flex aspect-4/2 items-center justify-center">
          </MeteorShower>
        )}
      </div>
      <div className="absolute inset-0 z-10 pointer-events-auto flex items-center justify-center">
        <motion.div animate={{ paddingLeft: isSidebarVisible ? "320px" : "0px" }} transition={{
          type: "spring",
          stiffness: 300,
          damping: 30
        }}
        className={`w-full h-full ${isSidebarVisible ? "pl-[20%]" : ""}`}
        >
          {renderContent()}
        </motion.div>
      </div>
    </div>
  );
}
