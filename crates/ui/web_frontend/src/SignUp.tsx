import React, { useEffect, useState } from 'react';
import {Separator} from "@heroui/react";
import {Check} from "@gravity-ui/icons";
import { InputOTP } from '@heroui/react';
import {Button, FieldError, Form, Input, Label, TextField} from "@heroui/react";
import {ProgressBar} from "@heroui/react";
import zxcvbn from 'zxcvbn';
import { alertTrigger } from "./alert";
import { useAuthStore } from './authStore';

interface SignUpProps {
  setActiveIndex: (index: number | null) => void;
}

const passwordProgressHandler = (password: string) => {
  if (!password) return "default";
  const result = zxcvbn(password);
  const strength = result.score / 4 * 100;
  if (strength <= 25) return 'danger';
  if (strength <= 50) return 'danger';
  if (strength <= 75) return 'warning';
  return 'success';
}

function SignUpForm({
  onSubmit,
  username,
  setUsername,
  email,
  setEmail,
  password,
  setPassword
}: {
  onSubmit: (e: React.FormEvent<HTMLFormElement>) => void,
  username: string,
  setUsername: (username: string) => void,
  email: string,
  setEmail: (email: string) => void,
  password: string,
  setPassword: (password: string) => void,
  setActiveIndex: (index: number | null) => void,
}) {
  const [validEmailInput, setValidEmailInput] = useState(false);
  const [validPasswordInput, setValidPasswordInput] = useState(false);
  const [isFormValid, setIsFormValid] = useState(false);
  const color = passwordProgressHandler(password);
  useEffect(() => {
    if (validEmailInput && validPasswordInput) {
      setIsFormValid(true);
    } else {
      setIsFormValid(false);
    }
  }, [validEmailInput, validPasswordInput]);
  return (
    <Form className="pointer-events-auto flex w-96 flex-col gap-4 bg-black/40 p-8 rounded-2xl shadow-xl backdrop-blur-md border border-white/10" onSubmit={onSubmit}>
      <TextField
        isRequired
        name="username"
        type="text"
        value={username}
        onChange={setUsername}
      >
        <Label>User Name</Label>
        <Input
          placeholder="John Doe"
          className="placeholder:text-white/40"
        />
        <FieldError />
      </TextField>
      <TextField
        isRequired
        name="email"
        type="email"
        value={email}
        onChange={setEmail}
        validate={(value) => {
          if (!/^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i.test(value)) {
            return "Please enter a valid email address";
          }
          setValidEmailInput(true);
          return null;
        }}
      >
        <Label>Email</Label>
        <Input
          placeholder="john@example.com"
          className="placeholder:text-white/40"
        />
        <FieldError />
      </TextField>
      <TextField
        isRequired
        name="password"
        type="password"
        value={password}
        onChange={setPassword}
        validate={(value) => {
          if (value.length < 8) {
            return "Password must be at least 8 characters long";
          }
          setValidPasswordInput(true);
          return null;
        }}
      >
        <Label>Password</Label>
        <Input
          placeholder="••••••••"
          className="placeholder:text-white/40"
        />
        <FieldError />
      </TextField>
      <div className="flex w-64 flex-col gap-6">
        <ProgressBar aria-label="Password Strength" color={color} value={zxcvbn(password).score / 4 * 100}>
          <Label>Password Strength</Label>
          <ProgressBar.Output />
          <ProgressBar.Track>
            <ProgressBar.Fill />
          </ProgressBar.Track>
        </ProgressBar>
      </div>
      <div className="flex gap-2">
        <Button
          type="submit"
          isDisabled={!isFormValid}
          className={`transition-colors ${
            isFormValid ? 'bg-blue-500 hover:bg-blue-600' : 'bg-gray-400 cursor-not-allowed'
          }`}
        >
          <Check />
          Submit
        </Button>
      </div>
    </Form>
  )
}

function Setup2FAForm({
  qrCode,
  totpCode,
  setTotpCode,
  handleVerify
}: {
  qrCode: string,
  totpCode: string,
  setTotpCode: (code: string) => void,
  handleVerify: (e: React.FormEvent<HTMLFormElement>) => void,
}) {
  return (
    <div className="pointer-events-auto flex w-96 flex-col gap-4 bg-black/40 p-8 rounded-2xl shadow-xl backdrop-blur-md border border-white/10">
      <div style={{ padding: '20px', textAlign: 'center', color: '#333' }}>
        <h2 style={{ marginBottom: '10px' }}>Setup Two-Factor Authentication</h2>
        <p style={{ marginBottom: '20px' }}>Scan the QR code below with your Authenticator app (like Google Authenticator or Authy).</p>
  
          {qrCode && (
          <img 
            src={`data:image/png;base64,${qrCode}`} 
            alt="Scan me" 
            style={{ margin: '0 auto 20px', display: 'block', maxWidth: '200px' }} 
          />
        )}
  
        <form onSubmit={handleVerify}>
          <InputOTP
            maxLength={6}
            value={totpCode}
            onChange={setTotpCode}
          >
          <InputOTP.Group>
            <InputOTP.Slot index={0} />
              <InputOTP.Slot index={1} />
                <InputOTP.Slot index={2} />
                <InputOTP.Slot index={3} />
                <InputOTP.Slot index={4} />
                <InputOTP.Slot index={5} />
              </InputOTP.Group>
            </InputOTP> 
          <Separator className="my-4" />
          <Button variant="primary" type="submit">
            <Check />
            Verify Code
          </Button>
        </form>
      </div>
    </div>
  )
}

const SignUp: React.FC<SignUpProps> = ({ setActiveIndex }) => {
  const addAccount = useAuthStore((state) => state.addAccount);
  const [step, setStep] = useState<'signup' | 'setup_2fa'>('signup');
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [qrCode, setQrCode] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const handleSignup = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const enteredEmail = formData.get('email') as string;
    const enteredPassword = formData.get('password') as string;
    const enteredUserName = formData.get('username') as string;

    if (!enteredUserName) return;
    setUsername(enteredUserName);

    if (!enteredEmail) return;
    setEmail(enteredEmail);

    if (!enteredPassword) return;
    setPassword(enteredPassword);
    try {
      const response = await fetch('/api/2fa/signup', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          username: enteredUserName,
          email: enteredEmail,
          password: enteredPassword,
        }),
      });

      if (response.ok) {
        const data = await response.json();
        useAuthStore.getState().removeAccount(email);
        addAccount({
          username,
          email,
          secret: data.secret,
          sessionToken: data.sessionToken,
        });
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
        body: JSON.stringify({ email, code: totpCode }),
      });

      if (response.ok) {
        setActiveIndex(0);
      } else {
        alertTrigger.danger("Login Failed. Please try again.", "")
      }
    } catch (error) {
      console.error('Error during verification:', error);
    }
  };
  if (step === 'setup_2fa') {
    return (
      <div className="flex items-center justify-center w-full h-full min-h-[80vh]">
        <Setup2FAForm
          qrCode={qrCode}
          totpCode={totpCode}
          setTotpCode={setTotpCode}
          handleVerify={handleVerify}
        />
      </div>
    );
  }
  return (
    <div className="flex items-center justify-center w-full h-full min-h-[80vh]">
      <SignUpForm
        onSubmit={handleSignup}
        email={email}
        setEmail={setEmail}
        username={username}
        setUsername={setUsername}
        password={password}
        setPassword={setPassword}
        setActiveIndex={setActiveIndex}
      />
    </div>
  );
};

export default SignUp;