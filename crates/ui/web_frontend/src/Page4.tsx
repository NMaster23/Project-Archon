import { useCallback, useEffect, useRef, useState } from "react";
import { v7 as uuidv7 } from 'uuid';
import { Card } from "@heroui/react";
import { StatusDot } from "./StatusDot";

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

export default function Page4({ websocket }: { websocket: WebSocket | null }) {
  const clients = clientPresenceDetection(websocket);
  return (
    <div style={{ color: 'white', padding: '2rem', pointerEvents: 'auto' }}>
      <h1>Currently Connected Clients</h1>
      <ul>
        {clients.map((client) => (
          <li key={client.id}>
            <Card className="right-6">
          <Card.Header>
            <Card.Title className="flex flex-row items-center gap-2">
              <StatusDot /> {client.name}
            </Card.Title>
          </Card.Header>
          <Card.Footer className="flex flex-col items-start">
            <div><strong>Last Online:</strong> {new Date(client.lastOnline).toLocaleString()}</div>
            <div><strong>Operating System:</strong> {client.clientOs}</div>
            <div><strong>Identification:</strong> {client.id}</div>
          </Card.Footer>
        </Card>
          </li>
        ))}
      </ul>
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
    function unload() {
      presenceBroadcast("presence:exit");
    }
    websocket.addEventListener("message", handleMessage);
    window.addEventListener("beforeunload", unload);
    return () => {
      unload()
      clearInterval(ping)
      clearInterval(offline)
      websocket.removeEventListener("message", handleMessage)
      websocket.removeEventListener("close", unload)
      window.removeEventListener("beforeunload", unload)
    }
  }, [websocket, clientName, sendMessage])
  const client: Peer = {
    id: RefID.current,
    name: clientName,
    clientOs: window.navigator.platform,
    lastOnline: Date.now()
  }
  return [client, ...Array.from(peers.values())]
}