import MagicRings from './MagicRings';
import LineSidebar from './LineSidebar';

export default function App() {
  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative', overflow: 'hidden' }}>
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
      <div style={{ 
        position: 'absolute', 
        top: '50%', 
        left: '50%', 
        transform: 'translate(-50%, -50%)', 
        zIndex: 5, 
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

      <div style={{ position: 'absolute', top: '50%', left: '4rem', transform: 'translateY(-50%)', zIndex: 10 }}>
        <LineSidebar />
      </div>
    </div>
  );
}