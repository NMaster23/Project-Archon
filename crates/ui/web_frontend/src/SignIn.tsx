import React, { useState } from 'react';
import { useAuthStore } from './authStore';
import { InputOTP } from '@heroui/react';
import {Check} from "@gravity-ui/icons";
import {Button, Description, FieldError, Form, Input, Label, TextField} from "@heroui/react";

interface SignInProps {
  setActiveIndex: (index: number | null) => void;
}

function SignInForm({ onSubmit }: { onSubmit: (e: React.FormEvent<HTMLFormElement>) => void }) {
  return (
    <Form className="flex w-96 flex-col gap-4" onSubmit={onSubmit}>
      <TextField
        isRequired
        name="email"
        type="email"
        validate={(value) => {
          if (!/^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i.test(value)) {
            return "Please enter a valid email address";
          }

          return null;
        }}
      >
        <Label>Email</Label>
        <Input placeholder="john@example.com" />
        <FieldError />
      </TextField>

      <InputOTP maxLength={6}>
          <InputOTP.Group>
            <InputOTP.Slot index={0} />
            <InputOTP.Slot index={1} />
            <InputOTP.Slot index={2} />
            <InputOTP.Slot index={3} />
            <InputOTP.Slot index={4} />
            <InputOTP.Slot index={5} />
          </InputOTP.Group>
        </InputOTP>

      <div className="flex gap-2">
        <Button type="submit">
          <Check />
          Submit
        </Button>
        <Button type="reset" variant="secondary">
          Reset
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
          username: email.split('@')[0],
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
    <SignInForm onSubmit={handleLogin} />
  );
};

export default SignIn;