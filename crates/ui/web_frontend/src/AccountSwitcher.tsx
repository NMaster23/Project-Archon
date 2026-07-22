import React, { useState } from 'react';
import { useAuthStore } from './authStore';
import { ChevronDown, Check, LogOut, Plus, User } from 'lucide-react';

interface AccountSwitcherProps {
  onAddAccount: () => void;
}

export const AccountSwitcher: React.FC<AccountSwitcherProps> = ({ onAddAccount }) => {
  const [isOpen, setIsOpen] = useState(false);
  
  const { accounts, activeEmail, switchAccount, removeAccount } = useAuthStore();
  const activeAccount = accounts.find(a => a.email === activeEmail);

  if (accounts.length === 0) return null;

  return (
    <div className="relative z-50">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-3 px-4 py-2 rounded-xl border border-white/10 bg-black/40 backdrop-blur-md shadow-lg hover:bg-white/5 transition-all text-sm font-medium text-white/90"
      >
        <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-indigo-500 to-purple-500 flex items-center justify-center shadow-inner">
          <span className="text-white font-bold">
            {activeAccount?.email.charAt(0).toUpperCase()}
          </span>
        </div>
        
        <span className="truncate max-w-[150px]">{activeAccount?.email}</span>
        
        <ChevronDown className={`w-4 h-4 text-white/50 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </button>
      {isOpen && (
        <div className="absolute right-0 mt-2 w-72 rounded-2xl border border-white/10 bg-black/60 backdrop-blur-xl shadow-2xl py-2 overflow-hidden">
          
          <div className="px-3 pb-2 pt-1">
            <p className="text-xs font-semibold text-white/50 uppercase tracking-wider">Switch Account</p>
          </div>
          
          <div className="flex flex-col max-h-[300px] overflow-y-auto">
            {accounts.map((account) => {
              const isActive = account.email === activeEmail;
              
              return (
                <div
                  key={account.email}
                  className="group flex items-center justify-between px-3 py-2 mx-2 rounded-lg hover:bg-white/10 cursor-pointer transition-colors"
                  onClick={() => {
                    switchAccount(account.email);
                    setIsOpen(false);
                  }}
                >
                  <div className="flex items-center gap-3 overflow-hidden">
                    <div className={`w-8 h-8 rounded-full flex items-center justify-center text-white ${isActive ? 'bg-gradient-to-tr from-indigo-500 to-purple-500' : 'bg-white/10'}`}>
                      {isActive ? <Check className="w-4 h-4" /> : <User className="w-4 h-4 text-white/50" />}
                    </div>
                    <span className={`truncate text-sm ${isActive ? 'text-white font-medium' : 'text-white/70'}`}>
                      {account.email}
                    </span>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      removeAccount(account.email);
                      if (accounts.length === 1) setIsOpen(false);
                    }}
                    className="p-1.5 rounded-md text-red-400 hover:bg-red-400/20 opacity-0 group-hover:opacity-100 transition-all"
                    title="Sign out"
                  >
                    <LogOut className="w-4 h-4" />
                  </button>
                </div>
              );
            })}
          </div>

          <div className="h-px bg-white/10 my-2" />
          <div className="px-2">
            <button
              onClick={() => {
                setIsOpen(false);
                onAddAccount();
              }}
              className="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-white/10 transition-colors text-sm font-medium text-white/80"
            >
              <div className="w-8 h-8 rounded-full bg-white/5 border border-dashed border-white/30 flex items-center justify-center">
                <Plus className="w-4 h-4 text-white/70" />
              </div>
              Add another account
            </button>
          </div>

        </div>
      )}
    </div>
  );
};