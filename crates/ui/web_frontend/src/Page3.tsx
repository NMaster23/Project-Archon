import { CloseButton, TextArea } from "@heroui/react";
import { PaperPlane } from '@gravity-ui/icons';
import { useState } from "react";
import { apiFetch } from "./api";

interface Page3Props {
  chatHistory: string[];
}

export default function Page3({ chatHistory }: Page3Props) {
  const [message, setMessage] = useState("");
  const sendMessage = async () => {
    try {
      const response = await apiFetch('/api/message', {
        method: 'POST',
        body: JSON.stringify({ message: message })
      });
      if (response.ok) {
        console.log("Message sent to backend");
        setMessage("");
      } else {
        console.log("Error sending message to backend: ", response.status);
      }
    } catch (error) {
      console.error("Could not send message to backend: ", error);
    }
  };
  return (
    <div className="text-white p-8 h-full flex flex-col w-full mx-auto">
      <h1 className="text-4xl font-bold mb-8 bg-linear-to-r from-[#00f2fe] to-[#4facfe] bg-clip-text text-transparent">Talos AI Core</h1>
      
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
      <div className="flex-1 bg-white/5 backdrop-blur-md rounded-2xl p-6 border border-white/10 overflow-y-auto font-mono text-sm shadow-2xl flex flex-col gap-2">
        <TextArea
          className="h-full w-full"
          placeholder="Send to Talos"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
        />
        <CloseButton
          className="pointer-events-auto text-white hover:bg-default-hover hover:text-foreground active:scale-95"
          onPress={sendMessage}
        >
          <PaperPlane />
        </CloseButton>
      </div>
    </div>
  );
}
