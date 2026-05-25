import logoPng from "../assets/logo.png";

type LogoVariant = "mark" | "full" | "text";
type LogoSize = "xs" | "sm" | "md" | "lg" | "xl";

interface LogoProps {
  variant?: LogoVariant;
  size?: LogoSize;
  animated?: boolean;
  className?: string;
}

const sizeMap: Record<LogoSize, { mark: number; font: string }> = {
  xs: { mark: 28, font: "text-xs" },
  sm: { mark: 36, font: "text-sm" },
  md: { mark: 44, font: "text-base" },
  lg: { mark: 56, font: "text-xl" },
  xl: { mark: 72, font: "text-3xl" },
};

export default function Logo({
  variant = "mark",
  size = "md",
  animated = true,
  className = "",
}: LogoProps) {
  const { mark, font } = sizeMap[size];

  if (variant === "text") {
    return (
      <span
        className={`font-extrabold tracking-tight bg-gradient-to-br from-neon-300 to-neon-500 bg-clip-text text-transparent ${font} ${
          animated ? "neon-glow-text" : ""
        } ${className}`}
      >
        StoryBox3
      </span>
    );
  }

  return (
    <div className={`flex items-center gap-3 ${className}`}>
      <img
        src={logoPng}
        alt="StoryBox3"
        width={mark}
        height={mark}
        className={`shrink-0 rounded-xl ${animated ? "neon-glow" : ""}`}
        style={{ objectFit: "contain" }}
      />
      {variant === "full" && (
        <span
          className={`font-extrabold tracking-tight bg-gradient-to-br from-neon-300 to-neon-500 bg-clip-text text-transparent ${font} ${
            animated ? "neon-glow-text" : ""
          }`}
        >
          StoryBox3
        </span>
      )}
    </div>
  );
}
