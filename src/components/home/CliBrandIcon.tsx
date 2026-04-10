import claudeFavicon from "../../assets/brand/claude-favicon.png";
import codexLogo from "../../assets/brand/codex-logo.svg";
import geminiSparkleAurora from "../../assets/brand/gemini-sparkle-aurora.svg";
import type { CliKey } from "../../services/providers/providers";

type CliBrandIconProps = {
  cliKey: CliKey;
  className?: string;
};

export function CliBrandIcon({ cliKey, className }: CliBrandIconProps) {
  if (cliKey === "claude") {
    return <img src={claudeFavicon} alt="" aria-hidden="true" className={className} />;
  }

  if (cliKey === "codex") {
    return <img src={codexLogo} alt="" aria-hidden="true" className={className} />;
  }

  return <img src={geminiSparkleAurora} alt="" aria-hidden="true" className={className} />;
}
