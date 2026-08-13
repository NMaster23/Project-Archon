import React, { useState } from 'react';
import { useAuthStore } from './authStore';
import { InputOTP } from '@heroui/react';
import {Check} from "@gravity-ui/icons";
import {Button, FieldError, Form, Input, Label, TextField} from "@heroui/react";
import {Separator} from "@heroui/react";

interface SignInProps {
  setActiveIndex: (index: number | null) => void;
}

function SignInForm({
  onSubmit,
  email,
  setEmail,
  totpCode,
  setTotpCode,
  setActiveIndex
}: {
  onSubmit: (e: React.FormEvent<HTMLFormElement>) => void,
  email: string,
  setEmail: (email: string) => void,
  totpCode: string,
  setTotpCode: (code: string) => void,
  setActiveIndex: (index: number | null) => void,
}) {
  return (
    <Form className="pointer-events-auto flex w-96 flex-col gap-4 bg-black/40 p-8 rounded-2xl shadow-xl backdrop-blur-md border border-white/10" onSubmit={onSubmit}>
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
        <Button
          onPress={() => setActiveIndex(14)}
        >
          Sign in With Password
        </Button>
      <div className="flex gap-2">
        <Button type="submit">
          <Check />
          Submit
        </Button>
      </div>
    </Form>
  );
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
        const data = await response.json();
        useAuthStore.getState().addAccount({ 
          username: data.username,
          email: email, 
          secret: '',
          sessionToken: data.token || data.sessionToken
        });

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
    <div className="flex flex-col items-center justify-center h-full w-full">
      <SignInForm
        onSubmit={handleLogin}
        email={email}
        setEmail={setEmail}
        totpCode={totpCode}
        setTotpCode={setTotpCode}
        setActiveIndex={setActiveIndex}
      />
    </div>
  );
};

export default SignIn;