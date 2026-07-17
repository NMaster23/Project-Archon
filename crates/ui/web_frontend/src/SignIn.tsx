import React, { useState } from 'react';
import LoginPage, { Logo, Username, Password, Submit, Title } from '@react-login-page/page1';
import LoginLogo from 'react-login-page/logo-rect';

interface SignInProps {
  setActiveIndex: (index: number | null) => void;
}

const SignIn: React.FC<SignInProps> = ({ setActiveIndex }) => {
  const [email, setEmail] = useState('');
  const [totpCode, setTotpCode] = useState('');

  const handleLogin = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    try {
      const response = await fetch('/api/2fa/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, code: totpCode }),
      });

      if (response.ok) {
        console.log("Login successful!");
        alert("Success! You are logged in.");
        setActiveIndex(0); 
      } else {
        console.error("Login failed. Check your 2FA code.");
        alert("Invalid code or email!");
      }
    } catch (error) {
      console.error('Error during login:', error);
    }
  };

  return (
    <form onSubmit={handleLogin} style={{ height: '100%' }}>
      <LoginPage style={{ height: 580 }}>
      <Logo>
        <LoginLogo />
      </Logo>
      <Title>Talos Login</Title>
      <Username 
        name="email" 
        placeholder="Email address" 
        onChange={(e) => setEmail(e.target.value)} 
      />
      <Password 
        name="totpCode" 
        placeholder="6-digit 2FA Code" 
        onChange={(e) => setTotpCode(e.target.value)} 
      />
      <Submit>Log In</Submit>
    </LoginPage>
    </form>
  );
};

export default SignIn;