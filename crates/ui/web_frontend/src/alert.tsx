import { create } from "zustand";
import { Alert, Button, CloseButton } from '@heroui/react';

interface AlertData {
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
    if (!current) {
        return null;
    }
    return (
        <div className="fixed top-6 z-50 w-full max-w-md px-4 pointer-events-auto right-6">
            <Alert status={current.type === "primary" ? "accent" : (current.type || "danger")}>
                <Alert.Indicator />
                <Alert.Content>
                    <Alert.Title>{current.header}</Alert.Title>
                    <Alert.Description>
                        {current.msg}
                    </Alert.Description>
                    <CloseButton onClick={clear} className="absolute top-3 right-2"/>
                </Alert.Content>
            </Alert>
        </div>
    )
}