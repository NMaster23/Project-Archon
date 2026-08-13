import { useState, useEffect, useRef } from "react";
import { apiFetch } from "./api";
import { BackgroundRippleEffect } from "./components/ui/background-ripple-effect";
import MeteorShower from "./components/meteor-shower-animation/meteor-shower";
import { cn } from "./lib/utils";
import { useAuthStore } from "./authStore";
import { Button } from "@heroui/react";
import { Spotlight } from "./components/ui/spotlight";
import { motion } from "motion/react";

import Page1 from "./Page1";
import Page2 from "./Page2";
import Page3 from "./Page3";
import Page4 from "./Page4";
import Page5 from "./Page5";
import Page6 from "./Page6";
import Page7 from "./Page7";
import Page8 from "./Page8";
import Page9 from "./Page9";
import Page10 from "./Page10";
import Page11 from "./Page11";
import Settings from "./Settings";

import SignIn from "./SignIn";
import SignIn2 from "./SignIn2";
import SignUp from "./SignUp";

interface ServerStatus {
  uptime: number;
  status: number;
}

const MotionButton = motion.create(Button);

export default function App() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const isSidebarVisible =
    activeIndex !== null && activeIndex >= 0 && activeIndex <= 11;
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [chatHistory, setChatHistory] = useState<string[]>([]);
  const [clientHistory, setClientHistory] = useState<string[]>([]);
  const [serverHistory, setServerHistory] = useState<string[]>([]);
  const [toolHistory, setToolHistory] = useState<string[]>([]);
  const failCountRef = useRef(0);
  const MAX_RETRIES = 100;
  useEffect(() => {
    const fetchConfig = async () => {
      const token = useAuthStore.getState().getActiveToken();
      if (!token) {
        return;
      }
      try {
        const response = await apiFetch("/api/config", { method: "GET" });
        if (!response.ok) {
          console.log("Error Code:", response.status);
          const errorMessage = await response.text();
          console.error("Error Message:", errorMessage);
          alert(`Server Error ${response.status}: ${errorMessage}`);
          return;
        }
        const config = await response.json();
        console.log("Fetched config:", config);
      } catch (error) {
        console.error("Error fetching config:", error);
        alert("Error fetching config. Please check the console for details.");
      }
    };

    fetchConfig();
  }, []);
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

    socket.onopen = () => {
      console.log("✅ Connected to TalosBus");
      socket.send("Hello from the frontend!");
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log("📥 TalosBus Data Received:", data);

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
        console.log("📥 TalosBus Data (Text):", event.data);
      }
    };

    socket.onerror = (error) => {
      console.error("❌ TalosBus WebSocket Error:", error);
    };

    socket.onclose = () => {
      console.log("🔌 Disconnected from TalosBus");
    };

    return () => socket.close();
  }, []);

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
        return <Page3 chatHistory={chatHistory} />;
      case 3:
        return <Page4 />;
      case 4:
        return <Page5 />;
      case 5:
        return <Page6 />;
      case 6:
        return <Page7 />;
      case 7:
        return <Page8 />;
      case 8:
        return <Page9 />;
      case 9:
        return <Page10 />;
      case 10:
        return <Page11 />;
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
          <div className="relative flex h-[40rem] w-full overflow-hidden rounded-md antialiased md:items-center md:justify-center">
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
                variant="outline"
                className="px-8 font-medium text-zinc-300 border-zinc-600 hover:bg-zinc-800"
                onClick={() =>
                  setActiveIndex(13)
                }
              >
                Sign In
              </MotionButton>
            </div>
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
      <div className="absolute top-4 right-4 z-100 text-white/50 font-sans">
        {serverStatus
          ? `🟢 Status ${serverStatus.status} (Uptime ${serverStatus.uptime}s)`
          : "🔴 Connecting to backend..."}
      </div>
      <div className="absolute inset-0 z-0">
        {!isSidebarVisible ? (
          <BackgroundRippleEffect />
        ) : (
          <MeteorShower className="flex aspect-4/2 items-center justify-center">
            <div className="z-10 space-y-4 text-center lg:space-y-6">
              <h4 className="text-2xl font-semibold text-black/80 lg:text-3xl dark:text-white/80">
                Bundui Components
              </h4>
            </div>
          </MeteorShower>
        )}
      </div>
      <div className="absolute inset-0 z-10 pointer-events-none flex items-center justify-center">
        <div className={`w-full h-full ${isSidebarVisible ? "pl-80" : ""}`}>
          {renderContent()}
        </div>
      </div>
    </div>
  );
}
