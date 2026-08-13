"use client";

import React, { useEffect, useRef, useState } from "react";
import { useTheme } from "next-themes";
import { cn } from "../../lib/utils";

interface Meteor {
  x: number;
  y: number;
  size: number;
  speed: number;
  angle: number;
  opacity: number;
  tail: { x: number; y: number }[];
  tailLength: number;
}

interface ThemeColors {
  background: string;
  meteorHead: string;
  meteorTailStart: string;
  meteorTailMiddle: string;
  meteorTailEnd: string;
}

export default function MeteorShower({
  children,
  className
}: {
  children: React.ReactNode;
  className?: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const meteorsRef = useRef<Meteor[]>([]);
  const animationRef = useRef<number>(0);
  const { theme, resolvedTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    const updateDimensions = () => {
      if (typeof window !== "undefined") {
        setDimensions({
          width: window.innerWidth,
          height: window.innerHeight
        });
      }
    };

    updateDimensions();
    window.addEventListener("resize", updateDimensions);

    return () => {
      window.removeEventListener("resize", updateDimensions);
    };
  }, []);

  useEffect(() => {
    if (!canvasRef.current || dimensions.width === 0 || dimensions.height === 0 || !mounted) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    canvas.width = dimensions.width;
    canvas.height = dimensions.height;

    const isDark =
      resolvedTheme === "dark" ||
      document.documentElement.classList.contains("dark") ||
      (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

    const themeColors: ThemeColors = isDark
      ? {
          background: "#000000",
          meteorHead: "rgba(255, 255, 255, 1)",
          meteorTailStart: "rgba(255, 255, 255, 1)",
          meteorTailMiddle: "rgba(255, 240, 200, 0.8)",
          meteorTailEnd: "rgba(255, 200, 100, 0.1)"
        }
      : {
          background: "#ffffff",
          meteorHead: "rgba(70, 90, 120, 1)",
          meteorTailStart: "rgba(70, 90, 120, 1)",
          meteorTailMiddle: "rgba(100, 120, 150, 0.8)",
          meteorTailEnd: "rgba(130, 150, 180, 0.1)"
        };

    const createMeteor = (): Meteor => {
      const positionFactor = Math.random();
      let x, y;

      if (positionFactor < 0.25) {
        x = -20;
        y = dimensions.height * Math.random() * 0.7;
      } else if (positionFactor < 0.5) {
        x = dimensions.width * Math.random();
        y = -20;
      } else if (positionFactor < 0.75) {
        x = dimensions.width + 20;
        y = dimensions.height * Math.random() * 0.7;
      } else {
        x = dimensions.width * 0.3 + Math.random() * dimensions.width * 0.4;
        y = -20;
      }

      const size = 1 + Math.random() * 10;
      const speed = 3;
      const angle = Math.PI / 4;
      const tailLength = 15;

      return {
        x,
        y,
        size,
        speed,
        angle,
        opacity: 0.7 + Math.random() * 0.3,
        tail: [],
        tailLength
      };
    };

    meteorsRef.current = Array.from({ length: 5 }, createMeteor);

    const animate = () => {
      ctx.fillStyle = themeColors.background;
      ctx.fillRect(0, 0, canvas.width, canvas.height);

      meteorsRef.current.forEach((meteor, index) => {
        meteor.x += Math.cos(meteor.angle) * meteor.speed;
        meteor.y += Math.sin(meteor.angle) * meteor.speed;

        meteor.tail.unshift({ x: meteor.x, y: meteor.y });

        if (meteor.tail.length > meteor.tailLength) {
          meteor.tail.pop();
        }

        if (meteor.tail.length > 1) {
          ctx.beginPath();

          const gradient = ctx.createLinearGradient(
            meteor.tail[0].x,
            meteor.tail[0].y,
            meteor.tail[meteor.tail.length - 1].x,
            meteor.tail[meteor.tail.length - 1].y
          );

          gradient.addColorStop(0, themeColors.meteorTailStart.replace("1)", `${meteor.opacity})`));
          gradient.addColorStop(
            0.3,
            themeColors.meteorTailMiddle.replace("0.8)", `${meteor.opacity * 0.8})`)
          );
          gradient.addColorStop(
            1,
            themeColors.meteorTailEnd.replace("0.1)", `${meteor.opacity * 0.1})`)
          );

          ctx.strokeStyle = gradient;
          ctx.lineWidth = meteor.size;

          ctx.moveTo(meteor.tail[0].x, meteor.tail[0].y);
          for (let i = 1; i < meteor.tail.length; i++) {
            ctx.lineTo(meteor.tail[i].x, meteor.tail[i].y);
          }

          ctx.stroke();
        }

        ctx.beginPath();
        ctx.arc(meteor.x, meteor.y, meteor.size / 2, 0, Math.PI * 2);
        ctx.fillStyle = themeColors.meteorHead.replace("1)", `${meteor.opacity})`);
        ctx.fill();

        if (meteor.y > dimensions.height || meteor.x < -50 || meteor.x > dimensions.width + 50) {
          meteorsRef.current[index] = createMeteor();
        }
      });

      if (Math.random() < 0.02 && meteorsRef.current.length < 12) {
        meteorsRef.current.push(createMeteor());
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    animate();

    return () => {
      cancelAnimationFrame(animationRef.current);
    };
  }, [dimensions, theme, resolvedTheme, mounted]);

  return (
    <div className={cn("relative w-full", className)}>
      <canvas
        ref={canvasRef}
        className="absolute h-full w-full"
        style={{ display: dimensions.width > 0 ? "block" : "none" }}
      />
      <div className="z-10">{children}</div>
    </div>
  );
}
