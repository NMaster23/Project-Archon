import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface Account {
    username: string;
    email: string;
    secret: string;
    sessionToken: string;
}

interface AuthState {
    accounts: Account[];
    activeEmail: string | null;
    addAccount: (account: Account) => void;
    switchAccount: (email: string) => void;
    removeAccount: (email: string) => void;
    getActiveToken: () => string | null;
}

export const useAuthStore = create<AuthState>()(
    persist(
        (set, get) => ({
            accounts: [],
            activeEmail: null,
            addAccount: (account: Account) => {
                set((state) => {
                    return {
                        ...state,
                        accounts: [...state.accounts, account],
                        activeEmail: account.email
                    };
                });
            },
            switchAccount: (email: string) => {
                set((state) => {
                    return {
                        ...state,
                        activeEmail: email
                    };
                });
            },
            removeAccount: (email: string) => {
                set((state) => {
                    if (state.activeEmail === email) {
                        alert("You have been signed out. Please sign in again.");
                    }
                    return {
                        ...state,
                        accounts: state.accounts.filter((acc) => acc.email !== email),
                        activeEmail: state.activeEmail === email ? null : state.activeEmail,
                    };
                });
            },
            getActiveToken: () => {
                const activeAccount = get().accounts.find((acc) => acc.email === get().activeEmail);
                return activeAccount ? activeAccount.sessionToken : null;
            }
        }),
        {
            name: 'auth-storage',
        }
    )
);