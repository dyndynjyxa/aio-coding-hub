import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GatewayErrorCodes } from "../../../constants/gatewayErrorCodes";
import { FastModeBadge, FolderBadge, FreeBadge, SessionReuseBadge } from "../LogBadges";
import { getErrorCodeLabel } from "../requestLogErrorLabels";

describe("components/home/LogBadges", () => {
  it("renders small badges with tooltips and labels", () => {
    render(
      <div>
        <SessionReuseBadge showCustomTooltip={false} />
        <SessionReuseBadge showCustomTooltip />
        <FastModeBadge showCustomTooltip={false} />
        <FastModeBadge showCustomTooltip />
        <FreeBadge />
        <FolderBadge folderName="workspace-alpha" folderPath="/tmp/workspace-alpha" allowWrap />
      </div>
    );

    expect(screen.getAllByText("会话复用")[0]).toHaveAttribute("title");
    expect(screen.getAllByText("会话复用")[0]).toHaveClass("ring-blue-400/35");
    expect(screen.getAllByText("fast")[0]).toHaveAttribute("title");
    expect(screen.getByText("免费")).toBeInTheDocument();
    expect(screen.getByText("workspace-alpha")).toBeInTheDocument();
    expect(screen.getByTitle("/tmp/workspace-alpha")).toHaveClass("border-border/45");
    expect(getErrorCodeLabel(GatewayErrorCodes.UPSTREAM_TIMEOUT)).toBe("上游超时");
  });
});
