interface Page3Props {
  chatHistory: string[];
}

export default function Page3({ chatHistory }: Page3Props) {
  return (
    <div className="text-white p-8 h-full flex flex-col max-w-4xl mx-auto">
      <h1 className="text-4xl font-bold mb-8 bg-gradient-to-r from-[#00f2fe] to-[#4facfe] bg-clip-text text-transparent">Talos AI Core</h1>
      
      <div className="flex-1 bg-white/5 backdrop-blur-md rounded-2xl p-6 border border-white/10 overflow-y-auto font-mono text-sm shadow-2xl flex flex-col gap-2">
        {chatHistory.length === 0 ? (
          <div className="text-white/40 italic flex h-full items-center justify-center">Awaiting TalosBus connection...</div>
        ) : (
          chatHistory.map((msg, i) => (
            <div key={i} className="text-white/80 border-b border-white/5 pb-2">
              <span className="text-[#a855f7] mr-2">➜</span>{msg}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
