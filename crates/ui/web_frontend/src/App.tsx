import { useState, useEffect } from 'react';

interface ServerStatus {
  uptime: number;
  status: number;
}
import MagicRings from './MagicRings';
import LineSidebar from './LineSidebar';

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
import Page12 from './Page12';

export default function App() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  // @ts-ignore
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);

  useEffect(() => {
    fetch('/api/status')
      .then(res => res.json())
      .then(data => setServerStatus(data))
      .catch(err => console.error("Failed to fetch from rust backend:", err));
    const backend_fetch_timer = setInterval(() => {
      fetch('/api/status')
        .then(res => res.json())
        .then(data => setServerStatus(data))
        .catch(err => console.error("Failed to fetch from rust backend:", err));
    }, 5000);

    return () => clearInterval(backend_fetch_timer);
  }, []);

  const renderContent = () => {
    switch (activeIndex) {
      case 0: return <Page1 />;
      case 1: return <Page2 />;
      case 2: return <Page3 />;
      case 3: return <Page4 />;
      case 4: return <Page5 />;
      case 5: return <Page6 />;
      case 6: return <Page7 />;
      case 7: return <Page8 />;
      case 8: return <Page9 />;
      case 9: return <Page10 />;
      case 10: return <Page11 />;
      case 11: return <Page12 />;
      default:
        return (
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
        );
    }
  };

  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative', overflow: 'hidden' }}>
      
      {/* Tiny overlay for our backend pass-through */}
      <div style={{ position: 'absolute', top: 16, right: 16, zIndex: 100, color: 'white', opacity: 0.5, fontFamily: 'sans-serif' }}>
        {serverStatus ? `Backend online: Status ${serverStatus.status} (Uptime ${serverStatus.uptime}s)` : "Connecting to backend..."}
      </div>

      <div style={{ position: 'absolute', inset: 0, zIndex: 0 }}>
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

      <div style={{ position: 'absolute', inset: 0, zIndex: 5, pointerEvents: 'none' }}>
        <div style={{ pointerEvents: 'auto', width: '100%', height: '100%', paddingLeft: '20rem' }}>
          {renderContent()}
        </div>
      </div>

      <div style={{ position: 'absolute', top: '50%', left: '4rem', transform: 'translateY(-50%)', zIndex: 10 }}>
        <LineSidebar onItemClick={(index: number) => setActiveIndex(index)} />
      </div>
    </div>
  );
}