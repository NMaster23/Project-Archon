import { create } from "zustand";
import { Alert, CloseButton } from '@heroui/react';
import { AnimatePresence, motion } from "framer-motion";

export interface AlertData {
    id?: string | number;
    header: string;
    msg: React.ReactNode;
    type?: "danger" | "warning" | "success" | "primary" | "default"
}

interface StoreAlert {
    current: AlertData | null;
    set: (data: AlertData) => void;
    clear: () => void;
}

const useAlertStore = create<StoreAlert>((set) => ({
  current: null,
  set: (data) => set({ current: { type: "danger", ...data } }),
  clear: () => set({ current: null }),
}));

export const alertTrigger = {
    danger: (header: string, msg: React.ReactNode) => useAlertStore.getState().set({
        header,
        msg,
        type: "danger"
    }),
    warning: (header: string, msg: React.ReactNode) => useAlertStore.getState().set({
        header,
        msg,
        type: "warning"
    }),
    success: (header: string, msg: React.ReactNode) => useAlertStore.getState().set({
        header,
        msg,
        type: "success"
    }),
    primary: (header: string, msg: React.ReactNode) => useAlertStore.getState().set({
        header,
        msg,
        type: "primary"
    }),
    default: (header: string, msg: React.ReactNode) => useAlertStore.getState().set({
        header,
        msg,
        type: "default"
    })
};

export function GlobalAlert() {
    const {
        current,
        clear
    } = useAlertStore();
    return (
        <div className="fixed top-6 right-6 z-50 w-full max-w-md px-4 pointer-events-auto">
            <AnimatePresence>
                {current && (
                <motion.div
                    key={current.id || current.header || "alert-box"}
                    initial={{ x: "200%", opacity: 0 }}
                    animate={{ x: 0, opacity: 1 }}
                    exit={{ x: "200%", opacity: 0 }}
                    transition={{
                    type: "spring",
                    stiffness: 300,
                    damping: 30
                    }}
                >
                <Alert status={current.type === "primary" ? "accent" : (current.type || "danger")}>
                    <Alert.Indicator />
                    <Alert.Content>
                        <Alert.Title>{current.header}</Alert.Title>
                        <Alert.Description>
                            {current.msg}
                        </Alert.Description>
                        <CloseButton onClick={clear} className="absolute right-3 -translate-x-1/2 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-white"/>
                    </Alert.Content>
                </Alert>
                </motion.div>
                )}
            </AnimatePresence>
        </div>
    )
}