import React, { useState } from 'react';
import LoginPage, { Logo, Username, Password, Input } from '@react-login-page/page1';
import LoginLogo from 'react-login-page/logo-rect';

const SignUp = () => {
  const [step, setStep] = useState<'signup' | 'setup_2fa'>('signup');
  const [email, setEmail] = useState('');
  const [qrCode, setQrCode] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const handleSignup = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const enteredEmail = formData.get('username') as string;
    
    if (!enteredEmail) return;
    setEmail(enteredEmail);

    try {
      const response = await fetch('/api/2fa/signup', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: enteredEmail }),
      });

      if (response.ok) {
        const data = await response.json();
        setQrCode(data.qr_code_base64);
        setStep('setup_2fa');
      } else {
        console.error('Failed to sign up');
      }
    } catch (error) {
      console.error('Error during signup:', error);
    }
  };
  const handleVerify = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    
    try {
      const response = await fetch('/api/2fa/verify', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, totpCode }),
      });

      if (response.ok) {
        alert('Verification successful! You can now log in.');
      } else {
        console.error('Invalid TOTP code');
      }
    } catch (error) {
      console.error('Error during verification:', error);
    }
  };
  if (step === 'setup_2fa') {
    return (
      <LoginPage style={{ height: 620 }}>
        <Logo>
          <LoginLogo />
        </Logo>
        <div style={{ padding: '20px', textAlign: 'center', color: '#333' }}>
          <h2 style={{ marginBottom: '10px' }}>Setup Two-Factor Auth</h2>
          <p style={{ marginBottom: '20px' }}>Scan the QR code below with your Authenticator app (like Google Authenticator or Authy).</p>
          
          {qrCode && (
            <img 
              src={`data:image/png;base64,${qrCode}`} 
              alt="Scan me" 
              style={{ margin: '0 auto 20px', display: 'block', maxWidth: '200px' }} 
            />
          )}

          <form onSubmit={handleVerify}>
            <input
              type="text"
              placeholder="Enter 6-digit code"
              value={totpCode}
              onChange={(e) => setTotpCode(e.target.value)}
              maxLength={6}
              style={{ 
                padding: '10px', 
                marginBottom: '15px', 
                width: '100%', 
                boxSizing: 'border-box',
                borderRadius: '4px',
                border: '1px solid #ccc'
              }}
            />
            <button 
              type="submit" 
              style={{ 
                padding: '10px 20px', 
                width: '100%',
                backgroundColor: '#007bff', 
                color: 'white', 
                border: 'none', 
                borderRadius: '4px',
                cursor: 'pointer' 
              }}
            >
              Verify Code
            </button>
          </form>
        </div>
      </LoginPage>
    );
  }
  return (
    <form onSubmit={handleSignup} style={{ height: '100%' }}>
      <LoginPage style={{ height: 620 }}>
        <Logo>
          <LoginLogo />
        </Logo>
        <Username name="username" index={3} />
        <Password name="password" index={2} />
        <Input name="phone" index={1} placeholder="Phone number" />
      </LoginPage>
    </form>
  );
};

export default SignUp;