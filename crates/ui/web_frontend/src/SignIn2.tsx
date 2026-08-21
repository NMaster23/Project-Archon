import React, { useState } from 'react';
import {Check} from "@gravity-ui/icons";
import {Separator} from "@heroui/react";
import {Button, FieldError, Form, Input, Label, TextField} from "@heroui/react";
import { useAuthStore } from './authStore';
import { alertTrigger } from "./alert";
import { CloseButton } from '@heroui/react';

interface SignInProps {
  setActiveIndex: (index: number | null) => void;
}

function SignIn({
  onSubmit,
  email,
  setEmail,
  password,
  setPassword,
  setActiveIndex,
}: {
  onSubmit: (e: React.FormEvent<HTMLFormElement>) => void,
  email: string,
  setEmail: (email: string) => void,
  password: string,
  setPassword: (password: string) => void,
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
      <TextField
        isRequired
        name="password"
        type="password"
        value={password}
        onChange={setPassword}
      >
        <Label>Password</Label>
        <Input
          placeholder="••••••••"
          className="placeholder:text-white/40"
        />
        <FieldError />
      </TextField>
      <Separator className="my-4" />
      <Button
        onPress={() => setActiveIndex(13)}
      >
        Sign in With 2FA Code
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

const SignIn2: React.FC<SignInProps> = ({ setActiveIndex }) => {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  const handleLogin = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    try {
      const response = await fetch('/api/password/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password }),
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
        alertTrigger.success("Successfully Logged In", "");
        setActiveIndex(0); 
      } else {
        console.error("Login failed. Check your password.");
        alertTrigger.danger("Login Failed. Please try again.", "")
      }
    } catch (error) {
      console.error('Error during login:', error);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center h-full w-full">
      <SignIn
        onSubmit={handleLogin}
        email={email}
        setEmail={setEmail}
        password={password}
        setPassword={setPassword}
        setActiveIndex={setActiveIndex}
      />
    </div>
  );
};

export default SignIn2;