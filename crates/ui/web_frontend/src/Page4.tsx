import { useCallback, useEffect, useRef, useState } from "react";
import { v7 as uuidv7 } from 'uuid';

export interface Peer {
  id: string;
  name: string;
  clientOs: string;
  lastOnline: number;
}

interface Presence {
  type: "presence:init" | "presence:ping" | "presence:exit";
  senderId: string;
  name?: string;
  clientOs?: string;
}

export default function Page4() {
  return (
    <div style={{ color: 'white', padding: '2rem' }}>
      <h1>Currently Connected Clients</h1>
      <p>Content for sidebar position 4</p>
    </div>
  );
}

function clientPresenceDetection(websocket: WebSocket | null, clientName = "Dashboard User") {
  const [peers, setPeers] = useState<Map<string, Peer>>(new Map());
  const RefID = useRef<string>(uuidv7());
  const sendMessage = useCallback((msg: object) => {
    if (websocket && websocket.readyState === WebSocket.OPEN) {
      websocket.send(JSON.stringify(msg));
    }
  }, [websocket]);
  useEffect(() => {
    if (!websocket || websocket.readyState === WebSocket.CLOSED) return;
    const id = RefID.current;
    const platform = window.navigator.platform;
    function presenceBroadcast(msgType: Presence["type"]) {
      sendMessage({
        type: msgType,
        senderId: id,
        name: clientName,
        clientOs: platform,
      });
    }
    presenceBroadcast("presence:init");
    const ping = setInterval(() => {
      presenceBroadcast("presence:ping");
    }, 5000);
    const offline = setInterval(() => {
      const now = Date.now();
      setPeers(prevPeers => {
        const nextPeers = new Map(prevPeers);
        let changed = false;
        for (const [peerId, peer] of nextPeers.entries()) {
          if (now - peer.lastOnline >= 15000) {
            nextPeers.delete(peerId);
            changed = true;
          }
        }
        return changed ? nextPeers : prevPeers;
      });
    }, 3000)
    const handleMessage = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data) as Presence;
        if (!data || !data.type || !data.type.startsWith("presence:")) return;
        if (data.senderId === id) return;
        if (data.type === "presence:init" || data.type === "presence:ping") {
          if (data.type === "presence:init") {
            presenceBroadcast("presence:ping");
          }
          setPeers((prevPeers) => {
            const nextPeers = new Map(prevPeers);
            nextPeers.set(data.senderId, {
              id: data.senderId,
              name: data.name || "Unknown Client",
              clientOs: data.clientOs || "Unknown OS",
              lastOnline: Date.now(),
            });
            return nextPeers;
          });
        } else if (data.type === "presence:exit") {
          setPeers((prevPeers) => {
            const nextPeers = new Map(prevPeers);
            nextPeers.delete(data.senderId);
            return nextPeers;
          });
        }
      } catch { return }
    };
  }, [websocket, clientName, sendMessage])
}