interface DashboardProps {
  clientHistory: string[];
  serverHistory: string[];
  toolHistory: string[];
}

export default function DashBoard({ clientHistory, serverHistory, toolHistory }: DashboardProps) {
  const renderHistory = (title: string, history: string[]) => (
    <div className="flex-1 bg-white/5 backdrop-blur-md rounded-2xl p-6 border border-white/10 flex flex-col gap-4 shadow-2xl h-full overflow-hidden">
      <h2 className="text-xl font-bold bg-linear-to-r from-[#00f2fe] to-[#4facfe] bg-clip-text text-transparent">{title}</h2>
      <div className="flex-1 overflow-y-auto font-mono text-sm flex flex-col gap-2">
        {history.length === 0 ? (
          <div className="text-white/40 italic flex h-full items-center justify-center text-center">Awaiting {title.toLowerCase()}...</div>
        ) : (
          history.map((msg, i) => (
            <div key={i} className="text-white/80 border-b border-white/5 pb-2 break-all">
              <span className="text-[#a855f7] mr-2">➜</span>{msg}
            </div>
          ))
        )}
      </div>
    </div>
  );

  return (
    <div className="pointer-events-auto text-white p-8 h-full flex flex-col max-w-360 mx-auto">
      <h1 className="text-4xl font-bold mb-8 bg-linear-to-r from-[#A855F7] to-[#6366F1] bg-clip-text text-transparent">Talos Dashboard</h1>
      
      <div className="flex-1 flex flex-row gap-6 h-full overflow-hidden">
        {renderHistory("Client Events", clientHistory)}
        {renderHistory("Server Events", serverHistory)}
        {renderHistory("Tool Updates", toolHistory)}
      </div>
    </div>
  );
}