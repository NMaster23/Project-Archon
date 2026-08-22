import { Button } from "@heroui/react";
import { useRef, useState } from "react";
import { alertTrigger } from "./alert";
import { AnimatePresence, motion } from "motion/react";
import { CloseButton } from "@heroui/react";
import {ArrowDownToSquare} from '@gravity-ui/icons';

export default function Page2() {
  const fileInput = useRef<HTMLInputElement>(null);
  const [isInstallVisible, setIsInstallVisible] = useState(false);
  const wasmUploadHandler = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.name.endsWith(".wasm")) {
      alertTrigger.danger("Please upload a valid .wasm file", "");
      return;
    }
    const buffer = file.arrayBuffer();
    const bytes = new Uint8Array((await buffer).slice(0, 4));
    const valid =
      bytes[0] === 0x00 &&
      bytes[1] === 0x61 &&
      bytes[2] === 0x73 &&
      bytes[3] === 0x6d;
    if (!valid) {
      alertTrigger.danger("Corrupted or invalid WASM module", "");
    }
    const form = new FormData();
    form.append("plugin_binary", file);
    try {
      const res = await fetch('/api/plugins/install', {
        method: 'POST',
        body: form,
      });

      const data = await res.json();
      if (!res.ok) throw new Error(data.error || 'Failed to install plugin');
      
      console.log('Plugin installed successfully:', data);
    } catch (err: any) {
      console.error('Upload error:', err.message);
    }
  }
  return (
    <div className="flex-1 flex flex-col items-start gap-4 shadow-2xl h-full overflow-hidden p-6 pointer-events-auto">
      <div className="absolute top-4 left-18 z-50">
        <CloseButton
          className="w-12 h-12 bg-zinc-800 hover:bg-zinc-700 rounded-lg flex items-center justify-center"
          onClick={ (() => setIsInstallVisible(!isInstallVisible))}
          >
          <ArrowDownToSquare color="white"/>
        </CloseButton>
      </div>
      <h1 className="absolute top-0 left-35">Plugins</h1>
      <AnimatePresence>
        {isInstallVisible && (
          <motion.div
            initial={{ y: "-100%", opacity: 0 }}
            animate={{ y: "100%", opacity: 1 }}
            exit={{ y: "-100%", opacity: 0 }}
            transition={{
              type: "spring",
              stiffness: 300,
              damping: 30
            }}
            className="w-min max-w-md backdrop-blur-md flex flex-col items-start p-6 gap-4 bg-blue-300/5 border border-white/10 rounded-xl"
          >
            <input
              type="file"
              accept=".wasm,application/wasm"
              ref={fileInput}
              className="hidden"
              onChange={wasmUploadHandler}
            />
            <Button
              onClick={() =>
                fileInput.current?.click()
              }
            >
              Upload WASM Module
            </Button>
          </motion.div>
      )}
      </AnimatePresence>
    </div>
  );
}
