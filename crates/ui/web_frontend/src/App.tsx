import { useState, useEffect, useRef } from 'react';
import { apiFetch } from './api';

interface ServerStatus {
  uptime: number;
  status: number;
}
import MagicRings from '../components/MagicRings';
import LineSidebar from '../components/LineSidebar';
import LiquidGlass from 'liquid-glass-react'

import Page1 from './Page1';
import Page2 from './Page2';
import Page3 from './Page3';
import Page4 from './Page4';
import Page5 from './Page5';
import Page6 from './Page6';
import Page7 from './Page7';
import Page8 from './Page8';
import Page9 from './Page9';
import Page10 from './Page10';
import Page11 from './Page11';
import Settings from './Settings';

import SignIn from './SignIn';
import SignIn2 from './SignIn2';
import SignUp from './SignUp';
import { useAuthStore } from './authStore';

export default function App() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const isSidebarVisible = activeIndex !== null && activeIndex >= 0 && activeIndex <= 11;
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
      if (!token) { return; }
      try {
        const response = await apiFetch('/api/config', { method: 'GET' });
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
      fetch('/api/status')
        .then(res => {
          if (!res.ok) throw new Error("Network response was not ok");
          return res.json();
        })
        .then(data => {
          setServerStatus(data);
          failCountRef.current = 0; 
        })
        .catch(err => {
          console.error("Failed to fetch from rust backend:", err);
          failCountRef.current += 1;

          if (failCountRef.current > MAX_RETRIES) {
            alert("Unable to connect to the backend after multiple attempts. Please check your server and attempt to reload this page.");
          }
        });
    };

    checkServer();
    const backend_fetch_timer = setInterval(checkServer, 5000);

    return () => clearInterval(backend_fetch_timer);
  }, []);

  useEffect(() => {
    const token = useAuthStore.getState().getActiveToken();
    if (!token) { return; }
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/api/talosbus?token=${token}`;
    const socket = new WebSocket(wsUrl);

    socket.onopen = () => {
      console.log('✅ Connected to TalosBus');
      socket.send('Hello from the frontend!');
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log('📥 TalosBus Data Received:', data);
        
        if (data.BusEvent) {
          const busData = data.BusEvent;
          if (busData.TerminalOutput) {
            setChatHistory(prev => [...prev, `Talos: ${busData.TerminalOutput}`]);
          } else if (busData.AiResponse) {
            setChatHistory(prev => [...prev, `Talos: ${busData.AiResponse}`]);
          } else if (busData.VoiceTranscript) {
            setChatHistory(prev => [...prev, `You: ${busData.VoiceTranscript}`]);
          }
        } else if (data.ClientEvent) {
          setClientHistory(prev => [...prev, `Client Event: ${JSON.stringify(data.ClientEvent)}`]);
        } else if (data.ServerEvent) {
          setServerHistory(prev => [...prev, `Server Event: ${JSON.stringify(data.ServerEvent)}`]);
        } else if (data.ToolsUpdate) {
          setToolHistory(prev => [...prev, `Tools Update: ${JSON.stringify(data.ToolsUpdate)}`]);
        }
      } catch (e) {
        setChatHistory(prev => [...prev, event.data]);
        console.log('📥 TalosBus Data (Text):', event.data);
      }
    };

    socket.onerror = (error) => {
      console.error('❌ TalosBus WebSocket Error:', error);
    };

    socket.onclose = () => {
      console.log('🔌 Disconnected from TalosBus');
    };

    return () => socket.close();
  }, []);

  const renderContent = () => {
    switch (activeIndex) {
      case 0: return <Page1 clientHistory={clientHistory} serverHistory={serverHistory} toolHistory={toolHistory} />;
      case 1: return <Page2 />;
      case 2: return <Page3 chatHistory={chatHistory} />;
      case 3: return <Page4 />;
      case 4: return <Page5 />;
      case 5: return <Page6 />;
      case 6: return <Page7 />;
      case 7: return <Page8 />;
      case 8: return <Page9 />;
      case 9: return <Page10 />;
      case 10: return <Page11 />;
      case 11: return <Settings />;
      case 12: return <SignUp setActiveIndex={setActiveIndex} />;
      case 13: return <SignIn setActiveIndex={setActiveIndex} />;
      case 14: return <SignIn2 setActiveIndex={setActiveIndex} />;
      default:
        return (
          <>
            <div style={{ 
              position: 'absolute', 
              top: '50%', 
              left: '50%', 
              transform: 'translate(-50%, -50%)', 
              pointerEvents: 'none',
              fontSize: 150,
              fontWeight: 100,
              background: 'linear-gradient(135deg, #00f2fe 0%, #4facfe 50%, #A855F7 100%)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
              fontFamily: 'system-ui, -apple-system, sans-serif',
              userSelect: 'none',
              filter: 'drop-shadow(0 0 40px rgba(79, 172, 254, 0.3))',
              letterSpacing: '-0.05em',
              textAlign: 'center',
              lineHeight: 1
            }}>
              Project Archon
            </div>
            <div 
              style={{ 
              position: 'absolute', 
              top: 'calc(50% + 120px)', 
              left: '50%', 
              transform: 'translate(-50%, -50%)',
              pointerEvents: 'auto',
              width: '320px',
              height: '120px'
            }}>
              <LiquidGlass
                mouseContainer={containerRef}
                displacementScale={64}
                blurAmount={0.5}
                saturation={130}
                aberrationIntensity={2}
                elasticity={0.35}
                cornerRadius={100}
                padding="16px 32px"
                style={{ position: 'absolute', top: '50%', left: '50%' }}
                onClick={() => setActiveIndex(12)}
              >
                <span className="text-white font-medium whitespace-nowrap">Sign Up</span>
              </LiquidGlass>
            </div>
            <div 
              style={{ 
              position: 'absolute', 
              top: 'calc(50% + 190px)', 
              left: '50%', 
              transform: 'translate(-50%, -50%)',
              pointerEvents: 'auto',
              width: '250px',
              height: '90px'
            }}>
              <LiquidGlass
                mouseContainer={containerRef}
                displacementScale={64}
                blurAmount={0.5}
                saturation={130}
                aberrationIntensity={2}
                elasticity={0.35}
                cornerRadius={100}
                padding="12px 25px"
                style={{ position: 'absolute', top: '50%', left: '50%' }}
                onClick={() => setActiveIndex(13)}
              >
                <span className="text-white font-medium whitespace-nowrap">Sign In</span>
              </LiquidGlass>
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
      <div className="absolute top-4 right-4 z-[100] text-white/50 font-sans">
        {serverStatus ? `Backend online: Status ${serverStatus.status} (Uptime ${serverStatus.uptime}s)` : "Connecting to backend..."}
      </div>
      <div className="absolute inset-0 z-0">
        <MagicRings
          color="#A855F7"
          colorTwo="#6366F1"
          ringCount={6}
          speed={1}
          attenuation={10}
          lineThickness={2}
          baseRadius={0.35}
          radiusStep={0.1}
          scaleRate={0.1}
          opacity={1}
          blur={0}
          noiseAmount={0.1}
          rotation={0}
          ringGap={1.5}
          fadeIn={0.7}
          fadeOut={0.5}
          followMouse={false}
          mouseInfluence={0.2}
          hoverScale={1.2}
          parallax={0.05}
          clickBurst={false}
        />
      </div>
      <div className="absolute inset-0 z-10 pointer-events-none">
        <div className={`pointer-events-auto w-full h-full ${isSidebarVisible ? 'pl-[20rem]' : ''}`}>
          {renderContent()}
        </div>
      </div>
      {isSidebarVisible && (
        <div className="absolute top-1/2 left-16 -translate-y-1/2 z-20">
          <LineSidebar 
            items={[
              'Dashboard',
              'Server Status',
              'Talos AI Core',
              'TalosBus Network',
              'Access Control',
              'Security Logs',
              'Voice Interface',
              'Telemetry',
              'System Events',
              'Database',
              'Integrations',
              'Settings'
            ]}
            defaultActive={activeIndex} 
            onItemClick={(index: number) => setActiveIndex(index)} 
          />
        </div>
      )}
    </div>
  );
}